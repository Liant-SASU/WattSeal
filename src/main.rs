#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::SocketAddr;

use bpaf::{OptionParser, Parser, construct, long};
use collector::{CollectorApp, ConsumptionUnit, MQTTInfo};

/// Configuration options for the application.
#[derive(Debug, Clone)]
struct Options {
    capture_interval: u64,
    mqtt_id: Option<String>,
    mqtt_addr: Option<SocketAddr>,
    mqtt_unit: Option<ConsumptionUnit>,
    #[cfg(target_os = "windows")]
    install_cpu_driver: bool,
    #[cfg(target_os = "windows")]
    uninstall_cpu_driver: bool,
}

/// Returns options parser to run
fn options() -> OptionParser<Options> {
    let capture_interval = long("intervals")
        .short('i')
        .help("Interval in seconds between each data capture.")
        .argument::<u64>("SECS")
        .fallback(1)
        .display_fallback();

    let mqtt_id = long("mqtt-id")
        .help("Identifier used as the root of MQTT topics (e.g. my-machine/cpu, my-machine/ram). Requires --mqtt-addr to be set. Defaults to \"wattseal_collector\".")
        .argument::<String>("ID")
        .optional();

    let mqtt_addr = long("mqtt-addr")
        .help("Specify MQTT broker address to send sensors data.")
        .argument::<SocketAddr>("ADDRESS")
        .optional();

    let mqtt_unit = long("mqtt-unit")
        .help(
            "Unit for collector consumption values published via MQTT. \
       One of: uj (microjoules), wh (watt-hours). \
       If omitted, returns raw collector values with their original unit (uj).",
        )
        .argument::<String>("UNIT")
        .parse(|s| match s.as_str() {
            "uj" => Ok(ConsumptionUnit::UJoul),
            "wh" => Ok(ConsumptionUnit::WattHour),
            other => Err(format!("Unknown returns unit '{}' for MQTT: expected uj or wh.", other)),
        })
        .optional();

    let description = "WattSeal - Per-app power monitoring tool";

    #[cfg(target_os = "windows")]
    {
        let install_cpu_driver = long("install-cpu-driver")
            .help("Install the Windows CPU MSR driver (requires Administrator privileges).")
            .switch();

        let uninstall_cpu_driver = long("uninstall-cpu-driver")
            .help("Uninstall the Windows CPU MSR driver (requires Administrator privileges).")
            .switch();

        return construct!(Options {
            capture_interval,
            mqtt_id,
            mqtt_addr,
            mqtt_unit,
            install_cpu_driver,
            uninstall_cpu_driver,
        })
        .to_options()
        .descr(description);
    }

    #[cfg(not(target_os = "windows"))]
    {
        return construct!(Options {
            capture_interval,
            mqtt_id,
            mqtt_addr,
            mqtt_unit,
        })
        .to_options()
        .descr(description);
    }
}

/// Initializes the collector
fn start_collector(capture_interval: u64, mqtt_infos: Option<MQTTInfo>) -> Result<CollectorApp, String> {
    let mut app =
        CollectorApp::new(capture_interval, mqtt_infos).map_err(|e| format!("Failed to create CollectorApp: {e}"))?;
    app.initialize()
        .map_err(|e| format!("Failed to initialize CollectorApp: {e}"))?;
    Ok(app)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(e) = common::set_current_dir_to_exe_dir() {
        common::clog!("⚠ Failed to set working directory to executable directory: {}", e);
    }

    let options = options().run();

    #[cfg(target_os = "windows")]
    {
        if options.install_cpu_driver {
            collector::sensors::cpu::windows_cpu::install();
            return;
        }

        if options.uninstall_cpu_driver {
            collector::sensors::cpu::windows_cpu::uninstall();
            return;
        }
    }

    if options.mqtt_addr.is_none() && options.mqtt_id.is_some() {
        let msg = format!("An MQTT broker address must be entered in order to specify the collector's MQTT topic.");
        common::clog!("✗ {msg}");
        return;
    }

    let mqtt_infos = if let Some(mqtt_addr) = options.mqtt_addr {
        let id = options.mqtt_id.unwrap_or("wattseal_collector".to_string());
        let unit = options.mqtt_unit;
        Some(MQTTInfo::new(&id, &mqtt_addr, unit))
    } else {
        None
    };

    match start_collector(options.capture_interval, mqtt_infos) {
        Ok(mut app) => app.run().await,
        Err(e) => common::clog!("✗ {e}"),
    }
}
