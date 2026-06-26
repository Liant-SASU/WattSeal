use common::SensorData;

use crate::sensors::{Sensor, SensorError};

pub enum TCPConnectionsCollector {
    EmptyCollector,
}

/// TCP Connections sensors thay collects informations about connection on
pub struct TCPConnectionsSensor {
    collector: TCPConnectionsCollector,
}

impl TCPConnectionsSensor {
    /// Creates a sensor with collector adapted for the os.
    pub fn new() -> Self {
        Self {
            collector: TCPConnectionsCollector::EmptyCollector,
        }
    }
}

impl Sensor for TCPConnectionsSensor {
    fn read_full_data(&self) -> Result<SensorData, SensorError> {
        match self.collector {
            TCPConnectionsCollector::EmptyCollector => Err(SensorError::NotSupported),
        }
    }
}
