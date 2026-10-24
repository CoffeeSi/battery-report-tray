# Battery Report Tray

A lightweight, zero-dependency Windows system tray application written in pure Rust for real-time laptop battery telemetry.

The application runs quietly in the background without launching a console window, providing instant access to hardware-accurate battery status, current capacity in milliwatt-hours (mWh), and power draw/charge rates in Watts via the system tray tooltip.

---

## Features

- **Pure Win32 API**: Built directly on native Windows APIs (`windows-rs`) without heavy runtimes or UI frameworks (no Electron, WebView, or Qt). Binary size is under 500 KB and consumes negligible system memory.
- **Low-Level Hardware Query**: Collects live power metrics directly via kernel device drivers (`IOCTL_BATTERY_QUERY_STATUS`, `IOCTL_BATTERY_QUERY_INFORMATION`).
- **Real-Time Telemetry**:
    - Connection state (AC Connected / Discharging).
    - Current battery capacity in **mWh**.
    - Real-time charge / discharge rate in Watts (**W**).
- **Headless GUI Mode**: Fully detached from terminal windows using `#![windows_subsystem = "windows"]`.
- **Self-Contained Executable**: Embedded application icon inside the `.exe` binary using custom Win32 resource linking.

---

## Tech Stack

- **Language**: [Rust](https://www.rust-lang.org/) (Edition 2024)
- **Windows Bindings**: `windows` crate (v0.62)
- **Platform**: Windows 10 / 11 (x86_64)

---

## Building and Running

### Prerequisites

- [Rust toolchain](https://rustup.rs/) (`rustc`, `cargo`).
- MinGW-w64 toolchain (with `windres.exe` available in `PATH`).

### Build

```powershell
# Clone the repository
git clone https://github.com/CoffeeSi/battery_report_tray.git
cd battery_report_tray

# Build optimized release binary
cargo build --release
```
