<p align="center">
  <img src="src-tauri/icons/icon.png" alt="Blur Launcher" width="200">
</p>

<h1 align="center">Blur Launcher</h1>

<p align="center">
  <img src="https://img.shields.io/badge/Tauri-2-24C8DB?style=flat-square&logo=tauri&logoColor=white" alt="Tauri">
  <img src="https://img.shields.io/badge/Rust-2021-DEA584?style=flat-square&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/React-19-61DAFB?style=flat-square&logo=react&logoColor=black" alt="React">
  <img src="https://img.shields.io/badge/TypeScript-5-3178C6?style=flat-square&logo=typescript&logoColor=white" alt="TypeScript">
  <img src="https://img.shields.io/badge/Vite-6-646CFF?style=flat-square&logo=vite&logoColor=white" alt="Vite">
  <img src="https://img.shields.io/badge/Tailwind-4-06B6D4?style=flat-square&logo=tailwindcss&logoColor=white" alt="Tailwind CSS">
  <img src="https://img.shields.io/badge/License-MIT-00C853?style=flat-square" alt="License">
</p>

<p align="center">Automate Blur setup — checks online fix files, configures firewall rules, enables network discovery, disables virtual adapters, and restores everything on exit.</p>

> Once you run Blur Launcher, all files needed for online play are already copied to your game directory. For additional setup steps and instructions, see the [AMAX Emulator — How to Play](https://amax-emu.com/how_to_play) guide.

## Project Structure

```
Blur Launcher/
├── src-tauri/            # Rust backend (Tauri 2)
│   ├── src/
│   │   ├── main.rs         # IPC commands, tray, window management
│   │   ├── network.rs      # WMI adapter disable/enable
│   │   ├── discovering.rs  # Network discovery & SMB config
│   │   └── updater.rs      # Auto-update via GitHub releases
│   ├── build.rs            # Admin manifest embedding
│   └── tauri.conf.json     # App config, window, bundle settings
├── src/                  # React frontend
│   ├── components/       # UI components (BlurControl, modals)
│   ├── hooks/            # State machine, event listeners
│   ├── services/         # IPC adapter service
│   └── types/            # TypeScript types
├── .github/workflows/    # Windows release workflow
└── package.json
```

## Quick Start

```bash
# Install dependencies
npm install

# Run in dev mode
npm run tauri dev

# Build for production
npm run tauri build
```

## How It Works

1. **Online Fix** — Checks bundled fix files against the game directory, copies any that are missing
2. **Firewall** — Verifies inbound/outbound firewall rules for Blur.exe, creates them if absent; enables ICMPv4 rules for LAN discovery
3. **Network Discovery** — Enables SMB signing and encryption via registry, starts Function Discovery and UPnP services
4. **Isolate & Launch** — Disables virtual adapters (VMware, VirtualBox, Hyper-V, VPN, etc.), launches the game, and restores all adapters automatically on exit

The app runs as a system tray application — compact, always on top, and positioned at the bottom-right corner.

## License

MIT License - see [LICENSE](LICENSE) for details.
