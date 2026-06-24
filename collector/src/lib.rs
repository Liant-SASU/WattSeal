pub mod sensors;

use std::{
    cell::RefCell,
    net::SocketAddr,
    rc::Rc,
    thread,
    time::{Duration, Instant, SystemTime},
};

pub use common::clog;
#[cfg(not(debug_assertions))]
use common::logging::start_log_session;
use common::{EnergyWh, SensorData};
use mqtt::{
    MQTTPublisher,
    topics::{hardware_info_topic, sensor_data_to_topic},
};
use sensors::{
    DiskSensor, NetworkSensor, RamSensor, SensorType, create_event_from_sensors, get_hardware_info,
    gpu::{GPUVendor, get_gpu_list},
};
use sysinfo::System;

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
    last_update: Instant,
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
    pub fn new(mqtt_info: Option<MQTTInfo>) -> Result<Self, String> {
        let s = System::new_all();
        Ok(CollectorApp {
            mqtt_info,
            sensors: Vec::new(),
            system: Rc::new(RefCell::new(s)),
            last_update: Instant::now(),
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
            let topic = hardware_info_topic(&mqtt_info.id);
            match mqtt_info.publisher.publish(&topic, &info.hardware_info.serialized()) {
                Ok(_) => crate::clog!("✓ Hardware info published on broker"),
                Err(e) => crate::clog!("✗ Failed to publish hardware info: {e}"),
            }
        }
        crate::clog!("Initialization complete");
        Ok(())
    }

    /// Runs the collection loop, sampling sensors every second.
    pub fn run(&mut self) {
        #[cfg(debug_assertions)]
        println!("\n========== POWER CONSUMPTION MONITORING ==========\nPress Ctrl+C to stop.\n");

        loop {
            let start_time = Instant::now();
            let since_last_update = self.last_update.elapsed();
            self.last_update = start_time;

            #[cfg(debug_assertions)]
            println!("\n--- Iteration {} ---", self.iteration);

            let event = create_event_from_sensors(&self.sensors, since_last_update);

            if let Some(mqtt_info) = &self.mqtt_info {
                for sensor_data in event.data() {
                    let topic = sensor_data_to_topic(&mqtt_info.id, &sensor_data);

                    let _result = mqtt_info.unit.map_or_else(
                        || mqtt_info.publisher.publish(&topic, sensor_data),
                        |u| match u {
                            ConsumptionUnit::UJoul => mqtt_info.publisher.publish(&topic, &sensor_data),
                            ConsumptionUnit::WattHour => {
                                let sensor_data_wh: SensorData<EnergyWh> = sensor_data.to_wh();
                                mqtt_info.publisher.publish(&topic, &sensor_data_wh)
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

            #[cfg(debug_assertions)]
            for sensor_data in event.data() {
                println!("{sensor_data}");
            }

            #[cfg(debug_assertions)]
            {
                self.iteration += 1;
                if start_time.elapsed() > Duration::from_millis(1000) {
                    eprintln!("WARNING: Iteration {} took longer than 1 second.", self.iteration);
                }
            }

            // Adjust sleep duration to maintain 1 second interval
            let now_sub_ms = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_millis()
                % 1000;
            if now_sub_ms < 1000 {
                thread::sleep(Duration::from_millis(1000 - now_sub_ms as u64));
            }
        }
    }
}
