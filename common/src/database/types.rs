use std::{fmt::Display, time::SystemTime};

use rusqlite::{
    ToSql,
    types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef},
};
use serde::{Deserialize, Serialize};

use crate::{
    AllTimeData, Byte, CPUData, DatabaseEntry, DiskData, EnergyUj, GPUData, GeneralData, NetworkData, RamData,
    SensorData, SensorKind,
};

impl ToSql for EnergyUj {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.as_u64() as i64))
    }
}

impl FromSql for EnergyUj {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value
            .as_i64()
            .map(|v| EnergyUj::from_u64(v as u64))
            .map_err(|e| FromSqlError::Other(Box::new(e)))
    }
}

impl ToSql for Byte {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.as_u64() as i64))
    }
}

impl FromSql for Byte {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value
            .as_i64()
            .map(|v| Byte::from(v as u64))
            .map_err(|e| FromSqlError::Other(Box::new(e)))
    }
}

/// Sensors data for database
pub struct EventDB {
    time: SystemTime,
    data: Vec<DataDB>,
}

pub type CPUDataDB = CPUData<EnergyUj>;
pub type GPUDataDB = GPUData<EnergyUj>;
pub type RamDataDB = RamData<EnergyUj>;
pub type DiskDataDB = DiskData<EnergyUj>;
pub type NetworkDataDB = NetworkData<EnergyUj>;
pub type SensorDataDB = SensorData<EnergyUj>;
pub type AllTimeDataDB = AllTimeData<EnergyUj>;

// Sensors dependant data used in database.
#[derive(Debug, Clone)]
pub enum DataDB {
    Sensor(SensorDataDB),
    Process(Vec<ProcessDataDB>),
    Total(TotalDataDB),
}

/// Raw RGBA icon pixel data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconData {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Per-application resource usage snapshot.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessDataDB {
    pub app_name: String,
    pub process_exe_path: Option<String>,
    pub process_energy: EnergyUj,
    pub process_cpu_usage: f64,
    pub process_gpu_usage: Option<f64>,
    pub process_mem_usage: f64,
    pub read_bytes: Byte,
    pub written_bytes: Byte,
    pub subprocess_count: u32,
    pub icon: Option<IconData>,
}

/// Aggregated total consumption across all components.
#[derive(Debug, Clone, Serialize)]
pub struct TotalDataDB {
    pub total_energy: EnergyUj,
}

#[derive(Debug)]
pub struct GeneralDataDB {
    pub tables: String,
    pub hardware_info_serialized: String,
}

/// Different kind of sensors dependent data kind.
#[derive(Debug, Clone)]
pub enum DataKindDB {
    CPU,
    GPU,
    Ram,
    Disk,
    Network,
    Process,
    Total,
}

impl EventDB {
    /// Creates an event with the given timestamp and data.
    pub fn new(time: SystemTime, data: Vec<DataDB>) -> Self {
        EventDB { time, data }
    }

    /// Returns the event timestamp.
    pub fn time(&self) -> SystemTime {
        self.time
    }

    /// Returns the list of sensor readings.
    pub fn data(&self) -> &Vec<DataDB> {
        &self.data
    }

    /// Appends a sensor reading to this event.
    pub fn push_data(&mut self, data: DataDB) {
        self.data.push(data);
    }
}

impl From<SensorKind> for DataKindDB {
    fn from(sk: SensorKind) -> Self {
        match sk {
            SensorKind::CPU => DataKindDB::CPU,
            SensorKind::GPU => DataKindDB::GPU,
            SensorKind::Ram => DataKindDB::Ram,
            SensorKind::Disk => DataKindDB::Disk,
            SensorKind::Network => DataKindDB::Network,
        }
    }
}

impl From<CPUDataDB> for DataDB {
    fn from(data: CPUDataDB) -> Self {
        DataDB::Sensor(SensorDataDB::CPU(data))
    }
}

impl From<GPUDataDB> for DataDB {
    fn from(data: GPUDataDB) -> Self {
        DataDB::Sensor(SensorData::GPU(data))
    }
}

impl From<RamDataDB> for DataDB {
    fn from(data: RamDataDB) -> Self {
        DataDB::Sensor(SensorData::Ram(data))
    }
}
impl From<DiskDataDB> for DataDB {
    fn from(data: DiskDataDB) -> Self {
        DataDB::Sensor(SensorData::Disk(data))
    }
}
impl From<NetworkDataDB> for DataDB {
    fn from(data: NetworkDataDB) -> Self {
        DataDB::Sensor(SensorData::Network(data))
    }
}

impl From<ProcessDataDB> for DataDB {
    fn from(data: ProcessDataDB) -> Self {
        DataDB::Process(vec![data])
    }
}

impl From<TotalDataDB> for DataDB {
    fn from(data: TotalDataDB) -> Self {
        DataDB::Total(data)
    }
}

impl From<&SensorData<EnergyUj>> for DataDB {
    fn from(sensor_data: &SensorData<EnergyUj>) -> Self {
        DataDB::Sensor(sensor_data.clone())
    }
}

impl From<GeneralData> for GeneralDataDB {
    fn from(data: GeneralData) -> Self {
        let mut tables: Vec<&str> = data
            .sensors
            .iter()
            .map(|s| DataKindDB::from(s.clone()).table_name())
            .collect();

        tables.push(ProcessDataDB::table_name_static());
        tables.push(TotalDataDB::table_name_static());

        Self {
            tables: tables.join(","),
            hardware_info_serialized: data.hardware_info.serialized(),
        }
    }
}

impl DataDB {
    /// Returns the total consumption value, if available.
    pub fn total_energy(&self) -> Option<EnergyUj> {
        match self {
            DataDB::Sensor(s) => s.total_energy(),
            DataDB::Total(power) => Some(power.total_energy.clone()),
            DataDB::Process(_) => None,
        }
    }

    /// Returns the database table name for this variant.
    pub fn table_name(&self) -> &'static str {
        match self {
            DataDB::Sensor(SensorDataDB::CPU(_)) => CPUDataDB::table_name_static(),
            DataDB::Sensor(SensorDataDB::GPU(_)) => GPUDataDB::table_name_static(),
            DataDB::Sensor(SensorDataDB::Ram(_)) => RamDataDB::table_name_static(),
            DataDB::Sensor(SensorDataDB::Disk(_)) => DiskDataDB::table_name_static(),
            DataDB::Sensor(SensorDataDB::Network(_)) => NetworkDataDB::table_name_static(),
            DataDB::Process(_) => ProcessDataDB::table_name_static(),
            DataDB::Total(_) => TotalDataDB::table_name_static(),
        }
    }

    /// Returns secondary metrics (usage or speed) if applicable.
    pub fn secondary_values(&self) -> Option<SecondaryValues> {
        let metric_type = self.secondary_metric()?;
        if let DataDB::Sensor(s) = self {
            match s {
                SensorData::CPU(data) => Some(SecondaryValues::from_labeled_values(
                    metric_type,
                    vec![LabeledValue::from_usage_percent(data.usage_percent)],
                )),
                SensorData::GPU(data) => Some(SecondaryValues::from_labeled_values(
                    metric_type,
                    vec![LabeledValue::from_usage_percent(data.usage_percent)],
                )),
                SensorData::Ram(data) => Some(SecondaryValues::from_labeled_values(
                    metric_type,
                    vec![LabeledValue::from_usage_percent(data.usage_percent)],
                )),
                SensorData::Disk(data) => Some(SecondaryValues::from_labeled_values(
                    metric_type,
                    vec![
                        LabeledValue::from_mb_s(Some(data.read_bytes.as_mb()), "Read"),
                        LabeledValue::from_mb_s(Some(data.written_bytes.as_mb()), "Write"),
                    ],
                )),
                SensorData::Network(data) => Some(SecondaryValues::from_labeled_values(
                    metric_type,
                    vec![
                        LabeledValue::from_mb_s(Some(data.downloaded_bytes.as_mb()), "Download"),
                        LabeledValue::from_mb_s(Some(data.uploaded_bytes.as_mb()), "Upload"),
                    ],
                )),
            }
        } else {
            None
        }
    }

    /// Returns the secondary metric kind for this sensor variant.
    pub fn secondary_metric(&self) -> Option<MetricKindDB> {
        if let DataDB::Sensor(s) = self {
            match s {
                SensorData::CPU(_) | SensorData::GPU(_) | SensorData::Ram(_) => Some(MetricKindDB::Usage),
                SensorData::Disk(_) | SensorData::Network(_) => Some(MetricKindDB::Speed),
            }
        } else {
            None
        }
    }

    /// Scales all energy fields by `factor`.
    pub fn scale_energy(&mut self, factor: f64) {
        match self {
            DataDB::Sensor(SensorData::CPU(d)) => d.total_energy = d.total_energy.clone().map(|w| w * factor),
            DataDB::Sensor(SensorData::GPU(d)) => d.total_energy = d.total_energy.clone().map(|w| w * factor),
            DataDB::Sensor(SensorData::Ram(d)) => d.total_energy = d.total_energy.clone().map(|w| w * factor),
            DataDB::Sensor(SensorData::Disk(d)) => d.total_energy = d.total_energy.clone().map(|w| w * factor),
            DataDB::Sensor(SensorData::Network(d)) => d.total_energy = d.total_energy.clone().map(|w| w * factor),
            DataDB::Total(d) => {
                d.total_energy *= factor;
            }
            DataDB::Process(procs) => {
                for p in procs {
                    p.process_energy *= factor;
                }
            }
        }
    }
}

impl DataKindDB {
    pub fn table_name(&self) -> &'static str {
        match self {
            DataKindDB::CPU => CPUDataDB::table_name_static(),
            DataKindDB::GPU => GPUDataDB::table_name_static(),
            DataKindDB::Total => TotalDataDB::table_name_static(),
            DataKindDB::Ram => RamDataDB::table_name_static(),
            DataKindDB::Disk => DiskDataDB::table_name_static(),
            DataKindDB::Network => NetworkDataDB::table_name_static(),
            DataKindDB::Process => ProcessDataDB::table_name_static(),
        }
    }
}

impl Default for CPUDataDB {
    fn default() -> Self {
        CPUDataDB {
            total_energy: Some(EnergyUj::from_u64(0)),
            pp0_energy: Some(EnergyUj::from_u64(0)),
            pp1_energy: Some(EnergyUj::from_u64(0)),
            dram_energy: Some(EnergyUj::from_u64(0)),
            usage_percent: Some(0.0),
        }
    }
}

impl Default for GPUDataDB {
    fn default() -> Self {
        GPUDataDB {
            total_energy: Some(EnergyUj::from_u64(0)),
            usage_percent: Some(0.0),
            vram_usage_percent: Some(0.0),
        }
    }
}

impl Default for RamDataDB {
    fn default() -> Self {
        RamDataDB {
            total_energy: Some(EnergyUj::from_u64(0)),
            usage_percent: Some(0.0),
        }
    }
}

impl Default for DiskDataDB {
    fn default() -> Self {
        DiskDataDB {
            total_energy: Some(EnergyUj::from_u64(0)),
            read_bytes: Byte::from(0),
            written_bytes: Byte::from(0),
        }
    }
}

impl Default for NetworkDataDB {
    fn default() -> Self {
        NetworkDataDB {
            total_energy: Some(EnergyUj::from_u64(0)),
            downloaded_bytes: Byte::from(0),
            uploaded_bytes: Byte::from(0),
        }
    }
}

impl Default for ProcessDataDB {
    fn default() -> Self {
        ProcessDataDB {
            app_name: String::new(),
            process_exe_path: None,
            process_energy: EnergyUj::from_u64(0),
            process_cpu_usage: 0.0,
            process_gpu_usage: None,
            process_mem_usage: 0.0,
            read_bytes: Byte::from(0),
            written_bytes: Byte::from(0),
            subprocess_count: 0,
            icon: None,
        }
    }
}

impl Default for TotalDataDB {
    fn default() -> Self {
        TotalDataDB {
            total_energy: EnergyUj::from_u64(0),
        }
    }
}

impl Display for DataDB {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sensor(s) => {
                writeln!(f, "{}", s)?;
                Ok(())
            }
            Self::Process(p) => {
                writeln!(f, "Top Processes by CPU Usage:")?;
                writeln!(
                    f,
                    "{:<30} {:>10} {:>10} {:>10} {:>10} {:>15} {:>15} {:>20}",
                    "App Name", "CPU %", "GPU %", "Mem %", "Power W", "Read MB/s", "Write MB/s", "Subprocesses"
                )?;
                for process in p {
                    write!(f, "{}", process)?;
                }
                Ok(())
            }
            Self::Total(t) => {
                writeln!(f, "{}", t)?;
                Ok(())
            }
        }
    }
}

impl Display for ProcessDataDB {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Force conversion for alignment
        let energy_str = self.process_energy.to_string();
        writeln!(
            f,
            "{:<30.30} {:>10.2} {:>10} {:>10.2} {:>16} {:>15.2} {:>15.2} {:>20}",
            self.app_name,
            self.process_cpu_usage,
            self.process_gpu_usage
                .map(|u| format!("{:.2} %", u))
                .unwrap_or_else(|| "N/A".to_string()),
            self.process_mem_usage,
            energy_str,
            self.read_bytes.as_mb(),
            self.written_bytes.as_mb(),
            self.subprocess_count
        )?;
        Ok(())
    }
}

impl Display for TotalDataDB {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Total Energy: {}", self.total_energy)?;
        Ok(())
    }
}

/// Different kind of metric in database.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum MetricKindDB {
    #[default]
    Power,
    Usage,
    Speed,
}

impl Display for MetricKindDB {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricKindDB::Power => write!(f, "Power"),
            MetricKindDB::Usage => write!(f, "Usage"),
            MetricKindDB::Speed => write!(f, "Speed"),
        }
    }
}

impl MetricKindDB {
    /// Returns the human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            MetricKindDB::Power => "Power",
            MetricKindDB::Usage => "Usage",
            MetricKindDB::Speed => "Speed",
        }
    }

    /// Returns the measurement unit string.
    pub fn unit_label(&self) -> &'static str {
        match self {
            MetricKindDB::Power => "W",
            MetricKindDB::Usage => "%",
            MetricKindDB::Speed => "MB/s",
        }
    }

    /// Formats a chart legend label for the given component.
    pub fn legend(&self, component_name: &str) -> String {
        format!("{} {}", component_name, self.label())
    }

    /// Returns the display unit, swapping uj for Wh when energy mode, and W otherwise.
    pub fn effective_unit(&self, energy_mode: bool) -> &'static str {
        if *self == MetricKindDB::Power && energy_mode {
            "Wh"
        } else {
            self.unit_label()
        }
    }
}

/// Named optional numeric value for secondary metrics.
#[derive(Debug, Clone, Copy)]
pub struct LabeledValue {
    pub label: &'static str,
    pub value: Option<f64>,
}

/// Collection of secondary metric values with their type.
#[derive(Debug, Clone)]
pub struct SecondaryValues {
    pub metric_kind: MetricKindDB,
    pub values: Vec<LabeledValue>,
}

impl SecondaryValues {
    fn from_labeled_values(metric_kind: MetricKindDB, values: Vec<LabeledValue>) -> Self {
        Self { metric_kind, values }
    }

    /// Returns the list of labeled values.
    pub fn values(&self) -> &Vec<LabeledValue> {
        &self.values
    }

    /// Returns the metric type of these secondary values.
    pub fn metric_type(&self) -> MetricKindDB {
        self.metric_kind
    }
}

impl LabeledValue {
    fn from_percent(percent: Option<f64>, label: &'static str) -> Self {
        Self { label, value: percent }
    }

    fn from_usage_percent(percent: Option<f64>) -> Self {
        Self::from_percent(percent, "Usage")
    }

    fn from_mb_s(speed: Option<f64>, label: &'static str) -> Self {
        Self {
            label: label,
            value: speed,
        }
    }
}
