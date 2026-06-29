use std::{cell::RefCell, collections::HashMap, rc::Rc};

use common::{Byte, Percent, ProcessData, ProcessID, ProcessesData, SensorData};
use sysinfo::System;

use crate::sensors::{Sensor, SensorError};

/// Process sensor backed by sysinfo.
pub struct ProcessesSensor {
    system: Rc<RefCell<System>>,
    machine_name: String,
    // TO CHANGE IN RC
    pid_to_id: RefCell<HashMap<u32, ProcessID>>,
}

impl ProcessesSensor {
    /// Creates a sensor sharing the given `System` handle and using the given machine name.
    pub fn new(system: Rc<RefCell<System>>, machine_name: String) -> Self {
        Self {
            system,
            machine_name,
            pid_to_id: RefCell::new(HashMap::new()),
        }
    }

    pub fn pid_to_id(&self) -> HashMap<u32, ProcessID> {
        self.pid_to_id.borrow().clone()
    }
}

/// A process key used to identify a process on a machine
struct ProcessKey {
    machine_id: String,
    process_name: String,
    pid: u32,
}

impl ProcessKey {
    fn new(machine_id: String, process_name: String, pid: u32) -> Self {
        ProcessKey {
            machine_id,
            process_name,
            pid,
        }
    }

    /// Hash the process key to obtain a unique id
    fn into_process_id(&self) -> ProcessID {
        let mut hasher = blake3::Hasher::new();

        hasher.update(self.machine_id.as_bytes());
        hasher.update(self.process_name.as_bytes());
        hasher.update(&self.pid.to_le_bytes());

        let hash = hasher.finalize();

        let id = u64::from_le_bytes(hash.as_bytes()[0..8].try_into().unwrap_or([0; 8]));

        ProcessID(id)
    }
}

impl Sensor for ProcessesSensor {
    fn read_full_data(&self) -> Result<SensorData, SensorError> {
        let mut sys = self
            .system
            .try_borrow_mut()
            .map_err(|e| SensorError::ReadError(format!("Failed to borrow system: {}", e)))?;

        let nb_cores = sys.cpus().len();
        let total_memory = sys.total_memory();

        sys.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            true,
            sysinfo::ProcessRefreshKind::everything()
                .without_environ()
                .without_cwd()
                .without_root()
                .without_tasks()
                .without_user(),
        );

        let processes = sys.processes();

        // Firstly computes processes key to be able to identify processes parent with ProcessID
        self.pid_to_id.borrow_mut().clear();
        for (pid, proc) in processes {
            let name = proc.name().to_str().unwrap_or("__unknown").to_string();
            let key = ProcessKey::new(self.machine_name.to_string(), name.clone(), pid.as_u32());
            let process_id = key.into_process_id();
            self.pid_to_id.borrow_mut().insert(pid.as_u32(), process_id);
        }

        let mut processes_data = Vec::new();

        // Capture every processes data
        for (pid, proc) in processes {
            let name = proc.name().to_str().unwrap_or("__unknown").to_string();
            let parent = proc.parent().and_then(|p| {
                let map = self.pid_to_id.borrow();
                map.get(&p.as_u32()).cloned()
            });
            let key = ProcessKey::new(self.machine_name.to_string(), name.clone(), pid.as_u32());

            let process_id = key.into_process_id();
            let exe_path = proc
                .exe()
                .and_then(|path| path.to_str().and_then(|str| Some(str.to_string())));
            let cpu_usage = (proc.cpu_usage() as f64 / nb_cores as f64) as f32;
            let ram_usage = (proc.memory() as f64 / total_memory as f64 * 100.0) as f32;

            let disk = proc.disk_usage();

            let read_bytes = Some(disk.read_bytes);
            let written_bytes = Some(disk.written_bytes);

            let process_data = ProcessData {
                process_id,
                name,
                parent,
                exe_path,
                cpu_usage: Percent::from(cpu_usage),
                gpu_usage: None,
                ram_usage: Percent::from(ram_usage),
                read_bytes: read_bytes.map(|b| Byte::from(b)),
                written_bytes: written_bytes.map(|b| Byte::from(b)),
            };
            processes_data.push(process_data);
        }
        Ok(SensorData::Processes(ProcessesData(processes_data)))
    }
}
