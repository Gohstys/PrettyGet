<p align="center">
  <img src="PrettyGet_logo.png" alt="PrettyGet" width="180" />
</p>

<h1 align="center">PrettyGet ⬇</h1>

<p align="center">A <b>pretty</b> desktop interface for <code>winget</code> — winget, but nice to look at.</p>

PrettyGet wraps Windows' built-in package manager (`winget`) in a clean, dark-themed desktop app. Instead of remembering `winget` flags and reading raw console tables, you get a proper UI: see what's outdated, search and install new software, watch the live output of any operation, and schedule silent updates to run on their own — all from one window.

It's built with **Tauri** (a Rust backend driving a lightweight web frontend), so it's small and fast, with no browser engine bundled separately.

![theme](https://img.shields.io/badge/theme-dark%20minimal-6d7cff) ![stack](https://img.shields.io/badge/stack-Tauri%20%2B%20Rust-9a6dff) ![price](https://img.shields.io/badge/price-free-45d483)

## Features

- **Updates** — lists every package with a newer version available (name, Id, current → available version). Update everything, a selection, or one at a time, with a live progress bar and a button to abort mid-update.
- **Explore** — search for new packages (`winget search`) and install them, or browse what's already installed (`winget list`) and uninstall it. **Advanced options**: pick the source (all / winget / Microsoft Store) and install mode (silent or interactive); preferences are remembered.
- **Live log** — winget's output streams line by line during any operation, with a progress bar and an Abort button while something is running.
- **Schedule** — create tasks in the Windows Task Scheduler (daily / weekly / monthly at a given time) that run `winget upgrade --all` silently. Test or delete them from the app.
- **Language** — English by default, with an EN/ES switcher in the top bar (remembered).
- **Run as administrator** — a button that relaunches the app elevated once, so you don't get a UAC prompt for every single package during an update.
- **Advanced** — State Sync (export/import your package list as JSON/YAML), Remote Deploy (run winget on remote machines over WinRM), IaC Generator (turn a selection into PowerShell or Ansible) and Silent Daemon (a background Windows service for silent scheduled updates). All free, no license needed.
- **Donate** — a tab with links to GitHub Sponsors and Buy Me a Coffee for anyone who wants to support the project. Entirely optional, never required for any feature.

**100% free, no ads.** If you find it useful, you can support development from the **Donate** tab (GitHub Sponsors / Buy Me a Coffee) — never required to use any feature.

## Requirements

1. **Windows 10/11** with **winget** installed (comes with *App Installer* from the Microsoft Store).
2. **Rust** → https://rustup.rs
3. **Node.js** (only for the Tauri CLI) → https://nodejs.org
4. System dependencies for Tauri on Windows: **Microsoft Edge WebView2** (already included on Windows 11) and **Visual Studio Build Tools** with the "Desktop development with C++" workload.

## Running in development

```bash
cd PrettyGet
npm install          # installs the Tauri CLI
npm run dev          # builds Rust + opens the window (tauri dev)
```

The first Rust build takes a little while; later ones are fast. Note: dev builds show a console window alongside the app (useful for Rust log output) — the release build below does not.

## Building the installer

```bash
npm run build        # produces a .msi and a .exe (NSIS) in src-tauri/target/release/bundle/
```

## Structure

```
PrettyGet/
├─ package.json            # dev/build scripts (Tauri CLI)
├─ src/                    # frontend (web)
│  ├─ index.html
│  ├─ styles.css
│  └─ main.js
└─ src-tauri/              # backend (Rust)
   ├─ Cargo.toml
   ├─ build.rs
   ├─ tauri.conf.json
   ├─ capabilities/        # Tauri v2 permission grants (ACL)
   ├─ icons/               # app icons
   └─ src/
      ├─ main.rs           # command registration
      ├─ winget.rs         # list/search/install/uninstall/upgrade + table parser
      └─ schedule.rs       # scheduled tasks via schtasks
```

## How it works internally

- **Listing**: runs `winget upgrade --include-unknown` and parses the table **independently of system language**: it locates columns by position (from the dashed separator line), strips ANSI codes, and filters out footer lines. Works with Windows in English, Spanish, etc. Output is normalized from CRLF before parsing, since winget's line endings would otherwise throw off the column detection.
- **Updating**: launches winget with `--silent` (without `--disable-interactivity`, so progress keeps flowing) and streams the output live: lines ending in `\r` (progress) are emitted as transient and replace the previous one — a percentage is parsed out and drives a real progress bar; lines ending in `\n` are committed. Events `winget-out` / `winget-done`. Stdin is explicitly closed so winget can never hang waiting on input that will never come. The running process's PID is tracked so it can be cancelled mid-operation.
- **Administrator**: `relaunch_as_admin` restarts the executable with `Start-Process -Verb RunAs` (a single UAC prompt); `is_elevated` checks the current state.
- **Schedule**: uses `schtasks /Create` with a `PrettyGet_` prefix so the app can list and delete only its own tasks.
- Winget/schtasks console windows are hidden with the `CREATE_NO_WINDOW` flag.

## Ideas for later

- Per-package progress and a system tray icon.
- Notifications when new updates are available.
- Export/import the package list.

## Notes

- If an update needs administrator rights, winget will prompt for elevation (UAC).
- The parser covers winget's standard output; if Microsoft changes the format, adjust `parse_upgrades` in `winget.rs` (it has unit tests: `cargo test`).

## License

[GPLv3](LICENSE). You're free to use, modify, and redistribute PrettyGet, including commercially, but any modified version you distribute must stay free software under the same license.
