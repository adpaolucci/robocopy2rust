use robocopy2rust_core::format::{format_bytes, format_duration};
use robocopy2rust_runner::RunResult;

pub fn render_markdown(result: &RunResult) -> String {
    let mut out = String::new();
    let mode = if result.dry_run {
        "Dry-Run (/L)"
    } else {
        "Live Execution"
    };
    let status = if result.cancelled {
        "Cancelled"
    } else if result.exit_info.is_success {
        "Success"
    } else {
        "Failed"
    };

    out.push_str(&format!("# Robocopy2Rust Audit Report: {}\n\n", result.job_name));
    out.push_str(&format!(
        "- **Status**: {status} (`{}`)\n",
        result.exit_info.short_label
    ));
    out.push_str(&format!("- **Mode**: {mode}\n"));
    out.push_str(&format!("- **Source**: `{}`\n", result.source));
    out.push_str(&format!("- **Destination**: `{}`\n", result.destination));
    out.push_str(&format!("- **Started**: `{}`\n", result.started_at));
    out.push_str(&format!("- **Ended**: `{}`\n", result.ended_at));
    out.push_str(&format!(
        "- **Duration**: `{}`\n",
        format_duration(result.duration_secs)
    ));
    out.push_str(&format!("- **Log SHA-256**: `{}`\n\n", result.log_sha256));

    out.push_str("## Command Line\n\n```bat\n");
    out.push_str(&result.command_line);
    out.push_str("\n```\n\n");

    out.push_str("## Exit Code Breakdown\n\n");
    for bit in &result.exit_info.bit_descriptions {
        out.push_str(&format!("- {bit}\n"));
    }
    out.push('\n');

    out.push_str("## Summary Matrix\n\n");
    out.push_str("| Category | Total | Copied | Skipped | Mismatch | FAILED | Extras |\n");
    out.push_str("| :--- | ---: | ---: | ---: | ---: | ---: | ---: |\n");
    let d = &result.summary.dirs;
    out.push_str(&format!(
        "| Dirs | {} | {} | {} | {} | {} | {} |\n",
        d.total, d.copied, d.skipped, d.mismatch, d.failed, d.extras
    ));
    let f = &result.summary.files;
    out.push_str(&format!(
        "| Files | {} | {} | {} | {} | {} | {} |\n",
        f.total, f.copied, f.skipped, f.mismatch, f.failed, f.extras
    ));
    let b = &result.summary.bytes;
    out.push_str(&format!(
        "| Bytes | {} | {} | {} | {} | {} | {} |\n\n",
        format_bytes(b.total),
        format_bytes(b.copied),
        format_bytes(b.skipped),
        format_bytes(b.mismatch),
        format_bytes(b.failed),
        format_bytes(b.extras),
    ));

    if !result.errors.is_empty() {
        out.push_str("## Errors\n\n");
        for err in &result.errors {
            out.push_str(&format!("- `{err}`\n"));
        }
        out.push('\n');
    }

    if !result.file_actions.is_empty() {
        out.push_str("## File & Directory Actions\n\n");
        out.push_str("| Class | Size | Timestamp | Path |\n");
        out.push_str("| :--- | ---: | :--- | :--- |\n");
        for rec in &result.file_actions {
            let sz = rec.size_bytes.map(format_bytes).unwrap_or_else(|| "-".to_string());
            let ts = rec.timestamp.as_deref().unwrap_or("-");
            out.push_str(&format!(
                "| {} | {} | {} | `{}` |\n",
                rec.kind.label(),
                sz,
                ts,
                rec.path
            ));
        }
        out.push('\n');
    }

    out
}
