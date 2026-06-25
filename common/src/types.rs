use std::{collections::HashMap, fmt::Display, time::SystemTime};

use serde::{Deserialize, Serialize};

pub const MICROJOULES_PER_JOULE: f64 = 1_000_000.0;
pub const SECONDS_PER_HOUR: f64 = 3600.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct EnergyUj(u64);

impl std::fmt::Display for EnergyUj {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} µJ", self.0)
    }
}

impl EnergyUj {
    pub fn from_joules(joules: f64) -> Self {
        EnergyUj((joules * MICROJOULES_PER_JOULE) as u64)
    }

    pub fn from_millijoules(mj: u64) -> Self {
        EnergyUj(mj * 1_000)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }

    pub fn as_f64(&self) -> f64 {
        self.0 as f64
    }

    pub fn as_joules(&self) -> f64 {
        self.as_f64() / MICROJOULES_PER_JOULE
    }

    pub fn as_watts_for_seconds(&self, seconds: f64) -> f64 {
        if seconds <= 0.0 {
            0.0
        } else {
            self.as_joules() / seconds
        }
    }

    pub fn as_watt_hours(&self) -> f64 {
        self.as_joules() / SECONDS_PER_HOUR
    }

    pub fn scale_by(self, factor: f64) -> Self {
        if !factor.is_finite() || factor <= 0.0 {
            EnergyUj(0)
        } else {
            EnergyUj::from_f64(self.as_f64() * factor)
        }
    }

    pub fn from_f64(value: f64) -> Self {
        EnergyUj(value.max(0.0) as u64)
    }

    pub fn from_u64(value: u64) -> Self {
        EnergyUj(value)
    }

    pub fn to_wh(self) -> EnergyWh {
        EnergyWh(self.as_watt_hours())
    }
}

impl std::ops::AddAssign for EnergyUj {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_add(rhs.0);
    }
}

impl std::ops::SubAssign for EnergyUj {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_sub(rhs.0);
    }
}

impl PartialEq<f64> for EnergyUj {
    fn eq(&self, other: &f64) -> bool {
        self.as_f64() == *other
    }
}

impl PartialOrd<f64> for EnergyUj {
    fn partial_cmp(&self, other: &f64) -> Option<std::cmp::Ordering> {
        self.as_f64().partial_cmp(other)
    }
}

impl std::ops::Mul<f64> for EnergyUj {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        self.scale_by(rhs)
    }
}

impl std::ops::MulAssign<f64> for EnergyUj {
    fn mul_assign(&mut self, rhs: f64) {
        *self = self.scale_by(rhs);
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct EnergyWh(f64);

impl std::fmt::Display for EnergyWh {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.3} Wh", self.0)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PowerW(f64);

//byte unit
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Byte(u64);

impl Byte {
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    pub fn as_f64(&self) -> f64 {
        self.0 as f64
    }

    pub fn as_mb(&self) -> f64 {
        self.as_f64() / 1_000_000.0
    }

    pub fn from(value: u64) -> Self {
        Byte(value)
    }
}

impl std::fmt::Display for Byte {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} B", self.0)
    }
}

impl std::ops::AddAssign for Byte {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_add(rhs.0);
    }
}

impl std::ops::AddAssign<u64> for Byte {
    fn add_assign(&mut self, rhs: u64) {
        self.0 = self.0.saturating_add(rhs);
    }
}

/// Timestamped collection of sensor readings.
#[derive(Debug, Clone)]
pub struct Event {
    time: SystemTime,
    data: Vec<SensorData>,
}

impl Event {
    /// Creates an event with the given timestamp and sensor data.
    pub fn new(time: SystemTime, data: Vec<SensorData>) -> Self {
        Event { time, data }
    }

    /// Returns the event timestamp.
    pub fn time(&self) -> SystemTime {
        self.time
    }

    /// Returns the list of sensor readings.
    pub fn data(&self) -> &Vec<SensorData> {
        &self.data
    }

    /// Appends a sensor reading to this event.
    pub fn push_data(&mut self, data: SensorData) {
        self.data.push(data);
    }
}

/// Cumulative per-component total consumption values.
#[derive(Debug, Clone, Default)]
pub struct AllTimeData<E = EnergyUj> {
    pub components: HashMap<String, E>,
}

/// CPU energy and usage readings.
#[derive(Debug, Clone, Serialize)]
pub struct CPUData<E = EnergyUj> {
    pub total_energy: Option<E>,
    pub pp0_energy: Option<E>,
    pub pp1_energy: Option<E>,
    pub dram_energy: Option<E>,
    pub usage_percent: Option<f64>,
}

/// GPU energy and usage readings.
#[derive(Debug, Clone, Serialize)]
pub struct GPUData<E = EnergyUj> {
    pub total_energy: Option<E>,
    pub usage_percent: Option<f64>,
    pub vram_usage_percent: Option<f64>,
}

/// RAM energy and usage readings.
#[derive(Debug, Clone, Serialize)]
pub struct RamData<E = EnergyUj> {
    pub total_energy: Option<E>,
    pub usage_percent: Option<f64>,
}

/// Disk energy and I/O throughput readings.
#[derive(Debug, Clone, Serialize)]
pub struct DiskData<E = EnergyUj> {
    pub total_energy: Option<E>,
    pub read_bytes: Byte,
    pub written_bytes: Byte,
}

/// Network energy and throughput readings.
#[derive(Debug, Clone, Serialize)]
pub struct NetworkData<E = EnergyUj> {
    pub total_energy: Option<E>,
    pub downloaded_bytes: Byte,
    pub uploaded_bytes: Byte,
}

/// Process identifier
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Hash)]
pub struct ProcessID(pub u64);

/// Process identifier, heredity, usage and thoughput readings
#[derive(Debug, Clone, Serialize)]
pub struct ProcessData {
    pub process_id: ProcessID,
    pub name: String,
    pub parent: Option<ProcessID>,
    pub exe_path: Option<String>,
    pub cpu_usage: Option<f64>,
    pub gpu_usage: Option<f64>,
    pub ram_usage: Option<f64>,
    pub read_bytes: Option<u64>,
    pub written_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessesData(pub Vec<ProcessData>);

/// Tagged union of all sensor reading types.
#[derive(Debug, Clone, Serialize)]
pub enum SensorData<E = EnergyUj> {
    CPU(CPUData<E>),
    GPU(GPUData<E>),
    Ram(RamData<E>),
    Disk(DiskData<E>),
    Network(NetworkData<E>),
    Processes(ProcessesData),
}

/// Sensor component category type.
#[derive(Debug, Clone)]
pub enum SensorKind {
    CPU,
    GPU,
    Ram,
    Disk,
    Network,
    Processes,
}

/// Hardware information variant collected at startup.
pub enum InitialInfo {
    System(SystemInfo),
    CPU(CpuInfo),
    Memory(MemoryInfo),
    Gpus(Vec<String>),
    Disks(Vec<DiskInfo>),
    Battery(BatteryInfo),
}

/// Complete hardware inventory of the system.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HardwareInfo {
    pub system: SystemInfo,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub gpus: Vec<String>,
    pub disks: Vec<DiskInfo>,
    pub battery: BatteryInfo,
}

impl HardwareInfo {
    /// Serializes this hardware info to a JSON string.
    pub fn serialized(&self) -> String {
        match serde_json::to_string(self) {
            Ok(json_string) => json_string,
            Err(e) => {
                crate::clog!("✗ Failed to serialize hardware info to JSON: {}", e);
                "{}".to_string()
            }
        }
    }
}

/// Metadata pairing entry list with serialized hardware info.
#[derive(Debug, Clone)]
pub struct GeneralData {
    pub sensors: Vec<SensorKind>,
    pub hardware_info: HardwareInfo,
}

impl From<Vec<InitialInfo>> for HardwareInfo {
    fn from(infos: Vec<InitialInfo>) -> Self {
        let mut system_info = None;
        let mut cpu_info = None;
        let mut memory_info = None;
        let mut gpu_list = None;
        let mut disk_infos = None;
        let mut battery_info = None;

        for info in infos {
            match info {
                InitialInfo::System(sys) => system_info = Some(sys),
                InitialInfo::CPU(cpu) => cpu_info = Some(cpu),
                InitialInfo::Memory(mem) => memory_info = Some(mem),
                InitialInfo::Gpus(gpus) => gpu_list = Some(gpus),
                InitialInfo::Disks(disks) => disk_infos = Some(disks),
                InitialInfo::Battery(battery) => battery_info = Some(battery),
            }
        }

        HardwareInfo {
            system: system_info.unwrap_or_default(),
            cpu: cpu_info.unwrap_or_default(),
            memory: memory_info.unwrap_or_default(),
            gpus: gpu_list.unwrap_or_default(),
            disks: disk_infos.unwrap_or_default(),
            battery: battery_info.unwrap_or_default(),
        }
    }
}

/// Operating system and host information.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemInfo {
    pub os: String,
    pub hostname: String,
    pub is_virtual_machine: bool,
}

/// CPU model, vendor, and core count.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CpuInfo {
    pub name: String,
    pub vendor: String,
    pub physical_cores: u16,
    pub logical_cores: u16,
    pub base_frequency_mhz: u64,
    pub architecture: String,
}

/// Total physical and swap memory sizes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryInfo {
    pub total_ram_bytes: u64,
    pub total_swap_bytes: u64,
}

/// Disk name, mount point, and capacity.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub file_system: String,
    pub disk_type: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
}

/// Battery presence, capacity, and cycle count.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BatteryInfo {
    pub present: bool,
    pub name: Option<String>,
    pub design_capacity_wh: Option<f32>,
    pub full_charge_capacity_wh: Option<f32>,
    pub cycle_count: Option<u32>,
}

impl SensorKind {
    pub fn label(&self) -> &'static str {
        match self {
            SensorKind::CPU => "CPU",
            SensorKind::GPU => "GPU",
            SensorKind::Ram => "Ram",
            SensorKind::Disk => "Disk",
            SensorKind::Network => "Network",
            SensorKind::Processes => "Processes",
        }
    }
}

impl<E: Clone> SensorData<E> {
    /// Returns the sensor kind of this sensor variant.
    pub fn sensor_kind(&self) -> SensorKind {
        match self {
            SensorData::CPU(_) => SensorKind::CPU,
            SensorData::GPU(_) => SensorKind::GPU,
            SensorData::Ram(_) => SensorKind::Ram,
            SensorData::Disk(_) => SensorKind::Disk,
            SensorData::Network(_) => SensorKind::Network,
            SensorData::Processes(_) => SensorKind::Processes,
        }
    }

    /// Returns the total energy, if available.
    pub fn total_energy(&self) -> Option<E> {
        match self {
            SensorData::CPU(data) => data.total_energy.clone(),
            SensorData::GPU(data) => data.total_energy.clone(),
            SensorData::Ram(data) => data.total_energy.clone(),
            SensorData::Disk(data) => data.total_energy.clone(),
            SensorData::Network(data) => data.total_energy.clone(),
            SensorData::Processes(_) => None,
        }
    }
}

impl Display for SensorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SensorKind::CPU => write!(f, "CPU"),
            SensorKind::GPU => write!(f, "GPU"),
            SensorKind::Ram => write!(f, "Ram"),
            SensorKind::Disk => write!(f, "Disk"),
            SensorKind::Network => write!(f, "Network"),
            SensorKind::Processes => write!(f, "Processes"),
        }
    }
}

impl<T: Display> Display for SensorData<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SensorData::CPU(data) => {
                writeln!(f, "CPU Data:")?;
                writeln!(
                    f,
                    "  Energy PKG:  {}",
                    data.total_energy
                        .as_ref()
                        .map(|c| format!("{c}"))
                        .unwrap_or_else(|| "N/A".to_string())
                )?;
                writeln!(
                    f,
                    "  Energy PP0:  {}",
                    data.pp0_energy
                        .as_ref()
                        .map(|c| format!("{c}"))
                        .unwrap_or_else(|| "N/A".to_string())
                )?;
                writeln!(
                    f,
                    "  Energy PP1:  {}",
                    data.pp1_energy
                        .as_ref()
                        .map(|c| format!("{c}"))
                        .unwrap_or_else(|| "N/A".to_string())
                )?;
                writeln!(
                    f,
                    "  Energy DRAM: {}",
                    data.dram_energy
                        .as_ref()
                        .map(|c| format!("{c}"))
                        .unwrap_or_else(|| "N/A".to_string())
                )?;
                writeln!(
                    f,
                    "  Usage:      {}",
                    data.usage_percent
                        .map(|u| format!("{:.2} %", u))
                        .unwrap_or_else(|| "N/A".to_string())
                )?;
                Ok(())
            }
            SensorData::GPU(data) => {
                writeln!(f, "GPU Data:")?;
                writeln!(
                    f,
                    "  Energy:       {}",
                    data.total_energy
                        .as_ref()
                        .map(|c| format!("{c}"))
                        .unwrap_or_else(|| "N/A".to_string())
                )?;
                writeln!(
                    f,
                    "  Usage:       {}",
                    data.usage_percent
                        .map(|u| format!("{:.2} %", u))
                        .unwrap_or_else(|| "N/A".to_string())
                )?;
                writeln!(
                    f,
                    "  VRAM Usage:  {}",
                    data.vram_usage_percent
                        .map(|u| format!("{:.2} %", u))
                        .unwrap_or_else(|| "N/A".to_string())
                )?;
                Ok(())
            }
            SensorData::Ram(data) => {
                writeln!(f, "RAM Data:")?;
                writeln!(
                    f,
                    "  Consumption: {}",
                    data.total_energy
                        .as_ref()
                        .map(|c| format!("{c}"))
                        .unwrap_or_else(|| "N/A".to_string())
                )?;
                writeln!(
                    f,
                    " Usage: {}",
                    data.usage_percent
                        .map(|u| format!("{:.2} %", u))
                        .unwrap_or_else(|| "N/A".to_string())
                )?;
                Ok(())
            }
            SensorData::Disk(data) => {
                writeln!(f, "Disk Data:")?;
                writeln!(
                    f,
                    "  Energy: {}",
                    data.total_energy
                        .as_ref()
                        .map(|c| format!("{c}"))
                        .unwrap_or_else(|| "N/A".to_string())
                )?;
                writeln!(f, "  Read Bytes:  {} B", data.read_bytes)?;
                writeln!(f, "  Write Speed: {} B", data.written_bytes)?;
                Ok(())
            }
            SensorData::Network(data) => {
                writeln!(f, "Network Data:")?;
                writeln!(
                    f,
                    "  Energy:        {}",
                    data.total_energy
                        .as_ref()
                        .map(|c| format!("{c}"))
                        .unwrap_or_else(|| "N/A".to_string())
                )?;
                writeln!(f, "  Downloaded Bytes: {:.2} MB/s", data.downloaded_bytes)?;
                writeln!(f, "  Uploaded Bytes:   {:.2} MB/s", data.uploaded_bytes)?;
                Ok(())
            }
            SensorData::Processes(data) => {
                writeln!(f, "Processes Data:")?;
                for p in data.0.iter() {
                    writeln!(f, " - {} (ID: {}):", p.name, p.process_id.0)?;

                    if let Some(ref parent) = p.parent {
                        writeln!(f, "       Parent ID: {}", parent.0)?;
                    }

                    if let Some(ref exe) = p.exe_path {
                        writeln!(f, "       Path exe: {}", exe)?;
                    }

                    if let Some(cpu) = p.cpu_usage {
                        writeln!(f, "       CPU Usage: {:.2}%", cpu)?;
                    }

                    if let Some(gpu) = p.gpu_usage {
                        writeln!(f, "       GPU Usage: {:.2}%", gpu)?;
                    }

                    if let Some(ram) = p.ram_usage {
                        writeln!(f, "       RAM Usage: {:.2}%", ram)?;
                    }

                    if let Some(ref read_bytes) = p.read_bytes {
                        writeln!(f, "       Read Bytes: {} B", read_bytes)?;
                    }
                    if let Some(ref written_bytes) = p.written_bytes {
                        writeln!(f, "       Written Bytes: {} B", written_bytes)?;
                    }
                }
                Ok(())
            }
        }
    }
}

impl<T> From<CPUData<T>> for SensorData<T> {
    fn from(data: CPUData<T>) -> Self {
        SensorData::CPU(data)
    }
}

impl<T> From<GPUData<T>> for SensorData<T> {
    fn from(data: GPUData<T>) -> Self {
        SensorData::GPU(data)
    }
}

impl<T> From<RamData<T>> for SensorData<T> {
    fn from(data: RamData<T>) -> Self {
        SensorData::Ram(data)
    }
}
impl<T> From<DiskData<T>> for SensorData<T> {
    fn from(data: DiskData<T>) -> Self {
        SensorData::Disk(data)
    }
}
impl<T> From<NetworkData<T>> for SensorData<T> {
    fn from(data: NetworkData<T>) -> Self {
        SensorData::Network(data)
    }
}

impl CPUData {
    fn to_wh(&self) -> CPUData<EnergyWh> {
        CPUData {
            total_energy: self.total_energy.map(|t| t.to_wh()),
            pp0_energy: self.pp0_energy.map(|pp0| pp0.to_wh()),
            pp1_energy: self.pp1_energy.map(|pp1| pp1.to_wh()),
            dram_energy: self.dram_energy.map(|dram| dram.to_wh()),
            usage_percent: self.usage_percent,
        }
    }
}

impl GPUData {
    fn to_wh(&self) -> GPUData<EnergyWh> {
        GPUData {
            total_energy: self.total_energy.map(|t| t.to_wh()),
            usage_percent: self.usage_percent,
            vram_usage_percent: self.vram_usage_percent,
        }
    }
}

impl RamData {
    fn to_wh(&self) -> RamData<EnergyWh> {
        RamData {
            total_energy: self.total_energy.map(|t| t.to_wh()),
            usage_percent: self.usage_percent,
        }
    }
}

impl DiskData {
    fn to_wh(&self) -> DiskData<EnergyWh> {
        DiskData {
            total_energy: self.total_energy.map(|t| t.to_wh()),
            read_bytes: self.read_bytes,
            written_bytes: self.written_bytes,
        }
    }
}

impl NetworkData {
    fn to_wh(&self) -> NetworkData<EnergyWh> {
        NetworkData {
            total_energy: self.total_energy.map(|t| t.to_wh()),
            downloaded_bytes: self.downloaded_bytes,
            uploaded_bytes: self.uploaded_bytes,
        }
    }
}

impl SensorData {
    pub fn to_wh(&self) -> SensorData<EnergyWh> {
        match self {
            SensorData::CPU(cpudata) => SensorData::CPU(cpudata.to_wh()),
            SensorData::GPU(gpudata) => SensorData::GPU(gpudata.to_wh()),
            SensorData::Ram(ramdata) => SensorData::Ram(ramdata.to_wh()),
            SensorData::Disk(diskdata) => SensorData::Disk(diskdata.to_wh()),
            SensorData::Network(networkdata) => SensorData::Network(networkdata.to_wh()),
            SensorData::Processes(processesdata) => SensorData::Processes(processesdata.clone()),
        }
    }
}
