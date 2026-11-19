# `robocopy2rust` — Pure-Rust GUI Wrapper, Orchestrator, CLI & MCP Server for Microsoft Robocopy

Part of the **GUAP** Ecosystem (`guap.ca`) • [**Project Hub & Web Documentation: https://robocopy2rust.guap.ca**](https://robocopy2rust.guap.ca)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust: 2024](https://img.shields.io/badge/Rust-2024%20(1.88%2B)-orange.svg)](Cargo.toml)
[![Architecture: 6 Crates](https://img.shields.io/badge/Workspace-6%20Crates-emerald.svg)](Cargo.toml)
[![GUI: Win32 MMC](https://img.shields.io/badge/GUI-Win32%20MMC%20(egui)-cyan.svg)](crates/robocopy2rust-gui)
[![MCP: stdio JSON-RPC](https://img.shields.io/badge/MCP-stdio%20JSON--RPC%202.0-purple.svg)](crates/robocopy2rust-mcp)

**`robocopy2rust`** is a 100% pure-Rust native **Win32/MMC GUI wrapper**, bidirectional command/job orchestrator, live telemetry runner, offline audit reporter, CLI, and **Model Context Protocol (MCP)** server for Microsoft's **Robocopy** (`robocopy.exe`). Zero Electron, zero WebView2, zero Node.js, and zero `.NET` runtime overhead.

---

## Comparison: `robocopy2rust` vs. Legacy Workflows

| Capability | `robocopy2rust` (Pure Rust) | Raw `robocopy.exe` | Legacy GUIs (RoboGUI / ChoEazyCopy) | Ad-hoc PowerShell Scripts |
| :--- | :--- | :--- | :--- | :--- |
| **Desktop UI Footprint** | **Native Win32/MMC (`eframe`/`egui`), 0 MB WebView** | Console only | .NET Framework / WinForms | None |
| **Windows 10/11 Switches** | **Full coverage (`/IoMaxSize`, `/IoRate`, `/LFSM`, `/COMPRESS`, `/NOCLONE`, `/SPARSE`)** | Manual flag recall | Partial / dated switches | Manual string concatenation |
| **Quoting & Trailing `\` Safety** | **Automatic path normalization + `CommandLineToArgvW` backslash doubling** | Easy to break `"C:\dir\"` | Basic quoting | Error-prone string escaping |
| **Bidirectional `.rcj` & CLI Parser** | **Paste any command or import UTF-8/UTF-16 `.rcj` back into GUI** | Separate `/JOB:` syntax | Export only or partial | None |
| **Destructive Safety Guards** | **Mandatory Dry-Run (`/L`) workflow & explicit `/MIR` / `/PURGE` / `/MOVE` guard** | Immediate execution | Basic prompt | Manual `-WhatIf` |
| **Live Telemetry & Reports** | **Real-time `\r` progress, parsed file diff table, air-gapped HTML/JSON/MD** | Plain text scroll | Raw text box | Plain log file |
| **AI Agent Integration** | **Native `stdio` MCP Server (`robocopy2rust-mcp`, 9 tools)** | None | None | None |

---

## Workspace Architecture

```text
robocopy2rust/
├── Cargo.toml                         # Workspace root manifest (6 modular crates)
├── mcp.json                           # Standard MCP server client configuration
├── crates/
│   ├── robocopy2rust-core/            # Job schema, Windows path quoting, CLI & .rcj parser, validator, 0-16 bitmask decoder, presets, profile store
│   ├── robocopy2rust-runner/          # Environment detector, CREATE_NO_WINDOW streaming executor, \r progress & summary parser, SHA-256 log hash
│   ├── robocopy2rust-report/          # Self-contained offline HTML, JSON, and Markdown audit reports + legacy Robocopy log importer
│   ├── robocopy2rust-mcp/             # Model Context Protocol (MCP) JSON-RPC 2.0 stdio server & standalone `robocopy2rust-mcp` binary
│   ├── robocopy2rust-cli/             # Unified CLI (`robocopy2rust`): presets, build, parse, decode, dry-run, run, profile, report, env, mcp, gui
│   └── robocopy2rust-gui/             # Classic Win32/MMC pure-Rust desktop management console (`robocopy2rust-gui`)
└── .github/workflows/                 # Cross-platform CI (`fmt`, `clippy`, `test`, `release`) & GitHub Release packaging
```

---

## Desktop Console (`robocopy2rust-gui`)

Run `robocopy2rust-gui` (or `robocopy2rust` with no arguments) to open the Classic Win32/MMC management workstation:

- **Console Tree (Left Pane)**: Fast switching across 8 views, 8 built-in presets (preserving your active Source & Destination paths), and persistent saved profiles stored atomically in `%APPDATA%\robocopy2rust\profiles.json`.
- **Job Designer & Real-Time Validator**: Instant diagnostics for incompatible switches (`/MT` + `/IPG`, `/MT` + `/EFSRAW`, `/MT` + `/LFSM`), destination-inside-source recursion risks, invalid `/RH:` hours, and destructive flags.
- **4 Complete Switch Tabs**:
  - **Copy & Transfer**: `/S`, `/E`, `/LEV`, `/MIR`, `/PURGE`, `/MOV`, `/MOVE`, `/Z`, `/B`, `/ZB`, `/J`, `/EFSRAW`, `/MT:1..128`, `/COPY:[DATXSOU]`, `/DCOPY:[DATEX]`, `/SECFIX`, `/TIMFIX`, `/A+`, `/A-`, `/CREATE`, `/FAT`, `/256`, `/SJ`, `/SL`, `/NOOFFLOAD`, `/COMPRESS`, `/SPARSE:Y|N`, `/NOCLONE`.
  - **File Selection**: `/XF`, `/XD`, `/XJ`, `/XJD`, `/XJF`, `/XC`, `/XN`, `/XO`, `/XX`, `/XL`, `/IS`, `/IT`, `/IM`, `/A`, `/M`, `/IA`, `/XA`, `/MAX`, `/MIN`, `/MAXAGE`, `/MINAGE`, `/MAXLAD`, `/MINLAD`, `/FFT`, `/DST`.
  - **Throttle & Retry**: `/R:n`, `/W:n`, `/REG`, `/TBD`, `/LFSM[:n[KMG]]`, `/IoMaxSize`, `/IoRate`, `/Threshold`, `/IPG`, `/MON`, `/MOT`, `/RH`, `/PF`.
  - **Logging & Jobs**: `/L`, `/X`, `/V`, `/TS`, `/FP`, `/BYTES`, `/NS`, `/NC`, `/NFL`, `/NDL`, `/NP`, `/ETA`, `/LOG`, `/LOG+`, `/UNILOG`, `/UNILOG+`, `/TEE`, `/NJH`, `/NJS`, `/UNICODE`, `/JOB`, `/SAVE`, `/QUIT`, `/NOSD`, `/NODD`.
- **Live Console & Dry-Run Diff**: Streaming `%` progress bar, live `Dirs`/`Files`/`Bytes`/Speed counters, searchable File Action Diff table (`New File`, `Newer`, `*EXTRA File`, `FAILED`), raw monospace stream, and one-click HTML/JSON/Markdown report export.
- **Batch Queue & Task Scheduler**: Ordered multi-job execution queue with `Stop on Error (exit >= 8)` and ready-to-run `schtasks /Create` command generation.
- **Script & Job Exports**: Export `.rcj` job files, `.bat` batch wrappers, and `.ps1` PowerShell scripts.

---

## Command-Line Interface (`robocopy2rust`)

```powershell
# Launch the Desktop GUI
robocopy2rust

# List all 8 built-in presets
robocopy2rust presets

# Build and validate a command, plus export .rcj / .bat / .ps1 / schtasks
robocopy2rust build --source "C:\Data Src" --dest "D:\Backup" --preset safe-backup --threads 16 --export-ps1 backup.ps1 --export-schtasks

# Parse any raw Robocopy command or .rcj job file into structured JSON
robocopy2rust parse "robocopy \"C:\Data Src\" \"D:\Backup\" *.docx /MIR /MT:16 /R:3 /W:5 /XD .git /XF *.tmp" --json
robocopy2rust parse job.rcj --rcj --json

# Decode any Robocopy exit code (0..=16)
robocopy2rust decode 3

# Run a safe /L Dry-Run simulation and write an air-gapped HTML report
robocopy2rust dry-run --source "C:\Src" --dest "D:\Dst" --report preview.html

# Execute a destructive mirror job (requires explicit --yes)
robocopy2rust run --source "C:\Src" --dest "D:\Dst" --mirror --yes --report audit.html

# Convert an existing Robocopy log file into HTML, JSON, and Markdown reports
robocopy2rust report --log-file nightly.log --output ./audit_bundle --format all

# Start the stdio MCP server
robocopy2rust mcp
```

---

## Model Context Protocol (`robocopy2rust-mcp`)

Add `mcp.json` to your MCP-compatible client (`command: "robocopy2rust"`, `args: ["mcp"]`) to expose 9 structured tools:

1. `robocopy2rust_environment` — Host OS & `robocopy.exe` probe
2. `robocopy2rust_presets` — Built-in templates
3. `robocopy2rust_build_command` — Validated command, `.rcj`, `.bat`, `.ps1`, and `schtasks` generator
4. `robocopy2rust_parse_command` — Command-line / `.rcj` parser + validation diagnostics
5. `robocopy2rust_decode_exit_code` — Bitmask `0–16` decoder
6. `robocopy2rust_profiles` — Persistent profile CRUD (`list`, `get`, `save`, `delete`)
7. `robocopy2rust_dry_run` — Safe non-destructive `/L` simulation
8. `robocopy2rust_run` — Live execution with strict `confirm_destructive: true` refusal guard for `/MIR`, `/PURGE`, `/MOVE`, and `/MOV`
9. `robocopy2rust_report` — Offline HTML / JSON / Markdown audit generator

---

## Robocopy Exit Code Bitmask (`0–16`)

Unlike traditional tools where `1` means failure, Robocopy returns a **bitmask** where **exit codes `0` through `7` indicate success** and **`8` or higher indicate failure**:

| Exit Code | Bitmask | Outcome | Meaning |
| :---: | :--- | :--- | :--- |
| **`0`** | `0x00` | Success | No files copied, no failures, source and destination already synchronized. |
| **`1`** | `0x01` | Success | One or more files were copied successfully. |
| **`2`** | `0x02` | Success | Extra files or directories were detected in the destination. |
| **`3`** | `0x01 \| 0x02` | Success | Files were copied and extra files/directories were detected. |
| **`4`** | `0x04` | Success | Mismatched files or directories were detected. |
| **`5–7`** | `0x01..0x04` | Success | Combinations of copied, extra, and mismatched files. |
| **`8`** | `0x08` | **Error** | One or more files or directories failed to copy after retries (`/R:n`). |
| **`16`** | `0x10` | **Fatal** | Serious/fatal error (invalid parameters, missing path, or access denied). |

---

## Building from Source

Requires Rust `1.88+` (2024 edition).

```powershell
git clone https://github.com/adpaolucci/robocopy2rust.git
cd robocopy2rust
cargo test --workspace
cargo build --workspace --release
```

Compiled binaries are emitted to `target\release\`:
- `robocopy2rust.exe` (Unified CLI & GUI launcher)
- `robocopy2rust-gui.exe` (Native Classic Win32/MMC Desktop Console)
- `robocopy2rust-mcp.exe` (Standalone `stdio` MCP server)

## License

Licensed under the [MIT License](LICENSE).
