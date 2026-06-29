use std::{cell::RefCell, collections::HashMap};

use common::{TCPConnectionData, TCPConnectionID, TCPConnectionsData};
use procfs::net::{TcpState, tcp, tcp6};

use crate::sensors::{SensorError, tcp_connections::TCPConnectionKey};

/// Linux TCP connections information collector
pub struct LinuxTCPConnectionsCollector {
    ephemeral_port_range: (u16, u16),                  // Port used briefly, often by client
    machine_name: String,                              // Name of the machine which collects data
    id_to_pid: RefCell<HashMap<TCPConnectionID, u32>>, // Map linking TCP connections id to pid of process which uses the connection
}

/// Defaults ephemeral ports range used in linux
const DEFAULT_MIN_EPHEMERAL_PORT: u16 = 32768;
const DEFAULT_MAX_EPHEMERAL_PORT: u16 = 60999;

/// Returns the ephemeral port range on macos using sysctl
fn get_ephemeral_port_range() -> (u16, u16) {
    let range = std::fs::read_to_string("/proc/sys/net/ipv4/ip_local_port_range")
        .unwrap_or_else(|_| DEFAULT_MIN_EPHEMERAL_PORT.to_string() + " " + &DEFAULT_MAX_EPHEMERAL_PORT.to_string()); // default linux value

    let mut range_parts = range.split_whitespace();
    let min = range_parts
        .next()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_MIN_EPHEMERAL_PORT);
    let max = range_parts
        .next()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_MAX_EPHEMERAL_PORT);

    (min, max)
}

/// Returns a map linking socket inodes with its pid
fn inode_to_pid_map() -> HashMap<u64, u32> {
    let mut hmap = HashMap::new();

    let Ok(ps) = procfs::process::all_processes() else {
        return hmap;
    };

    for p in ps.flatten() {
        let Ok(fds) = p.fd() else { continue };

        for fd in fds.flatten() {
            if let procfs::process::FDTarget::Socket(inode) = fd.target {
                hmap.insert(inode, p.pid as u32);
            }
        }
    }

    hmap
}

impl LinuxTCPConnectionsCollector {
    pub fn new(machine_name: String) -> Self {
        Self {
            ephemeral_port_range: get_ephemeral_port_range(),
            machine_name,
            id_to_pid: RefCell::new(HashMap::new()),
        }
    }

    pub fn id_to_pid_map(&self) -> HashMap<TCPConnectionID, u32> {
        self.id_to_pid.borrow().clone()
    }

    /// Collect IPV4 and IPV6 TCP connections information using procfs
    pub fn collect_tcp_connections(&self) -> Result<TCPConnectionsData, SensorError> {
        let inode_to_pid_map = inode_to_pid_map();
        self.id_to_pid.borrow_mut().clear();

        let mut connections = Vec::new();

        let entries4 = tcp().map_err(|e| SensorError::ReadError(e.to_string()))?;
        let entries6 = tcp6().map_err(|e| SensorError::ReadError(e.to_string()))?;

        for entry in entries4.into_iter().chain(entries6.into_iter()) {
            if entry.state != TcpState::Established {
                continue;
            }
            let local_addr = entry.local_address;
            let remote_addr = entry.remote_address;

            let (ep_min, ep_max) = self.ephemeral_port_range;
            let local_port = local_addr.port();

            let is_maybe_client = if local_port > ep_min && local_port < ep_max {
                Some(true)
            } else {
                Some(false)
            };

            let key = TCPConnectionKey::new(self.machine_name.to_string(), local_addr, remote_addr);

            let id = key.into_tcp_connection_id();

            if let Some(pid) = inode_to_pid_map.get(&entry.inode) {
                self.id_to_pid.borrow_mut().insert(id.clone(), *pid);
            }

            let data = TCPConnectionData {
                connection_id: id,
                local_addr,
                remote_addr,
                maybe_client: is_maybe_client,

                // Need process sensor information
                local_process_id: None,

                // TODO: Need eBPF for linux
                recv_bytes: None,
                sent_bytes: None,
            };
            connections.push(data);
        }
        Ok(TCPConnectionsData(connections))
    }
}
