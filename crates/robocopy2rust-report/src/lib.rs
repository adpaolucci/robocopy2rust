pub mod html;
pub mod json;
pub mod markdown;

use anyhow::{Context, Result};
use chrono::Utc;
use robocopy2rust_core::decode_exit_code;
use robocopy2rust_runner::{ParsedLineOutcome, RobocopyOutputParser, RunResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportFormat {
    Html,
    Json,
    Markdown,
    All,
}

impl ReportFormat {
    pub fn parse(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "html" | "htm" => Ok(Self::Html),
            "json" => Ok(Self::Json),
            "md" | "markdown" => Ok(Self::Markdown),
            "all" | "bundle" => Ok(Self::All),
            other => anyhow::bail!("Unsupported report format '{other}'. Use html, json, markdown, or all."),
        }
    }
}

pub fn write_report(result: &RunResult, format: ReportFormat, output: &Path) -> Result<Vec<PathBuf>> {
    let mut written = Vec::new();
    match format {
        ReportFormat::Html => {
            ensure_parent(output)?;
            std::fs::write(output, html::render_html(result))?;
            written.push(output.to_path_buf());
        }
        ReportFormat::Json => {
            ensure_parent(output)?;
            std::fs::write(output, json::render_json(result)?)?;
            written.push(output.to_path_buf());
        }
        ReportFormat::Markdown => {
            ensure_parent(output)?;
            std::fs::write(output, markdown::render_markdown(result))?;
            written.push(output.to_path_buf());
        }
        ReportFormat::All => {
            let dir = if output.extension().is_some() {
                output.parent().unwrap_or_else(|| Path::new("."))
            } else {
                output
            };
            std::fs::create_dir_all(dir)?;
            let stem = output
                .file_stem()
                .and_then(|s| s.to_str())
                .filter(|_| output.extension().is_some())
                .unwrap_or("robocopy2rust_report");

            let html_path = dir.join(format!("{stem}.html"));
            let json_path = dir.join(format!("{stem}.json"));
            let md_path = dir.join(format!("{stem}.md"));

            std::fs::write(&html_path, html::render_html(result))?;
            std::fs::write(&json_path, json::render_json(result)?)?;
            std::fs::write(&md_path, markdown::render_markdown(result))?;
            written.push(html_path);
            written.push(json_path);
            written.push(md_path);
        }
    }
    Ok(written)
}

fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Parses an existing Robocopy log file (UTF-8 or UTF-16LE `/UNILOG`) into a structured [`RunResult`]
/// ready for HTML, JSON, or Markdown reporting.
pub fn parse_log_file_to_run_result(path: &Path) -> Result<RunResult> {
    let bytes = std::fs::read(path).with_context(|| format!("Failed to read log file {}", path.display()))?;
    let text = decode_log_bytes(&bytes)?;

    let mut parser = RobocopyOutputParser::new();
    let mut source = String::new();
    let mut destination = String::new();
    let mut options_line = String::new();
    let mut file_actions = Vec::new();
    let mut errors = Vec::new();
    let mut log_lines = Vec::new();
    let mut hasher = Sha256::new();

    for raw in text.lines() {
        let line = raw.trim_end_matches('\r').to_string();
        hasher.update(line.as_bytes());
        hasher.update(b"\n");
        log_lines.push(line.clone());

        let trimmed = line.trim();
        if let Some(v) = trimmed.strip_prefix("Source :") {
            source = v.trim().to_string();
        } else if let Some(v) = trimmed.strip_prefix("Dest :") {
            destination = v.trim().to_string();
        } else if let Some(v) = trimmed.strip_prefix("Options :") {
            options_line = v.trim().to_string();
        }

        match parser.feed_line(&line) {
            ParsedLineOutcome::FileAction(rec) => file_actions.push(rec),
            ParsedLineOutcome::Error(e) => errors.push(e),
            _ => {}
        }
    }

    let dry_run = options_line
        .split_whitespace()
        .any(|tok| tok.eq_ignore_ascii_case("/L"));

    // Derive Robocopy-equivalent exit bitmask from summary counters
    let mut code = 0i32;
    if parser.summary.files.copied > 0 {
        code |= 1;
    }
    if parser.summary.files.extras > 0 || parser.summary.dirs.extras > 0 {
        code |= 2;
    }
    if parser.summary.files.mismatch > 0 || parser.summary.dirs.mismatch > 0 {
        code |= 4;
    }
    if parser.summary.files.failed > 0 || parser.summary.dirs.failed > 0 || !errors.is_empty() {
        code |= 8;
    }

    let now = Utc::now().to_rfc3339();
    let started_at = parser.summary.started.clone().unwrap_or_else(|| now.clone());
    let ended_at = parser.summary.ended.clone().unwrap_or(now);

    let command_line = if options_line.is_empty() {
        format!("robocopy (imported from {})", path.display())
    } else {
        format!("robocopy \"{}\" \"{}\" {}", source, destination, options_line)
    };

    Ok(RunResult {
        job_name: path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Imported Robocopy Log")
            .to_string(),
        source,
        destination,
        dry_run,
        command_line,
        args: Vec::new(),
        started_at,
        ended_at,
        duration_secs: 0.0,
        exit_code: code,
        exit_info: decode_exit_code(code),
        cancelled: false,
        summary: parser.summary,
        file_actions,
        errors,
        log_lines,
        log_sha256: format!("{:x}", hasher.finalize()),
    })
}

fn decode_log_bytes(bytes: &[u8]) -> Result<String> {
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let mut units = Vec::with_capacity((bytes.len() - 2) / 2);
        for chunk in bytes[2..].as_chunks::<2>().0 {
            units.push(u16::from_le_bytes(*chunk));
        }
        return Ok(String::from_utf16(&units)?);
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let mut units = Vec::with_capacity((bytes.len() - 2) / 2);
        for chunk in bytes[2..].as_chunks::<2>().0 {
            units.push(u16::from_be_bytes(*chunk));
        }
        return Ok(String::from_utf16(&units)?);
    }
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Ok(String::from_utf8_lossy(&bytes[3..]).into_owned());
    }
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn parses_legacy_log_and_writes_air_gapped_report_bundle() {
        let tmp = TempDir::new().unwrap();
        let log_path = tmp.path().join("sample.log");
        let log_content = "\
-------------------------------------------------------------------------------
   ROBOCOPY     ::     Robust File Copy for Windows
-------------------------------------------------------------------------------

  Started : Thursday, November 19, 2026 1:00:00 AM
   Source : C:\\Data\\Src\\
     Dest : D:\\Data\\Dst\\

    Files : *.*

  Options : *.* /L /E /BYTES /R:3 /W:5

------------------------------------------------------------------------------

\t                   1\tC:\\Data\\Src\\
\t    New File  \t\t     4096\tC:\\Data\\Src\\report.pdf

------------------------------------------------------------------------------

               Total    Copied   Skipped  Mismatch    FAILED    Extras
    Dirs :         1         0         1         0         0         0
   Files :         1         1         0         0         0         0
   Bytes :      4096      4096         0         0         0         0
   Speed :            40,960 Bytes/sec.
   Ended : Thursday, November 19, 2026 1:00:01 AM
";
        std::fs::write(&log_path, log_content).unwrap();

        let parsed = parse_log_file_to_run_result(&log_path).unwrap();
        assert!(parsed.dry_run);
        assert_eq!(parsed.exit_code, 1);
        assert_eq!(parsed.summary.files.copied, 1);
        assert_eq!(parsed.summary.bytes.copied, 4096);

        let bundle_dir = tmp.path().join("bundle");
        let written = write_report(&parsed, ReportFormat::All, &bundle_dir).unwrap();
        assert_eq!(written.len(), 3);

        let html = std::fs::read_to_string(&written[0]).unwrap();
        assert!(!html.contains("https://cdn"));
        assert!(!html.contains("<script src="));
        assert!(html.contains("report.pdf"));
        assert!(html.contains(&parsed.log_sha256));
    }
}
