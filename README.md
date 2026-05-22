# 🐧 lazywslc

A **lazydocker-inspired** TUI dashboard for managing WSL Linux containers via `wslc.exe`.

![Rust](https://img.shields.io/badge/Rust-stable-orange)
![Platform](https://img.shields.io/badge/platform-Windows-blue)

## Features

- **Three-panel layout** — Containers, Images, and Volumes each in their own section
- **Live stats** — CPU/memory sparkline graphs with auto-scaling
- **Combined Logs + Stats** — view both simultaneously for running containers
- **Mouse support** — click to select, scroll wheel to navigate
- **Context actions** — Space key popup menu with start/stop/kill/remove/prune
- **Keyboard-driven** — vim-style navigation (j/k), Tab to cycle sections
- **Auto-refresh** — data updates every second, stats every 2 seconds
- **VT sanitization** — handles container build output without screen corruption

## Install

### MSI installer (recommended)

Download `lazywslc-x.y.z-x86_64.msi` from the [Releases](https://github.com/crloewen/lazywslc/releases) page. Double-click to install — it adds `lazywslc` to your PATH and shows up in Add/Remove Programs.

### Download binary

Download `lazywslc.exe` from the [Releases](https://github.com/crloewen/lazywslc/releases) page and place it somewhere in your PATH.

### From source

```bash
cargo install --git https://github.com/crloewen/lazywslc
```

### Build locally

```bash
git clone https://github.com/crloewen/lazywslc
cd lazywslc
cargo build --release
# Binary at: target/release/lazywslc.exe
```

## Requirements

- Windows with [WSL](https://learn.microsoft.com/en-us/windows/wsl/) installed
- `wslc.exe` available in PATH (ships with recent Windows builds)

## Key Bindings

| Key | Action |
|-----|--------|
| `q` / `Ctrl-C` | Quit |
| `↑/k` `↓/j` | Navigate items |
| `Tab` | Cycle sections (Containers → Images → Volumes) |
| `1` `2` `3` | Jump to section |
| `←/→` | Switch detail tabs |
| `Space` | Open action menu |
| `s` / `S` / `K` | Start / Stop / Kill container |
| `x` | Remove selected item |
| `p` | Prune (stopped containers / dangling images) |
| `f` / `b` | Scroll Info tab forward / backward |
| `PageUp/Down` | Scroll logs |
| `/` | Filter list |
| `R` | Force refresh |
| `?` | Help overlay |

## License

MIT
