use crate::command::normalize_robocopy_path;
use crate::format::is_valid_kmg_spec;
use crate::model::{LogFileMode, RobocopyJob};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationFinding {
    pub code: String,
    pub severity: ValidationSeverity,
    pub message: String,
    pub suggestion: Option<String>,
}

fn same_or_nested_path(src: &str, dst: &str) -> (bool, bool) {
    let s = normalize_robocopy_path(src).replace('/', "\\").to_ascii_lowercase();
    let d = normalize_robocopy_path(dst).replace('/', "\\").to_ascii_lowercase();
    if s.is_empty() || d.is_empty() {
        return (false, false);
    }
    if s == d {
        return (true, false);
    }
    let s_prefix = if s.ends_with('\\') { s } else { format!("{s}\\") };
    (false, d.starts_with(&s_prefix))
}

fn valid_run_hours(raw: &str) -> bool {
    let parts: Vec<&str> = raw.trim().split('-').collect();
    if parts.len() != 2 {
        return false;
    }
    parts.iter().all(|p| {
        if p.len() != 4 || !p.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
        let hh: u32 = p[0..2].parse().unwrap_or(99);
        let mm: u32 = p[2..4].parse().unwrap_or(99);
        hh < 24 && mm < 60
    })
}

pub fn validate_job(job: &RobocopyJob) -> Vec<ValidationFinding> {
    let mut findings = Vec::new();
    let has_job_file = job.job_files.load_job.as_deref().is_some_and(|s| !s.trim().is_empty());

    if job.source.trim().is_empty() && !job.job_files.no_source_dir && !has_job_file {
        findings.push(ValidationFinding {
            code: "MISSING_SOURCE".to_string(),
            severity: ValidationSeverity::Error,
            message: "Source directory is empty.".to_string(),
            suggestion: Some("Select a source folder or check /NOSD when loading from a /JOB file.".to_string()),
        });
    }

    if job.destination.trim().is_empty() && !job.job_files.no_dest_dir && !has_job_file {
        findings.push(ValidationFinding {
            code: "MISSING_DEST".to_string(),
            severity: ValidationSeverity::Error,
            message: "Destination directory is empty.".to_string(),
            suggestion: Some("Select a destination folder or check /NODD when using /QUIT.".to_string()),
        });
    }

    let (same, nested) = same_or_nested_path(&job.source, &job.destination);
    if same {
        findings.push(ValidationFinding {
            code: "SAME_SRC_DST".to_string(),
            severity: ValidationSeverity::Error,
            message: "Source and Destination resolve to the same directory.".to_string(),
            suggestion: Some("Choose a separate destination directory.".to_string()),
        });
    } else if nested
        && (job.copy.subdirectories_non_empty || job.copy.subdirectories_including_empty || job.copy.mirror)
    {
        findings.push(ValidationFinding {
            code: "RECURSIVE_DEST_IN_SRC".to_string(),
            severity: ValidationSeverity::Warning,
            message: "Destination is nested inside Source while recursive directory copying is enabled.".to_string(),
            suggestion: Some("Exclude the destination directory via /XD to avoid infinite recursion.".to_string()),
        });
    }

    if let Some(mt) = job.copy.multithreaded {
        if !(1..=128).contains(&mt) {
            findings.push(ValidationFinding {
                code: "MT_OUT_OF_RANGE".to_string(),
                severity: ValidationSeverity::Error,
                message: format!("/MT:{mt} is outside Robocopy's allowed range (1..=128)."),
                suggestion: Some("Set thread count between 1 and 128.".to_string()),
            });
        }
        if job.copy.inter_packet_gap_ms.is_some() {
            findings.push(ValidationFinding {
                code: "MT_INCOMPATIBLE_IPG".to_string(),
                severity: ValidationSeverity::Error,
                message: "/MT is incompatible with /IPG (Inter-Packet Gap).".to_string(),
                suggestion: Some("Disable /MT or clear /IPG.".to_string()),
            });
        }
        if job.copy.efs_raw {
            findings.push(ValidationFinding {
                code: "MT_INCOMPATIBLE_EFSRAW".to_string(),
                severity: ValidationSeverity::Error,
                message: "/MT is incompatible with /EFSRAW.".to_string(),
                suggestion: Some("Disable /MT when copying encrypted files in raw EFS mode.".to_string()),
            });
        }
        if job.retry.low_free_space_mode || job.retry.low_free_space_floor.is_some() {
            findings.push(ValidationFinding {
                code: "MT_INCOMPATIBLE_LFSM".to_string(),
                severity: ValidationSeverity::Error,
                message: "/MT is incompatible with /LFSM (Low Free Space Mode).".to_string(),
                suggestion: Some("Disable /MT when /LFSM is enabled.".to_string()),
            });
        }
    }

    if (job.retry.low_free_space_mode || job.retry.low_free_space_floor.is_some()) && job.copy.efs_raw {
        findings.push(ValidationFinding {
            code: "LFSM_INCOMPATIBLE_EFSRAW".to_string(),
            severity: ValidationSeverity::Error,
            message: "/LFSM is incompatible with /EFSRAW.".to_string(),
            suggestion: Some("Disable either /LFSM or /EFSRAW.".to_string()),
        });
    }

    if job.copy.mirror && (job.copy.move_files || job.copy.move_files_and_dirs) {
        findings.push(ValidationFinding {
            code: "MIR_WITH_MOVE".to_string(),
            severity: ValidationSeverity::Warning,
            message: "Combining /MIR with /MOV or /MOVE can delete files from both Source and Destination.".to_string(),
            suggestion: Some("Test thoroughly with Dry-Run (/L) before executing.".to_string()),
        });
    }

    if job.copy.create_tree_only && (job.copy.move_files || job.copy.move_files_and_dirs) {
        findings.push(ValidationFinding {
            code: "CREATE_WITH_MOVE".to_string(),
            severity: ValidationSeverity::Warning,
            message: "/CREATE creates zero-byte placeholder files and should not be combined with /MOV or /MOVE."
                .to_string(),
            suggestion: Some("Disable /MOV / /MOVE when building a /CREATE skeleton tree.".to_string()),
        });
    }

    if let Some(rh) = &job.copy.run_hours
        && !rh.trim().is_empty()
        && !valid_run_hours(rh)
    {
        findings.push(ValidationFinding {
            code: "INVALID_RUN_HOURS".to_string(),
            severity: ValidationSeverity::Error,
            message: format!("/RH:{rh} is invalid. Expected 24-hour HHMM-HHMM format (e.g. 2200-0600)."),
            suggestion: Some("Use four-digit 24h start and end times separated by a hyphen.".to_string()),
        });
    }

    for (flag, value_opt) in [
        ("/IoMaxSize", &job.throttle.io_max_size),
        ("/IoRate", &job.throttle.io_rate),
        ("/Threshold", &job.throttle.threshold),
        ("/LFSM", &job.retry.low_free_space_floor),
    ] {
        if let Some(val) = value_opt
            && !val.trim().is_empty()
            && !is_valid_kmg_spec(val)
        {
            findings.push(ValidationFinding {
                code: "INVALID_SIZE_SPEC".to_string(),
                severity: ValidationSeverity::Error,
                message: format!("{flag}:{val} must be an integer with optional K, M, or G suffix."),
                suggestion: Some("Example values: 64K, 25M, 2G, or plain byte count.".to_string()),
            });
        }
    }

    if (job.copy.backup_mode || job.copy.restartable_or_backup) && job.copy.copy_flags.skip_alt_data_streams {
        findings.push(ValidationFinding {
            code: "BACKUP_IGNORES_X".to_string(),
            severity: ValidationSeverity::Info,
            message: "Copy flag X (Skip Alt Data Streams) is ignored by Robocopy when /B or /ZB is enabled."
                .to_string(),
            suggestion: None,
        });
    }

    if let (Some(min), Some(max)) = (job.selection.min_size_bytes, job.selection.max_size_bytes)
        && min > max
    {
        findings.push(ValidationFinding {
            code: "MIN_GREATER_THAN_MAX".to_string(),
            severity: ValidationSeverity::Error,
            message: format!("/MIN:{min} is greater than /MAX:{max}; no files can match this size range."),
            suggestion: Some("Ensure /MIN is less than or equal to /MAX.".to_string()),
        });
    }

    if job.copy.monitor_changes == Some(0) || job.copy.monitor_minutes == Some(0) {
        findings.push(ValidationFinding {
            code: "INVALID_MONITOR_THRESHOLD".to_string(),
            severity: ValidationSeverity::Error,
            message: "/MON:0 and /MOT:0 are rejected by Robocopy (thresholds must be >= 1).".to_string(),
            suggestion: Some("Set /MON or /MOT to at least 1, or leave blank to disable monitoring.".to_string()),
        });
    }

    if job.logging.log_mode != LogFileMode::None && job.logging.log_path.as_deref().is_none_or(|p| p.trim().is_empty())
    {
        findings.push(ValidationFinding {
            code: "LOG_MODE_WITHOUT_PATH".to_string(),
            severity: ValidationSeverity::Warning,
            message: "A log file mode (/LOG or /UNILOG) is selected, but the log file path is empty.".to_string(),
            suggestion: Some("Specify a log file path or set Log Mode to None.".to_string()),
        });
    }

    if job.copy.per_file_run_hours && job.copy.run_hours.as_deref().is_none_or(|s| s.trim().is_empty()) {
        findings.push(ValidationFinding {
            code: "PF_WITHOUT_RH".to_string(),
            severity: ValidationSeverity::Warning,
            message: "/PF (per-file run hours check) is enabled without a /RH:hhmm-hhmm window.".to_string(),
            suggestion: Some("Specify a /RH:hhmm-hhmm window or uncheck /PF.".to_string()),
        });
    }

    if job.copy.backup_mode || job.copy.restartable_or_backup {
        findings.push(ValidationFinding {
            code: "BACKUP_PRIVILEGE_NOTE".to_string(),
            severity: ValidationSeverity::Info,
            message: "/B and /ZB backup mode may require Administrator / SeBackupPrivilege & SeRestorePrivilege."
                .to_string(),
            suggestion: None,
        });
    }

    if job.has_destructive_flags() {
        let flags = job.destructive_switch_names().join(", ");
        findings.push(ValidationFinding {
            code: "DESTRUCTIVE_FLAGS_ACTIVE".to_string(),
            severity: ValidationSeverity::Warning,
            message: format!("Destructive operation active ({flags}): files may be deleted."),
            suggestion: Some("Run a Dry-Run (/L) preview first to inspect planned deletions.".to_string()),
        });
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_mt_conflicts_and_destructive_flags() {
        let mut job = RobocopyJob {
            source: r"C:\Src".to_string(),
            destination: r"C:\Src\Sub".to_string(),
            ..Default::default()
        };
        job.copy.mirror = true;
        job.copy.multithreaded = Some(16);
        job.copy.inter_packet_gap_ms = Some(10);
        job.retry.low_free_space_mode = true;
        job.selection.min_size_bytes = Some(2048);
        job.selection.max_size_bytes = Some(1024);
        job.copy.monitor_changes = Some(0);
        job.logging.log_mode = LogFileMode::Overwrite;
        job.copy.per_file_run_hours = true;

        let findings = validate_job(&job);
        let codes: Vec<&str> = findings.iter().map(|f| f.code.as_str()).collect();
        assert!(codes.contains(&"RECURSIVE_DEST_IN_SRC"));
        assert!(codes.contains(&"MT_INCOMPATIBLE_IPG"));
        assert!(codes.contains(&"MT_INCOMPATIBLE_LFSM"));
        assert!(codes.contains(&"MIN_GREATER_THAN_MAX"));
        assert!(codes.contains(&"INVALID_MONITOR_THRESHOLD"));
        assert!(codes.contains(&"LOG_MODE_WITHOUT_PATH"));
        assert!(codes.contains(&"PF_WITHOUT_RH"));
        assert!(codes.contains(&"DESTRUCTIVE_FLAGS_ACTIVE"));
    }
}
