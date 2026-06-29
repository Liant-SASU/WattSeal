use std::{
    cell::RefCell,
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};

use common::{Byte, TCPConnectionData, TCPConnectionID, TCPConnectionsData};
use windows::Win32::{
    NetworkManagement::IpHelper::{
        GetExtendedTcpTable, GetPerTcpConnectionEStats, MIB_TCP6TABLE_OWNER_PID, MIB_TCPROW_OWNER_PID,
        MIB_TCPTABLE_OWNER_PID, TCP_ESTATS_BANDWIDTH_ROD_v0, TCP_TABLE_OWNER_PID_CONNECTIONS,
        TcpConnectionEstatsBandwidth,
    },
    Networking::WinSock::{ADDRESS_FAMILY, AF_INET, AF_INET6},
};

use crate::sensors::{SensorError, tcp_connections::TCPConnectionKey};

pub struct WindowsTCPConnectionsCollector {
    ephemeral_port_range: (u16, u16),
    machine_name: String,
    id_to_pid: RefCell<HashMap<TCPConnectionID, u32>>,
}

const DEFAULT_MIN_EPHEMERAL_PORT: u16 = 49152;
const DEFAULT_MAX_EPHEMERAL_PORT: u16 = 65535;

fn get_tcp_estats(row: &MIB_TCPROW_OWNER_PID) -> (Option<Byte>, Option<Byte>) {
    let mut rod = TCP_ESTATS_BANDWIDTH_ROD_v0::default();
    let rod_size = std::mem::size_of::<TCP_ESTATS_BANDWIDTH_ROD_v0>() as u32;

    let rod_bytes = unsafe { std::slice::from_raw_parts_mut(&mut rod as *mut _ as *mut u8, rod_size as usize) };

    let result = unsafe {
        GetPerTcpConnectionEStats(
            row as *const _ as *mut _,
            TcpConnectionEstatsBandwidth,
            None,
            0,
            None,
            0,
            Some(rod_bytes),
            rod_size,
        )
    };

    let bytes = if result != 0 {
        (None, None)
    } else {
        (
            Some(Byte::from(rod.OutboundBandwidth)),
            Some(Byte::from(rod.InboundBandwidth)),
        )
    };
    bytes
}

fn get_tcp6_estats(row: &MIB_TCP6ROW_OWNER_PID) -> (Option<Byte>, Option<Byte>) {
    let mut rod = TCP_ESTATS_BANDWIDTH_ROD_v0::default();
    let rod_size = std::mem::size_of::<TCP_ESTATS_BANDWIDTH_ROD_v0>() as u32;
    let rod_bytes = unsafe { std::slice::from_raw_parts_mut(&mut rod as *mut _ as *mut u8, rod_size as usize) };

    let result = unsafe {
        GetPerTcp6ConnectionEStats(
            row as *const _ as *mut _,
            TcpConnectionEstatsBandwidth,
            None,
            0,
            None,
            0,
            Some(rod_bytes),
            rod_size,
        )
    };

    if result != 0 {
        (None, None)
    } else {
        (
            Some(Byte::from(rod.OutboundBandwidth)),
            Some(Byte::from(rod.InboundBandwidth)),
        )
    }
}

impl WindowsTCPConnectionsCollector {
    pub fn new(machine_name: String) -> Self {
        Self {
            ephemeral_port_range: (DEFAULT_MIN_EPHEMERAL_PORT, DEFAULT_MAX_EPHEMERAL_PORT),
            machine_name,
            id_to_pid: RefCell::new(HashMap::new()),
        }
    }

    pub fn id_to_pid_map(&self) -> HashMap<TCPConnectionID, u32> {
        self.id_to_pid.borrow().clone()
    }

    fn collect_for_family(&self, family: ADDRESS_FAMILY) -> Vec<TCPConnectionData> {
        let mut connections = Vec::new();

        let mut buf_size: u32 = 0;
        unsafe {
            GetExtendedTcpTable(
                None,
                &mut buf_size,
                false,
                family.0 as u32,
                TCP_TABLE_OWNER_PID_CONNECTIONS,
                0,
            );
        }

        let mut buf = vec![0u8; buf_size as usize];
        let result = unsafe {
            GetExtendedTcpTable(
                Some(buf.as_mut_ptr() as *mut _),
                &mut buf_size,
                false,
                family.0 as u32,
                TCP_TABLE_OWNER_PID_CONNECTIONS,
                0,
            )
        };

        if result != 0 {
            return connections;
        }

        if family == AF_INET {
            self.parse_ipv4_table(&buf, &mut connections);
        } else if family == AF_INET6 {
            self.parse_ipv6_table(&buf, &mut connections);
        }

        connections
    }

    fn parse_ipv4_table(&self, buf: &[u8], connections: &mut Vec<TCPConnectionData>) {
        let table = unsafe { &*(buf.as_ptr() as *const MIB_TCPTABLE_OWNER_PID) };
        let rows = unsafe { std::slice::from_raw_parts(table.table.as_ptr(), table.dwNumEntries as usize) };

        for row in rows {
            let local_addr = SocketAddr::new(
                IpAddr::V4(Ipv4Addr::from(u32::from_be(row.dwLocalAddr))),
                u16::from_be(row.dwLocalPort as u16),
            );
            let remote_addr = SocketAddr::new(
                IpAddr::V4(Ipv4Addr::from(u32::from_be(row.dwRemoteAddr))),
                u16::from_be(row.dwRemotePort as u16),
            );

            let key = TCPConnectionKey::new(self.machine_name.to_string(), local_addr, remote_addr);
            let id = key.into_tcp_connection_id();

            self.id_to_pid.borrow_mut().insert(id.clone(), row.dwOwningPid);

            let (ep_min, ep_max) = self.ephemeral_port_range;
            let local_port = local_addr.port();

            let is_maybe_client = if local_port > ep_min && local_port < ep_max {
                Some(true)
            } else {
                Some(false)
            };

            let (recv_bytes, sent_bytes) = get_tcp_estats(row);

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
    fn parse_ipv6_table(&self, buf: &[u8], connections: &mut Vec<TCPConnectionData>) {
        let table = unsafe { &*(buf.as_ptr() as *const MIB_TCP6TABLE_OWNER_PID) };
        let rows = unsafe { std::slice::from_raw_parts(table.table.as_ptr(), table.dwNumEntries as usize) };

        for row in rows {
            let local_addr = SocketAddr::new(
                IpAddr::V6(Ipv6Addr::from(row.ucLocalAddr)),
                u16::from_be(row.dwLocalPort as u16),
            );
            let remote_addr = SocketAddr::new(
                IpAddr::V6(Ipv6Addr::from(row.ucRemoteAddr)),
                u16::from_be(row.dwRemotePort as u16),
            );
            let key = TCPConnectionKey::new(self.machine_name.to_string(), local_addr, remote_addr);
            let id = key.into_tcp_connection_id();

            self.id_to_pid.borrow_mut().insert(id.clone(), row.dwOwningPid);

            let (ep_min, ep_max) = self.ephemeral_port_range;
            let local_port = local_addr.port();

            let is_maybe_client = if local_port > ep_min && local_port < ep_max {
                Some(true)
            } else {
                Some(false)
            };

            let (recv_bytes, sent_bytes) = get_tcp6_estats(row);

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

    pub fn collect_tcp_connections(&self) -> Result<TCPConnectionsData, SensorError> {
        self.id_to_pid.borrow_mut().clear();

        let mut connections = Vec::new();

        // IPv4
        connections.extend(self.collect_for_family(AF_INET));
        // IPv6
        connections.extend(self.collect_for_family(AF_INET6));

        Ok(TCPConnectionsData(connections))
    }
}
