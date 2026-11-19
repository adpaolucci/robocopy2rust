use regex::Regex;
use robocopy2rust_core::format::parse_robocopy_size_token;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLineKind {
    Header,
    FileAction,
    Progress,
    Error,
    Retry,
    Summary,
    Normal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileActionKind {
    NewFile,
    Newer,
    Older,
    Changed,
    Same,
    ExtraFile,
    ExtraDir,
    NewDir,
    Tweaked,
    Modified,
    Lonely,
    Mismatch,
    Failed,
    DirEntry,
    Other,
}

impl FileActionKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::NewFile => "New File",
            Self::Newer => "Newer",
            Self::Older => "Older",
            Self::Changed => "Changed",
            Self::Same => "Same",
            Self::ExtraFile => "*EXTRA File",
            Self::ExtraDir => "*EXTRA Dir",
            Self::NewDir => "New Dir",
            Self::Tweaked => "Tweaked",
            Self::Modified => "Modified",
            Self::Lonely => "Lonely",
            Self::Mismatch => "Mismatch",
            Self::Failed => "FAILED",
            Self::DirEntry => "Dir",
            Self::Other => "Other",
        }
    }

    pub fn is_copied_or_new(self) -> bool {
        matches!(
            self,
            Self::NewFile | Self::Newer | Self::Changed | Self::NewDir | Self::Tweaked | Self::Modified
        )
    }

    pub fn is_extra(self) -> bool {
        matches!(self, Self::ExtraFile | Self::ExtraDir)
    }

    pub fn is_skipped(self) -> bool {
        matches!(self, Self::Older | Self::Same | Self::Lonely)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileActionRecord {
    pub kind: FileActionKind,
    pub size_bytes: Option<u64>,
    pub timestamp: Option<String>,
    pub path: String,
    pub raw_line: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountRow {
    pub total: u64,
    pub copied: u64,
    pub skipped: u64,
    pub mismatch: u64,
    pub failed: u64,
    pub extras: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RunSummary {
    pub dirs: CountRow,
    pub files: CountRow,
    pub bytes: CountRow,
    pub speed_bytes_per_sec: Option<f64>,
    pub speed_mb_per_min: Option<f64>,
    pub started: Option<String>,
    pub ended: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ParsedLineOutcome {
    Header,
    FileAction(FileActionRecord),
    Progress(f32),
    Error(String),
    Retry(String),
    SummaryUpdated,
    Normal,
}

#[derive(Debug, Default)]
pub struct RobocopyOutputParser {
    pub summary: RunSummary,
    pub current_dir: Option<String>,
    pub current_file: Option<String>,
    pub in_header: bool,
    pub in_summary: bool,
}

fn progress_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*(\d+(?:\.\d+)?)%\s*$").expect("valid progress regex"))
}

fn error_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)^\s*(?:\d{4}/\d{2}/\d{2}\s+\d{2}:\d{2}:\d{2}\s+)?ERROR\s+\d+\s*\(").expect("valid error regex")
    })
}

fn retry_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^\s*Waiting\s+\d+\s+seconds\.\.\.\s*Retrying").expect("valid retry regex"))
}

fn timestamp_tail_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^(.+?)\s+(\d{4}/\d{2}/\d{2}\s+\d{2}:\d{2}:\d{2})$").expect("valid ts regex"))
}

impl RobocopyOutputParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed_line(&mut self, raw_line: &str) -> ParsedLineOutcome {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            return ParsedLineOutcome::Normal;
        }

        // 1. Live percentage lines (emitted via \r)
        if let Some(caps) = progress_regex().captures(trimmed)
            && let Ok(pct) = caps[1].parse::<f32>()
        {
            return ParsedLineOutcome::Progress(pct.clamp(0.0, 100.0));
        }

        // 2. Errors & Retries
        if error_regex().is_match(trimmed) {
            return ParsedLineOutcome::Error(trimmed.to_string());
        }
        if retry_regex().is_match(trimmed) {
            return ParsedLineOutcome::Retry(trimmed.to_string());
        }

        // 3. Banner / Header / Summary separators
        if trimmed.starts_with("-------------------------------------------------------------------------------")
            || trimmed.starts_with("ROBOCOPY     ::")
        {
            self.in_header = true;
            return ParsedLineOutcome::Header;
        }
        if trimmed.starts_with("------------------------------------------------------------------------------") {
            self.in_header = false;
            return ParsedLineOutcome::Header;
        }

        if let Some(rest) = trimmed.strip_prefix("Started :") {
            self.summary.started = Some(rest.trim().to_string());
            return ParsedLineOutcome::Header;
        }
        if let Some(rest) = trimmed.strip_prefix("Ended :") {
            self.summary.ended = Some(rest.trim().to_string());
            return ParsedLineOutcome::SummaryUpdated;
        }
        if trimmed.starts_with("Source :")
            || trimmed.starts_with("Dest :")
            || trimmed.starts_with("Files :")
            || trimmed.starts_with("Exc Files :")
            || trimmed.starts_with("Exc Dirs :")
            || trimmed.starts_with("Options :")
        {
            // Disambiguate header `Files : *.*` vs summary `Files : 1 1 0 0 0 0`
            if let Some(rest) = trimmed.strip_prefix("Files :")
                && let Some(row) = parse_count_row(rest.trim(), false)
            {
                self.summary.files = row;
                self.in_summary = true;
                return ParsedLineOutcome::SummaryUpdated;
            }
            return ParsedLineOutcome::Header;
        }

        if trimmed.starts_with("Total") && trimmed.contains("Copied") && trimmed.contains("Extras") {
            self.in_summary = true;
            return ParsedLineOutcome::SummaryUpdated;
        }

        if let Some(rest) = trimmed.strip_prefix("Dirs :")
            && let Some(row) = parse_count_row(rest, false)
        {
            self.summary.dirs = row;
            self.in_summary = true;
            return ParsedLineOutcome::SummaryUpdated;
        }
        if let Some(rest) = trimmed.strip_prefix("Files :")
            && let Some(row) = parse_count_row(rest, false)
        {
            self.summary.files = row;
            self.in_summary = true;
            return ParsedLineOutcome::SummaryUpdated;
        }
        if let Some(rest) = trimmed.strip_prefix("Bytes :")
            && let Some(row) = parse_count_row(rest, true)
        {
            self.summary.bytes = row;
            self.in_summary = true;
            return ParsedLineOutcome::SummaryUpdated;
        }
        if let Some(rest) = trimmed.strip_prefix("Times :") {
            let _ = rest;
            self.in_summary = true;
            return ParsedLineOutcome::SummaryUpdated;
        }
        if let Some(rest) = trimmed.strip_prefix("Speed :") {
            let r = rest.trim();
            if let Some(num_str) = r.strip_suffix("Bytes/sec.") {
                let cleaned: String = num_str.trim().chars().filter(|c| *c != ',').collect();
                if let Ok(v) = cleaned.parse::<f64>() {
                    self.summary.speed_bytes_per_sec = Some(v);
                }
            } else if let Some(num_str) = r.strip_suffix("MegaBytes/min.") {
                let cleaned: String = num_str.trim().chars().filter(|c| *c != ',').collect();
                if let Ok(v) = cleaned.parse::<f64>() {
                    self.summary.speed_mb_per_min = Some(v);
                }
            }
            return ParsedLineOutcome::SummaryUpdated;
        }

        // 4. Tab-delimited File / Directory classification lines
        if raw_line.starts_with('\t')
            && let Some(record) = self.parse_tab_record(raw_line)
        {
            if !matches!(
                record.kind,
                FileActionKind::DirEntry | FileActionKind::ExtraDir | FileActionKind::NewDir
            ) {
                self.current_file = Some(record.path.clone());
            } else {
                self.current_dir = Some(record.path.clone());
            }
            return ParsedLineOutcome::FileAction(record);
        }

        ParsedLineOutcome::Normal
    }

    fn parse_tab_record(&self, raw_line: &str) -> Option<FileActionRecord> {
        let fields: Vec<&str> = raw_line.split('\t').map(str::trim).filter(|s| !s.is_empty()).collect();
        if fields.is_empty() {
            return None;
        }

        // Pattern A: `<class>\t<size [ts]>\t<path>` or `<class+count>\t<dir>`
        // Pattern B: `<count>\t<dir_path>` (directory header row)
        if fields.len() == 2 && fields[0].chars().all(|c| c.is_ascii_digit() || c == '-') {
            let path = fields[1].to_string();
            return Some(FileActionRecord {
                kind: FileActionKind::DirEntry,
                size_bytes: None,
                timestamp: None,
                path,
                raw_line: raw_line.trim().to_string(),
            });
        }

        // Look for known directory markers like `*EXTRA Dir        -1\t<path>` or `New Dir          3\t<path>`
        let first_upper = fields[0].to_ascii_uppercase();
        if first_upper.starts_with("*EXTRA DIR") {
            let path = fields.last().copied().unwrap_or("").to_string();
            return Some(FileActionRecord {
                kind: FileActionKind::ExtraDir,
                size_bytes: None,
                timestamp: None,
                path,
                raw_line: raw_line.trim().to_string(),
            });
        }
        if first_upper.starts_with("NEW DIR") {
            let path = fields.last().copied().unwrap_or("").to_string();
            return Some(FileActionRecord {
                kind: FileActionKind::NewDir,
                size_bytes: None,
                timestamp: None,
                path,
                raw_line: raw_line.trim().to_string(),
            });
        }

        if fields.len() >= 2 {
            let class_tok = fields[0];
            let kind = classify_action(class_tok);
            let mut size_bytes = None;
            let mut timestamp = None;
            let mut path = fields.last().copied().unwrap_or("").to_string();

            if fields.len() >= 3 {
                let middle = fields[1];
                let (size_str, ts_opt) = if let Some(caps) = timestamp_tail_regex().captures(middle) {
                    (caps[1].trim().to_string(), Some(caps[2].trim().to_string()))
                } else {
                    (middle.to_string(), None)
                };
                timestamp = ts_opt;
                size_bytes = parse_robocopy_size_token(&size_str);
            } else if fields.len() == 2 {
                // With /NS (No Size), fields are [class, path]; or with /NC (No Class), [size, path]
                if let Some(sz) = parse_robocopy_size_token(fields[0]) {
                    size_bytes = Some(sz);
                    return Some(FileActionRecord {
                        kind: FileActionKind::Other,
                        size_bytes,
                        timestamp: None,
                        path: fields[1].to_string(),
                        raw_line: raw_line.trim().to_string(),
                    });
                }
            }

            // If path isn't rooted and we know current_dir, leave path readable or attach if relative
            if path.is_empty() {
                path = raw_line.trim().to_string();
            } else if !path.contains('\\')
                && !path.contains('/')
                && let Some(dir) = &self.current_dir
            {
                let base = dir.trim_end_matches('\\');
                path = format!("{base}\\{path}");
            }

            return Some(FileActionRecord {
                kind,
                size_bytes,
                timestamp,
                path,
                raw_line: raw_line.trim().to_string(),
            });
        }

        None
    }
}

fn classify_action(raw: &str) -> FileActionKind {
    match raw.trim().to_ascii_lowercase().as_str() {
        "new file" => FileActionKind::NewFile,
        "newer" => FileActionKind::Newer,
        "older" => FileActionKind::Older,
        "changed" => FileActionKind::Changed,
        "same" => FileActionKind::Same,
        "*extra file" => FileActionKind::ExtraFile,
        "*extra dir" => FileActionKind::ExtraDir,
        "new dir" => FileActionKind::NewDir,
        "tweaked" => FileActionKind::Tweaked,
        "modified" => FileActionKind::Modified,
        "lonely" => FileActionKind::Lonely,
        "mismatch" => FileActionKind::Mismatch,
        "failed" => FileActionKind::Failed,
        _ => FileActionKind::Other,
    }
}

/// Parses 6 numeric columns from a Robocopy summary row (`Total Copied Skipped Mismatch FAILED Extras`),
/// joining space-separated unit suffixes (`4.88 k`) when `allow_units` is true.
fn parse_count_row(rest: &str, allow_units: bool) -> Option<CountRow> {
    let raw_tokens: Vec<&str> = rest.split_whitespace().collect();
    if raw_tokens.is_empty() {
        return None;
    }

    let mut values = Vec::with_capacity(6);
    let mut i = 0;
    while i < raw_tokens.len() {
        let tok = raw_tokens[i];
        if allow_units
            && i + 1 < raw_tokens.len()
            && matches!(
                raw_tokens[i + 1].to_ascii_lowercase().as_str(),
                "k" | "m" | "g" | "t" | "kb" | "mb" | "gb" | "tb"
            )
        {
            let combined = format!("{tok}{}", raw_tokens[i + 1]);
            values.push(parse_robocopy_size_token(&combined)?);
            i += 2;
        } else {
            values.push(parse_robocopy_size_token(tok)?);
            i += 1;
        }
    }

    if values.len() != 6 {
        return None;
    }

    Some(CountRow {
        total: values[0],
        copied: values[1],
        skipped: values[2],
        mismatch: values[3],
        failed: values[4],
        extras: values[5],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_robocopy_lines_and_suffixed_byte_summary() {
        let mut p = RobocopyOutputParser::new();

        let dir_line = "\t                   1\tC:\\Src\\";
        match p.feed_line(dir_line) {
            ParsedLineOutcome::FileAction(r) => assert_eq!(r.kind, FileActionKind::DirEntry),
            other => panic!("unexpected: {other:?}"),
        }

        let file_line = "\t    New File  \t\t      28 2026/11/19 06:25:24\tC:\\Src\\test.txt";
        match p.feed_line(file_line) {
            ParsedLineOutcome::FileAction(r) => {
                assert_eq!(r.kind, FileActionKind::NewFile);
                assert_eq!(r.size_bytes, Some(28));
                assert_eq!(r.timestamp.as_deref(), Some("2026/11/19 06:25:24"));
                assert_eq!(r.path, r"C:\Src\test.txt");
            }
            other => panic!("unexpected: {other:?}"),
        }

        let extra_line = "\t*EXTRA File \t\t     512\told.bak";
        match p.feed_line(extra_line) {
            ParsedLineOutcome::FileAction(r) => {
                assert_eq!(r.kind, FileActionKind::ExtraFile);
                assert_eq!(r.size_bytes, Some(512));
                assert_eq!(r.path, r"C:\Src\old.bak");
            }
            other => panic!("unexpected: {other:?}"),
        }

        match p.feed_line(" 45.5% ") {
            ParsedLineOutcome::Progress(pct) => assert!((pct - 45.5).abs() < 0.01),
            other => panic!("unexpected: {other:?}"),
        }

        p.feed_line("    Dirs :         2         1         1         0         0         0");
        p.feed_line("   Files :         5         2         2         0         0         1");
        p.feed_line("   Bytes :    10.00 m    2.50 m    7.50 m         0         0       512");
        p.feed_line("   Speed :            1,234,567 Bytes/sec.");
        p.feed_line("   Speed :               70.642 MegaBytes/min.");

        assert_eq!(p.summary.dirs.total, 2);
        assert_eq!(p.summary.files.copied, 2);
        assert_eq!(p.summary.files.extras, 1);
        assert_eq!(p.summary.bytes.total, 10 * 1024 * 1024);
        assert_eq!(p.summary.bytes.copied, 2_621_440);
        assert_eq!(p.summary.speed_bytes_per_sec, Some(1_234_567.0));
        assert_eq!(p.summary.speed_mb_per_min, Some(70.642));
    }
}
