# Contributing to `robocopy2rust`

Thank you for your interest in contributing to `robocopy2rust`!

## Code Architecture

The workspace is organized into 7 modular crates:
- `crates/robocopy2rust-core`: Job schema, Windows path normalization & `CommandLineToArgvW`-safe quoting, bidirectional CLI & `.rcj` parsers, validation rules, `0–16` exit-code bitmask decoder, built-in presets, and persistent profile/history store.
- `crates/robocopy2rust-runner`: Environment detection, real-time `\r`/`\n` streaming Robocopy executor (`CREATE_NO_WINDOW` on Windows), stateful output & summary parser, and SHA-256 log hashing.
- `crates/robocopy2rust-report`: Self-contained offline HTML, JSON, and Markdown audit report generators plus legacy Robocopy log parser.
- `crates/robocopy2rust-mcp`: Model Context Protocol (MCP) JSON-RPC 2.0 `stdio` server (`robocopy2rust-mcp`).
- `crates/robocopy2rust-cli`: Unified command-line interface binary (`robocopy2rust`) that launches `robocopy2rust-win32` (Windows default) or `robocopy2rust-iced` (Linux/macOS or `--iced`) when invoked with no arguments.
- `crates/robocopy2rust-win32`: Native zero-GPU Win32 desktop console built on `winsafe` (`robocopy2rust-win32`).
- `crates/robocopy2rust-iced`: Cross-platform retained-mode desktop console built on `iced` (`robocopy2rust-iced`).

## Development Workflow

1. Fork and clone the repository.
2. Ensure you have a recent stable Rust toolchain installed (`rustup update stable`).
3. Create a feature branch (`git checkout -b feature/my-enhancement`).
4. Run checks, linter, and tests:
   ```bash
   cargo check --workspace --all-targets
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
5. Commit with clear, descriptive commit messages and open a Pull Request on GitHub.

## Guidelines
- **Safety First**: Any path that executes `/MIR`, `/PURGE`, `/MOVE`, or `/MOV` must provide Dry-Run (`/L`) previewing and explicit destructive confirmation in Win32 GUI, Iced GUI, CLI, and MCP.
- **Zero WebView / Zero External Runtime**: All GUI rendering, parsing, validation, and report generation remain 100% pure Rust (wrapping OS `robocopy.exe` strictly for copy execution).
- **Quoting Integrity**: Never allow trailing backslashes inside Windows double quotes to escape closing quotes (`C:\` -> `"C:\\"`, non-root `C:\foo\` -> `"C:\foo"`).
- **Logging**: Use `tracing` macros writing diagnostics to `stderr` so CLI `stdout` and MCP `stdio` streams remain clean.
