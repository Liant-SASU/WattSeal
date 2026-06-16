use std::collections::HashMap;

use rusqlite::{Row, ToSql};

use crate::database::types::{
    AllTimeDataDB, CPUDataDB, DataDB, DiskDataDB, GPUDataDB, NetworkDataDB, ProcessDataDB, RamDataDB, TotalDataDB,
};

/// Maps a data type to its SQLite table schema and row conversion.
pub trait DatabaseEntry {
    fn generic_name() -> &'static str;
    fn table_name_static() -> &'static str;
    fn insert_params<'a>(&'a self, timestamp_id: &'a i64) -> Vec<&'a dyn ToSql>;
    fn columns_static() -> &'static [(&'static str, &'static str)];
    fn from_row(row: &Row) -> rusqlite::Result<Self>
    where
        Self: Sized;

    fn zero() -> DataDB
    where
        Self: Default + Into<DataDB>,
    {
        Self::default().into()
    }

    fn insert_sql() -> String {
        let cols = Self::columns_static();
        let col_names: Vec<&str> = cols.iter().map(|(name, _)| *name).collect();
        let all_cols = format!("timestamp_id, {}", col_names.join(", "));
        let params: Vec<String> = (1..=cols.len() + 1).map(|i| format!("?{}", i)).collect();
        format!(
            "INSERT INTO {} ({}) VALUES ({})",
            Self::table_name_static(),
            all_cols,
            params.join(", ")
        )
    }

    fn create_table_sql() -> String {
        let mut col_defs = vec![
            "id INTEGER PRIMARY KEY".to_string(),
            "timestamp_id INTEGER NOT NULL REFERENCES timestamp(id) ON DELETE CASCADE".to_string(),
        ];
        for (name, type_) in Self::columns_static() {
            col_defs.push(format!("{} {}", name, type_));
        }
        let table_name = Self::table_name_static();
        format!(
            "CREATE TABLE IF NOT EXISTS {} ({});\
             CREATE INDEX IF NOT EXISTS idx_{}_timestamp_id ON {}(timestamp_id)",
            table_name,
            col_defs.join(", "),
            table_name,
            table_name,
        )
    }

    fn avg_columns_sql(prefix: &str) -> String {
        Self::columns_static()
            .iter()
            .map(|(col_name, _)| format!("AVG({}{}) AS {}", prefix, col_name, col_name))
            .collect::<Vec<String>>()
            .join(", ")
    }
}

macro_rules! impl_database_entry {
    (
        struct $type:ty {
            generic_name: $generic_name:expr,
            table_name: $table_name:expr,
            mappings: {
                $($field:ident : $col_name:literal => $sql_type:literal),* $(,)?
            }
            $(, extra_fields: { $($extra_field:ident : $extra_val:expr),* $(,)? } )?
        }
    ) => {
        impl DatabaseEntry for $type {
            fn generic_name() -> &'static str {
                $generic_name
            }

            fn table_name_static() -> &'static str {
                $table_name
            }

            fn columns_static() -> &'static [(&'static str, &'static str)] {
                &[ $(($col_name, $sql_type)),* ]
            }

            fn insert_params<'a>(&'a self, timestamp_id: &'a i64) -> Vec<&'a dyn ToSql> {
                let mut params: Vec<&'a dyn ToSql> = vec![timestamp_id];
                $( params.push(&self.$field); )*
                params
            }

            fn from_row(row: &Row) -> rusqlite::Result<Self> {
                Ok(Self {
                    $($field: row.get($col_name)?),*
                    $(, $($extra_field: $extra_val),* )?
                })
            }
        }
    };
}

impl_database_entry! {
    struct CPUDataDB {
        generic_name: "CPU",
        table_name: "cpu_data",
        mappings: {
            total_energy: "total_energy_uj" => "INTEGER",
            pp0_energy: "pp0_energy_uj" => "INTEGER",
            pp1_energy: "pp1_energy_uj" => "INTEGER",
            dram_energy: "dram_energy_uj" => "INTEGER",
            usage_percent: "usage_percent" => "REAL",
        }
    }
}

impl_database_entry! {
    struct GPUDataDB {
        generic_name: "GPU",
        table_name: "gpu_data",
        mappings: {
            total_energy: "total_energy_uj" => "INTEGER",
            usage_percent: "usage_percent" => "REAL",
            vram_usage_percent: "vram_usage_percent" => "REAL",
        }
    }
}

impl_database_entry! {
    struct DiskDataDB {
        generic_name: "Disk",
        table_name: "disk_data",
        mappings: {
            total_energy: "total_energy_uj" => "INTEGER",
            read_bytes: "read_bytes" => "INTEGER",
            written_bytes: "written_bytes" => "INTEGER",
        }
    }
}

impl_database_entry! {
    struct RamDataDB {
        generic_name: "RAM",
        table_name: "ram_data",
        mappings: {
            total_energy: "total_energy_uj" => "INTEGER",
            usage_percent: "usage_percent" => "REAL",
        }
    }
}

impl_database_entry! {
    struct NetworkDataDB {
        generic_name: "Network",
        table_name: "network_data",
        mappings: {
            total_energy: "total_energy_uj" => "INTEGER",
            downloaded_bytes: "downloaded_bytes" => "INTEGER",
            uploaded_bytes: "uploaded_bytes" => "INTEGER",
        }
    }
}

impl_database_entry! {
    struct TotalDataDB {
        generic_name: "Total",
        table_name: "total_data",
        mappings: {
            total_energy: "total_energy_uj" => "INTEGER",
        }
    }
}

impl_database_entry! {
    struct ProcessDataDB {
        generic_name: "Processes",
        table_name: "process_data",
        mappings: {
            app_name: "app_name" => "TEXT NOT NULL",
            process_exe_path: "process_exe_path" => "TEXT",
            process_energy: "process_energy_uj" => "INTEGER",
            process_cpu_usage: "process_cpu_usage" => "REAL",
            process_gpu_usage: "process_gpu_usage" => "REAL",
            process_mem_usage: "process_mem_usage" => "REAL",
            read_bytes: "read_bytes" => "INTEGER",
            written_bytes: "written_bytes" => "INTEGER",
            subprocess_count: "subprocess_count" => "INTEGER",
        },
        extra_fields: {
            icon: None,
        }
    }
}

// Manual fallback block preserved for custom structure handling
impl DatabaseEntry for AllTimeDataDB {
    fn generic_name() -> &'static str {
        "AllTime"
    }

    fn table_name_static() -> &'static str {
        "all_time_data"
    }

    fn insert_params<'a>(&'a self, _timestamp_id: &'a i64) -> Vec<&'a dyn ToSql> {
        vec![]
    }

    fn columns_static() -> &'static [(&'static str, &'static str)] {
        &[]
    }

    fn from_row(_: &Row) -> rusqlite::Result<Self> {
        Ok(AllTimeDataDB {
            components: HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CPUDataDB, DataDB, DatabaseEntry, DiskDataDB, GPUDataDB, NetworkDataDB, ProcessDataDB, RamDataDB, TotalDataDB,
    };
    use crate::{
        SensorDataDB,
        types::{Byte, EnergyUj},
    };

    #[test]
    fn zero_defaults_are_zero_filled() {
        // CPU
        match CPUDataDB::zero() {
            DataDB::Sensor(SensorDataDB::CPU(cpu)) => {
                assert_eq!(cpu.total_energy, Some(EnergyUj::from_u64(0)));
                assert_eq!(cpu.pp0_energy, Some(EnergyUj::from_u64(0)));
                assert_eq!(cpu.pp1_energy, Some(EnergyUj::from_u64(0)));
                assert_eq!(cpu.dram_energy, Some(EnergyUj::from_u64(0)));
                assert_eq!(cpu.usage_percent, Some(0.0));
            }
            _ => panic!("CPUData::zero() returned wrong SensorData variant"),
        }

        // GPU
        match GPUDataDB::zero() {
            DataDB::Sensor(SensorDataDB::GPU(gpu)) => {
                assert_eq!(gpu.total_energy, Some(EnergyUj::from_u64(0)));
                assert_eq!(gpu.usage_percent, Some(0.0));
                assert_eq!(gpu.vram_usage_percent, Some(0.0));
            }
            _ => panic!("GPUData::zero() returned wrong SensorData variant"),
        }

        // RAM
        match RamDataDB::zero() {
            DataDB::Sensor(SensorDataDB::Ram(ram)) => {
                assert_eq!(ram.total_energy, Some(EnergyUj::from_u64(0)));
                assert_eq!(ram.usage_percent, Some(0.0));
            }
            _ => panic!("RamData::zero() returned wrong SensorData variant"),
        }

        // Disk
        match DiskDataDB::zero() {
            DataDB::Sensor(SensorDataDB::Disk(disk)) => {
                assert_eq!(disk.total_energy, Some(EnergyUj::from_u64(0)));
                assert_eq!(disk.read_bytes, Byte::from(0));
                assert_eq!(disk.written_bytes, Byte::from(0));
            }
            _ => panic!("DiskData::zero() returned wrong SensorData variant"),
        }

        // Network
        match NetworkDataDB::zero() {
            DataDB::Sensor(SensorDataDB::Network(net)) => {
                assert_eq!(net.total_energy, Some(EnergyUj::from_u64(0)));
                assert_eq!(net.downloaded_bytes, Byte::from(0));
                assert_eq!(net.uploaded_bytes, Byte::from(0));
            }
            _ => panic!("NetworkData::zero() returned wrong SensorData variant"),
        }

        // Total
        match TotalDataDB::zero() {
            DataDB::Total(total) => {
                assert_eq!(total.total_energy, EnergyUj::from_u64(0));
            }
            _ => panic!("TotalData::zero() returned wrong SensorData variant"),
        }

        // Process
        match ProcessDataDB::zero() {
            DataDB::Process(vec) => {
                assert!(vec.is_empty());
            }
            _ => panic!("ProcessData::zero() returned wrong SensorData variant"),
        }
    }
}
