pub mod logging;
pub mod types;
pub mod utils;

/// In debug → `println!`. In release → append timestamped line to log file.
#[macro_export]
macro_rules! clog {
    ($($arg:tt)*) => {{
        #[cfg(debug_assertions)]
        println!($($arg)*);
        #[cfg(not(debug_assertions))]
        $crate::logging::log_to_file(&format!($($arg)*));
    }};
}

pub use types::{
    AllTimeData, Byte, CPUData, DiskData, EnergyUj, EnergyWh, Event, GPUData, GeneralData, HardwareInfo,
    MICROJOULES_PER_JOULE, NetworkData, Percent, PowerW, ProcessData, ProcessID, ProcessesData, RamData,
    SECONDS_PER_HOUR, SensorData, SensorKind,
};
pub use utils::set_current_dir_to_exe_dir;
