# Security Policy

## Supported Versions

Make sure to always use the latest version of Colhidor, as it includes the most up-to-date security patches and improvements.

## Scope

Colhidor may request elevated privileges to install a Windows CPU driver or to access Linux RAPL counters.

---

## Scaphandre RAPL Driver (Windows)

On Windows, Colhidor uses the **Scaphandre RAPL driver**, a minimal signed kernel-mode driver, to read CPU Model Specific Registers (MSRs) required for RAPL energy counters. This replaces the previous generic MSR driver and reduces the exposed surface to the read-only operations Colhidor needs.

### Why it exists

Reading CPU energy registers on Windows requires Ring-0 (kernel) access. The Scaphandre driver provides read-only MSR access focused on RAPL counters, avoiding the generic read/write capabilities of legacy drivers.

### Security implications

Kernel drivers run at the highest privilege level on the system. While the Scaphandre driver is minimal and read-only, it is still privileged code. Colhidor installs it once and then accesses it from user mode; normal operation does not require elevated privileges.

### Installing and removing the driver

To install the driver:

1. Run `Colhidor --install-cpu-driver` **as Administrator**.

To remove the driver:

1. Run `Colhidor --uninstall-cpu-driver` **as Administrator**.
