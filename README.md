<h1 align="center">dumbctl</h1>

<p align="center">
  <img src="https://img.shields.io/badge/rust-1.70+-orange.svg" alt="Rust 1.70+">
  <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT">
  <img src="https://img.shields.io/badge/platform-linux-lightgrey.svg" alt="Platform: Linux">
</p>

A Terminal User Interface (TUI) for HDD/SSD health monitoring, benchmarking, and SMART data analysis.

---

## ✨ Features

| Feature | Description |
|---------|-------------|
| 🖴 **Disk Detection** | Automatically detects and lists all connected disks (HDD/SSD) |
| 📊 **SMART Data** | View health status, temperature, power-on hours, reallocated sectors |
| ⚡ **Benchmarking** | Sequential read/write speed tests |
| 📈 **Sector Health** | Visual progress bar showing good vs bad sectors |
| 📁 **Export** | Export data to JSON or CSV format |
| ⌨️ **Keyboard-Driven** | Fully navigable via keyboard |

---

## 📋 Requirements

- **Linux** (primary support)
- **[smartmontools](https://www.smartmontools.org/)** - for SMART data access
- **Root access** - required for SMART data

---

## 🚀 Installation

### Build from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/dumbctl.git
cd dumbctl

# Build
cargo build --release

# Run (requires root for SMART data)
sudo ./target/release/dumbctl
```

---

## 🎮 Usage

```bash
sudo ./dumbctl
```

### Keyboard Controls

| Key | Action |
|:---:|--------|
| `Tab` | Switch between tabs |
| `↑` `↓` | Navigate lists |
| `Enter` | Select disk |
| `r` | Refresh SMART data |
| `s` | Start benchmark |
| `q` | Quit |

### Tabs Overview

| Tab | Purpose |
|-----|---------|
| **Disk List** | View all detected disks |
| **SMART Details** | Detailed SMART attributes |
| **Sectors** | Sector health overview with progress bar |
| **Benchmark** | Run disk speed tests |
| **Export** | Export data to JSON/CSV |

---

## 🔧 Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release
```

The binary will be at `target/release/dumbctl`.

---

## ⚠️ Permissions

SMART data requires root access. Run with:

```bash
sudo dumbctl
```

---

## 📄 License

MIT License - see [LICENSE](LICENSE) file.

---

## 🤝 Contributing

Contributions are welcome! Please open an issue or submit a pull request.
