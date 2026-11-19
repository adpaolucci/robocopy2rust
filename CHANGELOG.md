# Changelog

All notable changes to `robocopy2rust` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-19

### Added
- **Modular 7-Crate Pure-Rust Workspace (MSRV `1.88`)**: `robocopy2rust-core`, `robocopy2rust-runner`, `robocopy2rust-report`, `robocopy2rust-mcp`, `robocopy2rust-cli`, `robocopy2rust-win32`, and `robocopy2rust-iced`.
- **Complete Microsoft Robocopy Switch Model & Validation (`robocopy2rust-core`)**: Covers all Windows 10/11 copy, selection, I/O throttling, retry, low-free-space (`/LFSM`), logging, and job-file switches, bidirectional command-line parser, `.rcj` UTF-8/UTF-16LE importer & exporter, `.bat`/`.ps1`/`schtasks` export, `0–16` bitmask exit-code decoder, 8 built-in job presets, and persistent saved profile & run history storage.
- **Live Streaming Process Runner & Output Parser (`robocopy2rust-runner`)**: Executes `robocopy.exe` with `CREATE_NO_WINDOW` on Windows, streams `\r` progress percentages and file classification lines in real time, parses `Dirs`/`Files`/`Bytes`/`Speed` summaries, supports clean process cancellation, and computes SHA-256 log digests.
- **Air-Gapped Audit Reports (`robocopy2rust-report`)**: Generates standalone offline HTML (with interactive filtering and light/dark toggle), JSON, and Markdown audit reports from live runs or legacy Robocopy log files.
- **Model Context Protocol Server (`robocopy2rust-mcp` & `robocopy2rust mcp`)**: JSON-RPC 2.0 `stdio` MCP server exposing 9 tools with explicit `confirm_destructive` safety enforcement for `/MIR`, `/PURGE`, `/MOVE`, and `/MOV`.
- **Native Win32 Desktop Console (`robocopy2rust-win32`)**: Zero-GPU WinSafe `0.0.29` Win32 Common Controls GUI (ComCtl v6 + Per-Monitor DPI v2 manifest, Vista+ COM dialogs, `run_ui_thread` live console relay, `MessageBox` destructive confirmation) engineered for Windows desktops, elevated sessions, VMs, and Windows Server RDP.
- **Cross-Platform Iced Desktop Console (`robocopy2rust-iced`)**: Retained-mode Iced `0.14` (`wgpu` + `tiny-skia` software fallback) desktop console shipping across Linux, macOS, and Windows (`robocopy2rust gui --iced`), featuring Light/Dark mode, full Robocopy switch coverage, live diff/console, batch queue, run history, and modal overlays.
- **CLI Suite & Automated Test Coverage (`robocopy2rust`)**: 11 CLI subcommands with two-stage `Ctrl+C` cancellation and a 23-test workspace suite covering core quoting/parsers/presets, real `robocopy.exe` dry-run/sync/mirror integration, MCP safety gates, CLI parsing/overrides, and both GUIs.
- **CI, Automated Releases & Community Governance**: Cross-platform GitHub Actions CI (`ubuntu-latest`, `windows-latest`, `macos-latest`), multi-target release packaging (`LICENSE` + `NOTICE` + `SHA256SUMS.txt`), Dependabot configuration, and issue/PR templates.
- **Apache-2.0 Licensing & Attribution**: Licensed under `Apache-2.0` (`Cargo.toml`, `LICENSE`, `NOTICE`, `smithery.yaml`, `README.md`).
