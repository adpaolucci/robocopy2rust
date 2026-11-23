# Security Policy

## Supported Versions

| Version | Supported |
| :--- | :--- |
| `0.1.x` | :white_check_mark: |

Command-line safety, path quoting integrity, and explicit guards around destructive Robocopy flags (`/MIR`, `/PURGE`, `/MOVE`, `/MOV`) are top priorities for `robocopy2rust`.

If you discover a security vulnerability, command-injection/quoting edge case, or destructive guard bypass:
1. **Do not disclose it publicly** via GitHub Issues.
2. Submit a report privately via GitHub Private Vulnerability Reporting under the Security tab:
   `https://github.com/adpaolucci/robocopy2rust/security/advisories/new`
3. You will receive an acknowledgment within 48 hours.
4. A security patch and advisory will be coordinated and released promptly.
