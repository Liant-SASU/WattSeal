#[cfg(target_os = "linux")]
mod linux_tcp_connections;

#[cfg(target_os = "macos")]
mod mac_tcp_connections;

mod windows_tcp_connections;

use std::{collections::HashMap, net::SocketAddr};

use common::{SensorData, TCPConnectionID};

#[cfg(target_os = "linux")]
use crate::sensors::tcp_connections::linux_tcp_connections::LinuxTCPConnectionsCollector;
#[cfg(target_os = "macos")]
use crate::sensors::tcp_connections::mac_tcp_connections::MacosTCPConnectionsCollector;
use crate::sensors::{Sensor, SensorError};
#[cfg(target_os = "windows")]
use crate::sensors::{Sensor, SensorError, tcp_connections::windows_tcp_connections::WindowsTCPConnectionsCollector};

/// A TCP connection key used to identify a TCP Connection on a machine
struct TCPConnectionKey {
    machine_id: String,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
}

impl TCPConnectionKey {
    fn new(machine_id: String, local_addr: SocketAddr, remote_addr: SocketAddr) -> Self {
        TCPConnectionKey {
            machine_id,
            local_addr,
            remote_addr,
        }
    }

    /// Hash the tcp connection key to obtain a unique id
    fn into_tcp_connection_id(&self) -> TCPConnectionID {
        let mut hasher = blake3::Hasher::new();

        hasher.update(self.machine_id.as_bytes());
        hasher.update(self.local_addr.ip().to_string().as_bytes());
        hasher.update(&self.local_addr.port().to_be_bytes());
        hasher.update(self.remote_addr.ip().to_string().as_bytes());
        hasher.update(&self.remote_addr.port().to_be_bytes());

        let hash = hasher.finalize();

        let id = u64::from_le_bytes(hash.as_bytes()[0..8].try_into().unwrap_or([0; 8]));

        TCPConnectionID(id)
    }
}

pub enum TCPConnectionsCollector {
    EmptyCollector,
    #[cfg(target_os = "linux")]
    LinuxCollector(LinuxTCPConnectionsCollector),
    #[cfg(target_os = "macos")]
    MacOSCollector(MacosTCPConnectionsCollector),
    #[cfg(target_os = "windows")]
    WindowsCollector(WindowsTCPConnectionsCollector),
}

/// TCP Connections sensors thay collects informations about connection on
pub struct TCPConnectionsSensor {
    collector: TCPConnectionsCollector,
}

#[cfg(target_os = "linux")]
fn create_collector(_hostname: String) -> TCPConnectionsCollector {
    TCPConnectionsCollector::LinuxCollector(LinuxTCPConnectionsCollector::new(_hostname))
}

#[cfg(target_os = "macos")]
fn create_collector(_hostname: String) -> TCPConnectionsCollector {
    TCPConnectionsCollector::MacOSCollector(MacosTCPConnectionsCollector::new(_hostname))
}

#[cfg(target_os = "windows")]
fn create_collector(_hostname: String) -> TCPConnectionsCollector {
    TCPConnectionsCollector::WindowsCollector(WindowsTCPConnectionsCollector::new(_hostname))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn create_collector(_: String) -> TCPConnectionsCollector {
    TCPConnectionsCollector::EmptyCollector
}

impl TCPConnectionsSensor {
    /// Creates a sensor with collector adapted for the os.
    pub fn new(_hostname: String) -> Self {
        Self {
            collector: create_collector(_hostname),
        }
    }

    pub fn id_to_pid_map(&self) -> Option<HashMap<TCPConnectionID, u32>> {
        match self.collector {
            #[cfg(target_os = "linux")]
            TCPConnectionsCollector::LinuxCollector(ref collector) => Some(collector.id_to_pid_map()),
            #[cfg(target_os = "macos")]
            TCPConnectionsCollector::MacOSCollector(ref collector) => Some(collector.id_to_pid_map()),
            #[cfg(target_os = "windows")]
            TCPConnectionsCollector::WindowsCollector(ref collector) => Some(collector.id_to_pid_map()),
            _ => None,
        }
    }
}

impl Sensor for TCPConnectionsSensor {
    fn read_full_data(&self) -> Result<SensorData, SensorError> {
        match self.collector {
            TCPConnectionsCollector::EmptyCollector => Err(SensorError::NotSupported),
            #[cfg(target_os = "linux")]
            TCPConnectionsCollector::LinuxCollector(ref c) => c
                .collect_tcp_connections()
                .map(|connections| SensorData::TCPConnections(connections)),
            #[cfg(target_os = "macos")]
            TCPConnectionsCollector::MacOSCollector(ref c) => c
                .collect_tcp_connections()
                .map(|connections| SensorData::TCPConnections(connections)),
            #[cfg(target_os = "windows")]
            TCPConnectionsCollector::WindowsCollector(ref c) => c
                .collect_tcp_connections()
                .map(|connections| SensorData::TCPConnections(connections)),
        }
    }
}
