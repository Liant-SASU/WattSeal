pub mod process;
mod tables;

use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub use common::{
    Byte, CPUDataDB, DATABASE_PATH, DataDB, Database, DatabaseEntry, DiskDataDB, EnergyUj, Event, EventDB, GPUDataDB,
    NetworkDataDB, ProcessDataDB, RamDataDB, SensorData, SensorDataDB, TotalDataDB,
};
use sysinfo::System;

use crate::database::process::get_processes;

pub fn consumption_event_to_eventdb(
    sensors_event: &Event,
    system: Rc<RefCell<System>>,
    process_gpu_usage: HashMap<u32, f64>,
) -> EventDB {
    let mut data: Vec<DataDB> = sensors_event
        .data()
        .into_iter()
        .map(|sensor_data| DataDB::from(sensor_data))
        .collect();

    let (mut cpu_energy, mut cpu_usage, mut nb_cpus) = (EnergyUj::from_u64(0), 0.0, 0);
    let (mut gpu_energy, mut gpu_usage, mut nb_gpus) = (EnergyUj::from_u64(0), 0.0, 0);

    let mut total_energy = EnergyUj::from_u64(0);

    for datadb in &data {
        if let Some(energy) = datadb.total_energy() {
            total_energy += energy;

            if let DataDB::Sensor(SensorData::CPU(cpu)) = &datadb {
                cpu_energy += energy;
                cpu_usage += cpu.usage_percent.unwrap_or(0.0);
                nb_cpus += 1;
            }

            if let DataDB::Sensor(SensorData::GPU(gpu)) = &datadb {
                gpu_energy += energy;
                gpu_usage += gpu.usage_percent.unwrap_or(0.0);
                nb_gpus += 1;
            }
        }
    }

    data.push(DataDB::Total(TotalDataDB { total_energy }));

    cpu_usage /= nb_cpus.max(1) as f64;
    gpu_usage /= nb_gpus.max(1) as f64;
    let top10_process_data: Vec<ProcessDataDB> = get_processes(
        system.clone(),
        cpu_energy,
        cpu_usage,
        gpu_energy,
        gpu_usage,
        total_energy,
        10,
        process_gpu_usage,
    );
    data.push(DataDB::Process(top10_process_data));

    EventDB::new(sensors_event.time(), data)
}
