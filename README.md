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
- **Advanced** — State Sync (export/import your package list as JSON/YAML), Remote Deploy (run winget on remote machines over WinRM), IaC Generator (turn a selection into PowerShell or Ansible) and Silent Daemon (a background Windows service for silent scheduled updates — its binary ships inside the installer, so there's nothing extra to download). All free, no license needed.
- **Donate** — a tab with links to GitHub Sponsors and Buy Me a Coffee for anyone who wants to support the project. Entirely optional, never required for any feature.

**100% free, no ads.** If you find it useful, you can support development from the **Donate** tab (GitHub Sponsors / Buy Me a Coffee) — never required to use any feature.

## A note on Windows security warnings

PrettyGet isn't code-signed yet, so Windows may flag the installer/exe the first time you run it — either a SmartScreen warning ("Windows protected your PC") or, on Windows 11 with **Smart App Control** enabled, an outright block with no "run anyway" option. This isn't malware; Windows just doesn't yet recognize the publisher, since that requires a paid or approved code-signing certificate.

- **SmartScreen warning**: click **More info** → **Run anyway**.
- **Smart App Control block**: it can only be turned off while still in "Evaluation" mode (Settings → Privacy & security → Windows Security → App & browser control); once it's switched to "On", disabling it requires reinstalling Windows. Building PrettyGet yourself from source (see below) avoids this entirely.

See the code signing policy below — this note will be removed once releases are signed.

## Downloads

Get the latest installer from the [Releases page](https://github.com/Gohstys/PrettyGet/releases):

- **`PrettyGet_x.y.z_x64-setup.exe`** — the normal installer, for most people.
- **`PrettyGet_x.y.z_x64_en-US.msi`** — for silent/managed deployments (`msiexec /quiet`).

Both are built by GitHub Actions from the matching tag, and both include everything —
there is nothing extra to download.

## Code signing policy

Free code signing provided by [SignPath.io](https://signpath.io/), certificate by
[SignPath Foundation](https://signpath.org/).

*(Application to the SignPath Foundation open source program is pending; releases are
not signed yet.)*

- **Committers and reviewers**: [Gohsty](https://github.com/Gohstys) — sole maintainer
- **Approvers**: [Gohsty](https://github.com/Gohstys) — sole maintainer

**Privacy policy**: this program will not transfer any information to other networked
systems unless specifically requested by the user or the person installing or operating
it. PrettyGet has no analytics, no accounts and no telemetry; it drives the `winget`
tool and the Windows Task Scheduler that are already part of Windows, and the package
data it shows comes from whatever sources your own `winget` is configured to use.

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
# Build the daemon first — the installer bundles it as a resource, so it has to exist:
cargo build --release --manifest-path tools/prettyget-daemon/Cargo.toml
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

Copyright (C) 2026 Gohsty. Licensed under [GPLv3](LICENSE). You're free to use, modify, and redistribute PrettyGet, including commercially, but any modified version you distribute must stay free software under the same license, and any notice of authorship must be kept intact.
