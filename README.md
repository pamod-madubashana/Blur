# Blur // LAN Launcher

A small Tauri desktop app that wraps the "disable non-Wi-Fi adapters → launch Blur →
restore adapters" workflow in a proper UI, instead of a raw PowerShell window.

It does exactly what your original `.ps1` did:
1. Disables every network adapter except `Wi-Fi` that is currently `Up`.
2. Waits 5 seconds.
3. Launches `Blur.exe` and waits for it to close.
4. Re-enables every adapter it disabled.

The game path is remembered between runs (stored in your app-data folder), and every
step is streamed live into an on-screen console instead of a console window.

## Project layout

```
blur-lan-launcher/
├─ src/                     # frontend (vanilla HTML/CSS/JS, no build step)
│  ├─ index.html
│  ├─ styles.css
│  └─ main.js
└─ src-tauri/                # Rust backend
   ├─ Cargo.toml
   ├─ tauri.conf.json
   ├─ build.rs
   ├─ icons/                 # placeholder app icons — swap for your own art
   └─ src/
      ├─ main.rs             # commands, config persistence, launch sequence
      └─ network.rs          # PowerShell-backed adapter enable/disable
```

## Prerequisites (Windows, since this drives `Disable-NetAdapter`)

- [Rust](https://rustup.rs) (stable toolchain)
- [Node.js](https://nodejs.org) 18+ (only needed for the Tauri CLI)
- Tauri v2 CLI: `cargo install tauri-cli --version "^2"`
- Microsoft Visual Studio C++ Build Tools (the Tauri prerequisite installer will
  prompt you if missing — see https://v2.tauri.app/start/prerequisites/)

## Run it in dev mode

```powershell
cd blur-lan-launcher
cargo tauri dev
```

This opens the app in a live-reloading window. Because `Disable-NetAdapter` /
`Enable-NetAdapter` need administrator rights, **launch your terminal as
Administrator** before running this, or the disable/enable steps will fail with an
"Access is denied" error (the app will still run and log the failure — it just
won't touch the adapters).

## Build a distributable .exe / installer

```powershell
cd blur-lan-launcher
cargo tauri build
```

The installer (NSIS `.exe`) lands in
`src-tauri/target/release/bundle/nsis/`. Because the app calls network-adapter
cmdlets, tell users to **right-click → Run as administrator** the first time (or
set "Run this program as an administrator" on the shortcut's Compatibility tab so
they don't have to do it every time).

## Notes / things you may want to tweak

- **Icons**: `src-tauri/icons/` currently has placeholder art (a simple asphalt
  square with a cyan ring). Regenerate real ones with
  `cargo tauri icon path/to/your-artwork.png`.
- **Admin elevation**: right now the app doesn't self-elevate. If you want it to
  prompt for UAC automatically on launch, add a `manifest` requesting
  `requireAdministrator` via `tauri-plugin-window-state`/a custom `.exe.manifest`,
  or simply ship the shortcut pre-configured to run as admin.
- **Multiple non-Wi-Fi adapters**: the app disables *all* of them (matching your
  original script), then only re-enables whatever it finds in a `Disabled` state
  afterward — same behavior as the source script.
- **If the game crashes hard** (not just closed normally) `Wait-Process`/`child.wait()`
  still returns once the process exits, so adapters get restored either way.
- **Config file** lives at `%APPDATA%/com.blurlan.launcher/blur-lan-launcher.config.json`.
  Delete it if you want the app to ask for the `.exe` path again.

## UI overview

- **Gauge** in the top-left tracks the current stage (standby → disabling →
  settling → ignition → LAN mode → restore) with color shifting from amber
  (network changes) to cyan (game running) to green (restoring).
- **Session log** at the bottom mirrors every line the original script printed to
  the console, timestamped.
