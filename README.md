# Colhidor

Colhidor is a tool for collecting consumption data and sending it over the MQTT protocol.

This one can be used on Windows, macOS, and Linux.

---

## What can Colhidor measure?

| Component                | How it's measured                                      |
| --------------------------| --------------------------------------------------------|
| **CPU (Intel / AMD)**    | Direct hardware energy counters (RAPL) — very accurate |
| **GPU (NVIDIA)**         | NVML vendor API — very accurate                        |
| **GPU (AMD, Windows)**   | ADLX vendor API — very accurate                        |
| **GPU (Intel, Windows)** | PDH performance counters                               |
| **RAM**                  | Estimated from memory usage                            |
| **Disk**                 | Estimated from read/write activity                     |
| **Network**              | Estimated from data throughput                         |
| **Per-process**          | CPU + GPU + I/O breakdown per process                  |
| **TCP Connections**      | TCP connections throughput                             |

---

## Platform Support

|                            | Windows          | Linux           | macOS     |
| ----------------------------| :----------------:| :---------------:| :---------:|
| Full application           | ✅                | ✅               | ✅         |
| CPU energy counters        | ✅                | ✅               | Estimated |
| NVIDIA GPU                 | ✅                | ✅               | ❌         |
| AMD GPU                    | ✅                | ❌               | ❌         |
| Intel GPU                  | ✅                | ❌               | ❌         |
| Other sensors (usage, I/O) | ✅                | ✅               | ✅         |
| Auto admin elevation       | ✅ UAC (one time) | Manual (`sudo`) | Manual    |

<details>
<summary><strong>Support without admin privileges</strong></summary>

|                            | Windows                  | Linux     | macOS     |
| ----------------------------| :------------------------:| :---------:| :---------:|
| Full application           | ✅                        | ✅         | ✅         |
| CPU energy counters        | ✅ (after driver install) | Estimated | Estimated |
| NVIDIA GPU                 | ✅                        | ✅         | ❌         |
| AMD GPU                    | ✅                        | ❌         | ❌         |
| Intel GPU                  | ✅                        | ❌         | ❌         |
| Other sensors (usage, I/O) | ✅                        | ✅         | ✅         |

</details>

---

<br>

## Architecture Overview

Colhidor is a Rust workspace made up of three crates:

```
colhidor/               ← Root binary
  ├── collector/        ← Background sensor polling, energy estimation and data sending
  ├── common/           ← Shared types, SQLite layer, utilities
  ├── mqtt/             ← MQTT data sender
```

---

## Prerequisites

- **Rust** stable toolchain (version pinned in [`rust-toolchain.toml`](rust-toolchain.toml)).
- On linux, install the build deps for the tray icon, not needed at runtime but required to build the Linux version:

  ```bash
  sudo apt install libgtk-3-dev pkg-config libxkbcommon-dev libwayland-dev
  ```

---

## Building from Source

Clone the repository:
```bash
git clone https://github.com/Liant-SASU/Colhidor.git
```

```bash
cd colhidor
```

Debug build and run:
```bash
cargo run
```

Release build:
```bash
cargo build --release
```

> ⚠️ **Elevated privileges are required** only to install the Windows CPU MSR driver once.
> Run with administrator rights on Windows (you will be prompted to elevate for driver setup), or use `sudo` on Linux for RAPL access.

---

## Project Layout

| Path          | What it does                                                                            |
| ---------------| -----------------------------------------------------------------------------------------|
| `src/main.rs` | Entry point: admin elevation, tray icon, collector thread, UI subprocess                |
| `collector/`  | All sensor implementations (CPU, GPU, RAM, disk, network, per-process, tcp-connections) |
| `common/`     | Shared types (`Event`, `SensorData`, …), utilities                                      |
| `mqtt/`       | MQTT data sender                                                                        |
---

## Code Style & Quality

The project enforces the formatting and linting rules defined in `rustfmt.toml`. Compliance is checked in CI. You can run the following command locally to ensure your code meets the project's style guidelines before pushing:

```bash
cargo +nightly fmt
```

> The `.vscode/settings.json` and `.zed/settings.json` are configured to format on save, so if you're using VS Code or Zed your code will be formatted automatically when you save a file.

---

## License

Colhidor is licensed under [GPL-3.0](LICENSE). See the [LICENSE](LICENSE) file for details.

## See also

[WattSeal](https://github.com/Daminoup88/WattSeal): The original tool by Damien PHILIPPE. 