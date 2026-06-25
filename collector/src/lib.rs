pub mod sensors;

use std::{
    cell::RefCell,
    net::SocketAddr,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

pub use common::clog;
#[cfg(not(debug_assertions))]
use common::logging::start_log_session;
use common::{EnergyWh, Event, SensorData};
use mqtt::{
    MQTTPublisher,
    topics::{hardware_info_topic, sensor_data_to_topic},
};
use sensors::{
    DiskSensor, NetworkSensor, RamSensor, SensorType, create_event_from_sensors, get_hardware_info,
    gpu::{GPUVendor, get_gpu_list},
};
use sysinfo::System;
use tokio::time::{Duration, Instant, interval, sleep_until};

use crate::sensors::processes::ProcessesSensor;

/// Possible units to choose as output
#[derive(Debug, Default, Clone, Copy)]
pub enum ConsumptionUnit {
    WattHour,
    #[default]
    UJoul,
}

/// MQTT information to interact with a MQTT client
pub struct MQTTInfo {
    id: String,
    publisher: MQTTPublisher<rumqttc::Client>,
    unit: Option<ConsumptionUnit>,
}

/// Background sensor-collection application.
pub struct CollectorApp {
    mqtt_info: Option<MQTTInfo>,
    sensors: Vec<SensorType>,
    system: Rc<RefCell<System>>,
    capture_interval: u64,
    last_timestamp: Option<u64>,
    #[cfg(debug_assertions)]
    iteration: u64,
}

impl MQTTInfo {
    pub fn new(id: &str, addr: &SocketAddr, unit: Option<ConsumptionUnit>) -> Self {
        let publisher = MQTTPublisher::new_from_addr(addr);
        MQTTInfo {
            id: id.to_string(),
            publisher,
            unit,
        }
    }
}

impl CollectorApp {
    /// Creates a new collector.
    pub fn new(capture_interval: u64, mqtt_info: Option<MQTTInfo>) -> Result<Self, String> {
        let s = System::new_all();

        Ok(CollectorApp {
            mqtt_info,
            sensors: Vec::new(),
            system: Rc::new(RefCell::new(s)),
            capture_interval,
            last_timestamp: None,
            #[cfg(debug_assertions)]
            iteration: 0,
        })
    }

    /// Detects hardware sensors, and saves hardware info.
    pub fn initialize(&mut self) -> Result<(), String> {
        #[cfg(not(debug_assertions))]
        start_log_session();

        crate::clog!("\n========== INITIALIZING SYSTEM ==========\n");

        // CPU sensor
        crate::clog!("Initializing sensors...");
        match sensors::cpu::get_cpu_power_sensor(self.system.clone(), 0) {
            Ok(sensor) => {
                if let SensorType::CPU(cpu_sensor) = &sensor {
                    let (os_label, mode_label) = cpu_sensor.power_mode_labels();
                    crate::clog!("CPU power mode: {os_label}/{mode_label}");
                }
                crate::clog!("✓ CPU Power Sensor initialized successfully");
                self.sensors.push(sensor);
            }
            Err(e) => crate::clog!("✗ Failed to initialize CPU Power Sensor: {:?}", e),
        }

        // GPU sensors
        let gpu_list = get_gpu_list();
        crate::clog!("\nDetected GPUs: {gpu_list:#?}");
        if gpu_list.is_empty() {
            crate::clog!("⚠ No supported GPU adapters detected.");
        }

        let (mut nvidia_index, mut amd_index, mut intel_index) = (0u32, 0u32, 0u32);
        for gpu_name in &gpu_list {
            let vendor = GPUVendor::from_str(gpu_name);
            let vendor_index = match vendor {
                GPUVendor::Nvidia => {
                    let idx = nvidia_index;
                    nvidia_index += 1;
                    idx
                }
                GPUVendor::Amd => {
                    let idx = amd_index;
                    amd_index += 1;
                    idx
                }
                GPUVendor::Intel => {
                    let idx = intel_index;
                    intel_index += 1;
                    idx
                }
                GPUVendor::Other => 0,
            };

            match sensors::gpu::get_gpu_energy_sensor(gpu_name, vendor_index) {
                Ok(sensor) => {
                    crate::clog!(
                        "✓ GPU sensor initialized: '{}' (vendor={:?}, vendor_index={})",
                        gpu_name,
                        vendor,
                        vendor_index
                    );
                    self.sensors.push(sensor);
                }
                Err(e) => {
                    crate::clog!(
                        "✗ Failed to initialize GPU sensor for '{}' (vendor={:?}, vendor_index={}): {:?}",
                        gpu_name,
                        vendor,
                        vendor_index,
                        e
                    );
                }
            }
        }

        // RAM, Disk, Network sensors
        self.sensors.push(SensorType::RAM(RamSensor::new(self.system.clone())));
        self.sensors.push(SensorType::Disk(DiskSensor::new()));
        self.sensors.push(SensorType::Network(NetworkSensor::new()));

        //  Processes sensors
        let hostname = hostname::get().unwrap_or_default().to_string_lossy().to_string();
        self.sensors.push(SensorType::Processes(ProcessesSensor::new(
            self.system.clone(),
            hostname,
        )));

        // Hardware info
        crate::clog!("\n========== GATHERING HARDWARE INFORMATION ==========\n");
        let info = get_hardware_info(&self.sensors);

        // Publish hardware info on MQTT Broker
        if let Some(mqtt_info) = &self.mqtt_info {
            if let Ok(timestamp) = SystemTime::now().duration_since(UNIX_EPOCH).map(|t| t.as_secs()) {
                let topic = hardware_info_topic(&mqtt_info.id);

                match mqtt_info
                    .publisher
                    .publish(&topic, &info.hardware_info.serialized(), timestamp)
                {
                    Ok(_) => crate::clog!("✓ Hardware info published on broker"),
                    Err(e) => crate::clog!("✗ Failed to publish hardware info: {e}"),
                }
            } else {
                crate::clog!("✗ Failed to get system time and timestamped hardware info publishing");
            }
        }
        crate::clog!("Initialization complete");
        Ok(())
    }

    /// Publish event sensors data with given timestamp on the collector mqtt client
    fn publish_event_data(&self, event: &Event, timestamp: u64) {
        if let Some(mqtt_info) = &self.mqtt_info {
            for sensor_data in event.data() {
                let topic = sensor_data_to_topic(&mqtt_info.id, &sensor_data);

                let _result = mqtt_info.unit.map_or_else(
                    || mqtt_info.publisher.publish(&topic, sensor_data, timestamp),
                    |u| match u {
                        ConsumptionUnit::UJoul => mqtt_info.publisher.publish(&topic, &sensor_data, timestamp),
                        ConsumptionUnit::WattHour => {
                            let sensor_data_wh: SensorData<EnergyWh> = sensor_data.to_wh();
                            mqtt_info.publisher.publish(&topic, &sensor_data_wh, timestamp)
                        }
                    },
                );
                #[cfg(debug_assertions)]
                match _result {
                    Ok(_) => println!("✓ Sensor data published on topic {}", topic),
                    Err(e) => eprintln!("✗ Failed to publish sensor data on topic {}: {:?}", topic, e),
                }
            }
        }
    }

    /// Runs the collection loop, sampling sensors every capture interval second.
    pub async fn run(&mut self) {
        #[cfg(debug_assertions)]
        println!("\n========== POWER CONSUMPTION MONITORING ==========\nPress Ctrl+C to stop.\n");

        let now_result = SystemTime::now().duration_since(UNIX_EPOCH);
        let Ok(now) = now_result else {
            crate::clog!("✗ Failed to get system time and stop collecting run");
            return;
        };

        // To synchronize machines timestamp on the modulo
        let remaining_secs = self.capture_interval - (now.as_secs() % self.capture_interval);
        let remaining = Duration::from_secs(remaining_secs) - Duration::from_millis(now.subsec_millis() as u64);

        sleep_until(Instant::now() + remaining).await;

        let mut interval = interval(Duration::from_secs(self.capture_interval));

        loop {
            interval.tick().await;
            let Ok(timestamp) = SystemTime::now().duration_since(UNIX_EPOCH).map(|t| t.as_secs()) else {
                crate::clog!("✗ Failed to get system time for timestamp and stop collector run");
                return;
            };

            if let Some(last_timestamp) = self.last_timestamp {
                let since_last_update = Duration::from_secs(timestamp - last_timestamp);

                #[cfg(debug_assertions)]
                println!("\n--- Iteration {} (timestamp : {}) ---", self.iteration, timestamp);

                let event = create_event_from_sensors(&self.sensors, since_last_update);

                self.publish_event_data(&event, timestamp);

                #[cfg(debug_assertions)]
                for sensor_data in event.data() {
                    println!("{sensor_data}");
                }

                #[cfg(debug_assertions)]
                {
                    self.iteration += 1;
                    if since_last_update > Duration::from_secs(1) {
                        eprintln!("WARNING: Iteration {} took longer than 1 second.", self.iteration);
                    }
                }
            }

            self.last_timestamp = Some(timestamp);
        }
    }
}
