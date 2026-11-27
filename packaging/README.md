# Packaging Manifests (`WinGet` & `Homebrew`)

> **Templates, not published packages.** The `sha256` / `InstallerSha256` fields are zeroed placeholders because the release artifacts do not exist at commit time. Run `python scripts/packaging_hashes.py --repo robocopy2rust --version <ver> --assets-dir <dir>` at release time to fill in the real hashes.

This directory stores ready-to-submit package manager manifests matching published GitHub Releases for **`robocopy2rust`**.

## 1. Windows Package Manager (`winget`)

Validate and submit the YAML manifests in `packaging/winget/manifests/a/adpaolucci/robocopy2rust/<version>/`:

```powershell
# Local validation using winget CLI
winget validate --manifest .\packaging\winget\manifests\a\adpaolucci\robocopy2rust\0.1.0

# Or submit an automated PR to microsoft/winget-pkgs using wingetcreate
wingetcreate submit .\packaging\winget\manifests\a\adpaolucci\robocopy2rust\0.1.0
```

Installed portable command aliases: `robocopy2rust`, `robocopy2rust-win32`, `robocopy2rust-iced`, `robocopy2rust-mcp`.

## 2. Homebrew (`macOS` & `Linux`)

Copy `packaging/homebrew/robocopy2rust.rb` into `Formula/robocopy2rust.rb` of a tap repository (e.g., `adpaolucci/homebrew-tap`):

```bash
brew install adpaolucci/tap/robocopy2rust
# Or test locally directly from file:
brew install --formula ./packaging/homebrew/robocopy2rust.rb
brew test robocopy2rust
```
