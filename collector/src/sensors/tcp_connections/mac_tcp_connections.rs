use std::{
    cell::RefCell,
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};

use common::{Byte, TCPConnectionData, TCPConnectionID, TCPConnectionsData};
use libproc::{
    file_info::{ListFDs, ProcFDType},
    net_info::{SocketFDInfo, SocketInfoKind, TcpSIState, TcpSockInfo},
    proc_pid::pidinfo,
    processes::{ProcFilter, pids_by_type},
};
use sysctl::Sysctl;

use crate::sensors::{SensorError, tcp_connections::TCPConnectionKey};

pub struct MacosTCPConnectionsCollector {
    ephemeral_port_range: (u16, u16),
    machine_name: String,
    id_to_pid: RefCell<HashMap<TCPConnectionID, u32>>,
}

const DEFAULT_MIN_EPHEMERAL_PORT: u16 = 49152;
const DEFAULT_MAX_EPHEMERAL_PORT: u16 = 65535;

fn get_ephemeral_port_range() -> (u16, u16) {
    let min = (sysctl::Ctl::new("net.inet.ip.portrange.first").and_then(|ctl| ctl.value_as::<u32>()) as u16)
        .unwrap_or(DEFAULT_MAX_EPHEMERAL_PORT);

    let max = (sysctl::Ctl::new("net.inet.ip.portrange.last").and_then(|ctl| ctl.value_as::<u32>()) as u16)
        .unwrap_or(DEFAULT_MAX_EPHEMERAL_PORT);

    (min, max)
}

fn extract_addrs(tcp_info: &TcpSockInfo) -> (SocketAddr, SocketAddr) {
    unsafe {
        if tcp_info.tcpsi_ini.insi_vflag == 4 {
            let local_ip = IpAddr::V4(Ipv4Addr::from(
                tcp_info.tcpsi_ini.insi_laddr.ina_46.i46a_addr4.s_addr.to_be(),
            ));
            let remote_ip = IpAddr::V4(Ipv4Addr::from(
                tcp_info.tcpsi_ini.insi_faddr.ina_46.i46a_addr4.s_addr.to_be(),
            ));
            let local_port = u16::from_be(tcp_info.tcpsi_ini.insi_lport as u16);
            let remote_port = u16::from_be(tcp_info.tcpsi_ini.insi_fport as u16);
            (
                SocketAddr::new(local_ip, local_port),
                SocketAddr::new(remote_ip, remote_port),
            )
        } else {
            let local_ip = IpAddr::V6(Ipv6Addr::from(tcp_info.tcpsi_ini.insi_laddr.ina_6.s6_addr));
            let remote_ip = IpAddr::V6(Ipv6Addr::from(tcp_info.tcpsi_ini.insi_faddr.ina_6.s6_addr));
            let local_port = u16::from_be(tcp_info.tcpsi_ini.insi_lport as u16);
            let remote_port = u16::from_be(tcp_info.tcpsi_ini.insi_fport as u16);
            (
                SocketAddr::new(local_ip, local_port),
                SocketAddr::new(remote_ip, remote_port),
            )
        }
    }
}

impl MacosTCPConnectionsCollector {
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

    pub fn collect_tcp_connections(&self) -> Result<TCPConnectionsData, SensorError> {
        self.id_to_pid.borrow_mut().clear();

        let mut connections = Vec::new();

        let Ok(pids) = pids_by_type(ProcFilter::All) else {
            return Err(SensorError::ReadError(
                "Problem while listing procs for socket".to_string(),
            ));
        };

        for pid in pids {
            let fds = match libproc::file_info::listpidinfo::<ListFDs>(pid as i32, 0) {
                Ok(f) => f,
                Err(_) => continue,
            };
            for fd in fds {
                if fd.proc_fdtype != ProcFDType::Socket as u32 {
                    continue;
                }
                let Ok(socket_info) = libproc::file_info::pidfdinfo::<SocketFDInfo>(pid as i32, fd.proc_fd) else {
                    continue;
                };

                if socket_info.psi.soi_kind != SocketInfoKind::Tcp as i32 {
                    continue;
                }
                let tcp_info = unsafe { socket_info.psi.soi_proto.pri_tcp };

                if tcp_info.tcpsi_state != TcpSIState::ESTABLISHED as i32 {
                    continue;
                }
                let (local_addr, remote_addr) = extract_addrs(&tcp_info);

                let (ep_min, ep_max) = self.ephemeral_port_range;
                let local_port = local_addr.port();

                let is_maybe_client = if local_port > ep_min && local_port < ep_max {
                    Some(true)
                } else {
                    Some(false)
                };

                let key = TCPConnectionKey::new(self.machine_name.to_string(), local_addr, remote_addr);
                let id = key.into_tcp_connection_id();

                self.id_to_pid.borrow_mut().insert(id.clone(), *pid);

                let sent_bytes = None; // TODO: find good field
                let recv_bytes = None;

                let data = TCPConnectionData {
                    connection_id: id,
                    local_addr,
                    remote_addr,
                    maybe_client: is_maybe_client,

                    // Need process sensor information
                    local_process_id: None,

                    recv_bytes,
                    sent_bytes,
                };
                connections.push(data);
            }
        }
        Ok(TCPConnectionsData(connections))
    }
}
