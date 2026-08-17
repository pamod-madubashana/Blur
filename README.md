<p align="center">
  <img src="src-tauri/icons/icon.png" alt="Blur Launcher" width="200">
</p>

<h1 align="center">Blur Launcher</h1>

<p align="center">
  <img src="https://img.shields.io/badge/Tauri-2-24C8DB.svg" alt="Tauri">
  <img src="https://img.shields.io/badge/Rust-2021-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/React-19-61DAFB.svg" alt="React">
  <img src="https://img.shields.io/badge/TypeScript-5-3178c6.svg" alt="TypeScript">
  <img src="https://img.shields.io/badge/Vite-6-646CFF.svg" alt="Vite">
  <img src="https://img.shields.io/badge/Tailwind-4-06B6D4.svg" alt="Tailwind CSS">
  <img src="https://img.shields.io/badge/license-MIT-yellow.svg" alt="License">
</p>

<p align="center">Disable virtual network adapters, launch Blur in LAN mode, and restore adapters automatically when the game exits.</p>

## Project Structure

```
Blur Launcher/
├── src-tauri/            # Rust backend (Tauri 2)
│   ├── src/
│   │   ├── main.rs       # IPC commands, tray, window management
│   │   └── network.rs    # WMI adapter disable/enable
│   ├── build.rs          # Admin manifest embedding
│   └── tauri.conf.json   # App config, window, bundle settings
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

1. Detects virtual network adapters (VMware, VirtualBox, etc.) via WMI
2. Disables them to isolate the network for LAN play
3. Launches Blur with the game executable
4. Waits for the game process to exit
5. Restores all disabled adapters automatically

The app runs as a system tray application — compact, always on top, and positioned at the bottom-right corner.

## License

MIT License - see [LICENSE](LICENSE) for details.
