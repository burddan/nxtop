# nxtop

`nxtop` is a TUI system monitor built in Rust that goes far beyond `htop`. Instead of opening five different tools to understand what your machine is doing, `nxtop` puts processes, CPU, memory, disks, network, Bluetooth, and more into a single keyboard-driven interface.`


## Current Features

| Tab | What you get |
|---|---|
| **Processos** | Process list with CPU%, memory, threads, state. Filter by name/PID, tree view, kill |
| **Sistema** | Per-core CPU sparklines (60s history) + RAM gauge |
| **Rede** | Per-interface RX/TX rates and totals |

## Roadmap

- [ ] **Disco** — mount points, usage %, read/write I/O rates per device
- [ ] **Bluetooth** — paired devices, connection status, RSSI, connect/disconnect
- [ ] **USB** — connected devices, bus/port, vendor/product info
- [ ] **GPU** — utilization, VRAM usage, temperature (NVIDIA + AMD)
- [ ] **Temperatura** — CPU/GPU/NVMe sensors via `/sys/class/hwmon`
- [ ] **Logs** — tail of `journalctl` filtered per selected process
- [ ] **Network per process** — which process is eating your bandwidth
- [ ] **Containers** — Docker/Podman containers alongside system processes

## Keybindings

| Key | Action |isco — mount points, usage %, read/write I/O rates per device
Bluetooth — paired devices, connection status, RSSI, connect/disconnect
USB — connected devices, bus/port, vendor/product info
GPU — utilization, VRAM usage, temperature (NVIDIA + AMD)
|---|---|
| `Tab` / `Shift+Tab` | Next / previous tab |
| `j` / `k` or `↓` / `↑` | Navigate list |
| `/` | Filter processes by name or PID |
| `t` | Toggle tree view |
| `x` | Kill selected process (SIGKILL) |
| `r` | Force refresh |
| `q` | Quit |

## Install

```bash
git clone https://github.com/burddan/nxtop
cd nxtop
cargo build --release
./target/release/nxtop
```

**Requirements:** Linux, Rust 1.85+. No root needed for most features (kill and some sensors may require elevated privileges).

## Tech

- [`ratatui`](https://github.com/ratatui-org/ratatui) — TUI rendering
- [`crossterm`](https://github.com/crossterm-rs/crossterm) — terminal input/output
- Zero system dependencies — reads directly from `/proc`, `/sys`, and kernel interfaces

