use crate::model::{CopyFlags, DirCopyFlags, FileAttrFlags, LogFileMode, RobocopyJob, SparseMode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseResult {
    pub job: RobocopyJob,
    pub warnings: Vec<String>,
}

/// Tokenizes a Windows command line following `CommandLineToArgvW` backslash-quote escaping rules.
pub fn tokenize_windows_cmdline(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut token_started = false;
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        let ch = chars[i];
        if !in_quotes && ch.is_whitespace() {
            if token_started {
                tokens.push(std::mem::take(&mut current));
                token_started = false;
            }
            i += 1;
            continue;
        }

        if ch == '\\' {
            let mut slashes = 0usize;
            while i < chars.len() && chars[i] == '\\' {
                slashes += 1;
                i += 1;
            }
            if i < chars.len() && chars[i] == '"' {
                for _ in 0..(slashes / 2) {
                    current.push('\\');
                }
                if slashes % 2 == 1 {
                    current.push('"');
                } else {
                    in_quotes = !in_quotes;
                }
                token_started = true;
                i += 1;
            } else {
                for _ in 0..slashes {
                    current.push('\\');
                }
                token_started = true;
            }
            continue;
        }

        if ch == '"' {
            in_quotes = !in_quotes;
            token_started = true;
            i += 1;
            continue;
        }

        current.push(ch);
        token_started = true;
        i += 1;
    }

    if token_started {
        tokens.push(current);
    }

    tokens
}

pub fn parse_command_line(input: &str) -> ParseResult {
    let tokens = tokenize_windows_cmdline(input);
    parse_tokens(&tokens)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MultiListSection {
    None,
    Xf,
    Xd,
}

pub fn parse_tokens(tokens: &[String]) -> ParseResult {
    let mut job = RobocopyJob::empty_cli_defaults();
    let mut warnings = Vec::new();

    let mut idx = 0usize;
    if let Some(first) = tokens.first() {
        let lower = first.trim().to_ascii_lowercase();
        let file_name = std::path::Path::new(&lower)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&lower);
        if matches!(file_name, "robocopy" | "robocopy.exe") {
            idx = 1;
        }
    }

    let mut seen_switch = false;
    let mut section = MultiListSection::None;

    while idx < tokens.len() {
        let tok = tokens[idx].trim();
        idx += 1;
        if tok.is_empty() {
            continue;
        }

        if tok.starts_with('/') {
            seen_switch = true;
            section = MultiListSection::None;
            let upper = tok.to_ascii_uppercase();
            if upper == "/XF" {
                section = MultiListSection::Xf;
                continue;
            }
            if upper == "/XD" {
                section = MultiListSection::Xd;
                continue;
            }
            apply_switch(tok, &mut job, &mut warnings);
            continue;
        }

        match section {
            MultiListSection::Xf => job.selection.exclude_files.push(tok.to_string()),
            MultiListSection::Xd => job.selection.exclude_dirs.push(tok.to_string()),
            MultiListSection::None => {
                if !seen_switch {
                    if job.source.is_empty() && !job.job_files.no_source_dir {
                        job.source = tok.to_string();
                    } else if job.destination.is_empty() && !job.job_files.no_dest_dir {
                        job.destination = tok.to_string();
                    } else {
                        job.file_specs.push(tok.to_string());
                    }
                } else {
                    warnings.push(format!("Unexpected positional argument after switches: '{tok}'"));
                    job.extra_args.push(tok.to_string());
                }
            }
        }
    }

    ParseResult { job, warnings }
}

pub(crate) fn apply_switch(tok: &str, job: &mut RobocopyJob, warnings: &mut Vec<String>) {
    let (key_raw, val_opt) = match tok.split_once(':') {
        Some((k, v)) => (k, Some(v)),
        None => (tok, None),
    };
    let key = key_raw.to_ascii_uppercase();
    let val = val_opt.unwrap_or("").trim();

    match key.as_str() {
        "/S" => job.copy.subdirectories_non_empty = true,
        "/E" => job.copy.subdirectories_including_empty = true,
        "/LEV" => match val.parse::<u32>() {
            Ok(n) => job.copy.max_levels = Some(n),
            Err(_) => warnings.push(format!("Invalid /LEV value in '{tok}'")),
        },
        "/Z" => job.copy.restartable = true,
        "/B" => job.copy.backup_mode = true,
        "/ZB" => job.copy.restartable_or_backup = true,
        "/J" => job.copy.unbuffered_io = true,
        "/EFSRAW" => job.copy.efs_raw = true,
        "/COPY" => match CopyFlags::from_flag_string(val) {
            Ok(flags) => {
                job.copy.copy_flags = flags;
                job.copy.no_copy = false;
            }
            Err(e) => warnings.push(format!("{e} in '{tok}'")),
        },
        "/SEC" => {
            job.copy.copy_flags = CopyFlags::sec();
            job.copy.no_copy = false;
        }
        "/COPYALL" => {
            job.copy.copy_flags = CopyFlags::copy_all();
            job.copy.no_copy = false;
        }
        "/NOCOPY" => job.copy.no_copy = true,
        "/SECFIX" => job.copy.sec_fix = true,
        "/TIMFIX" => job.copy.tim_fix = true,
        "/PURGE" => job.copy.purge = true,
        "/MIR" => {
            job.copy.mirror = true;
            job.copy.subdirectories_including_empty = true;
            job.copy.purge = true;
        }
        "/MOV" => job.copy.move_files = true,
        "/MOVE" => job.copy.move_files_and_dirs = true,
        "/A+" => match FileAttrFlags::from_flag_string(val) {
            Ok(f) => job.copy.add_attributes = f,
            Err(e) => warnings.push(format!("{e} in '{tok}'")),
        },
        "/A-" => match FileAttrFlags::from_flag_string(val) {
            Ok(f) => job.copy.remove_attributes = f,
            Err(e) => warnings.push(format!("{e} in '{tok}'")),
        },
        "/CREATE" => job.copy.create_tree_only = true,
        "/FAT" => job.copy.fat_names = true,
        "/256" => job.copy.disable_long_paths = true,
        "/MON" => match val.parse::<u32>() {
            Ok(n) => job.copy.monitor_changes = Some(n),
            Err(_) => warnings.push(format!("Invalid /MON value in '{tok}'")),
        },
        "/MOT" => match val.parse::<u32>() {
            Ok(n) => job.copy.monitor_minutes = Some(n),
            Err(_) => warnings.push(format!("Invalid /MOT value in '{tok}'")),
        },
        "/RH" => job.copy.run_hours = Some(val.to_string()),
        "/PF" => job.copy.per_file_run_hours = true,
        "/IPG" => match val.parse::<u32>() {
            Ok(n) => job.copy.inter_packet_gap_ms = Some(n),
            Err(_) => warnings.push(format!("Invalid /IPG value in '{tok}'")),
        },
        "/SJ" => job.copy.copy_junctions = true,
        "/SL" => job.copy.copy_symlinks = true,
        "/MT" => {
            if val.is_empty() {
                job.copy.multithreaded = Some(8);
            } else {
                match val.parse::<u16>() {
                    Ok(n) => job.copy.multithreaded = Some(n),
                    Err(_) => warnings.push(format!("Invalid /MT thread count in '{tok}'")),
                }
            }
        }
        "/DCOPY" => match DirCopyFlags::from_flag_string(val) {
            Ok(flags) => {
                job.copy.dir_copy_flags = flags;
                job.copy.no_dir_copy = false;
            }
            Err(e) => warnings.push(format!("{e} in '{tok}'")),
        },
        "/NODCOPY" => job.copy.no_dir_copy = true,
        "/NOOFFLOAD" => job.copy.no_offload = true,
        "/COMPRESS" => job.copy.network_compress = true,
        "/SPARSE" => match val.to_ascii_uppercase().as_str() {
            "" | "Y" => job.copy.sparse_mode = SparseMode::Enable,
            "N" => job.copy.sparse_mode = SparseMode::Disable,
            _ => warnings.push(format!("Invalid /SPARSE flag in '{tok}'")),
        },
        "/NOCLONE" => job.copy.no_clone = true,

        // Throttling
        "/IOMAXSIZE" => job.throttle.io_max_size = Some(val.to_string()),
        "/IORATE" => job.throttle.io_rate = Some(val.to_string()),
        "/THRESHOLD" => job.throttle.threshold = Some(val.to_string()),

        // File selection
        "/A" => job.selection.archive_only = true,
        "/M" => job.selection.archive_and_reset = true,
        "/IA" => match FileAttrFlags::from_flag_string(val) {
            Ok(f) => job.selection.include_attrs = f,
            Err(e) => warnings.push(format!("{e} in '{tok}'")),
        },
        "/XA" => match FileAttrFlags::from_flag_string(val) {
            Ok(f) => job.selection.exclude_attrs = f,
            Err(e) => warnings.push(format!("{e} in '{tok}'")),
        },
        "/XC" => job.selection.exclude_changed = true,
        "/XN" => job.selection.exclude_newer = true,
        "/XO" => job.selection.exclude_older = true,
        "/XX" => job.selection.exclude_extra = true,
        "/XL" => job.selection.exclude_lonely = true,
        "/IS" => job.selection.include_same = true,
        "/IT" => job.selection.include_tweaked = true,
        "/IM" => job.selection.include_modified = true,
        "/MAX" => match val.parse::<u64>() {
            Ok(n) => job.selection.max_size_bytes = Some(n),
            Err(_) => warnings.push(format!("Invalid /MAX size in '{tok}'")),
        },
        "/MIN" => match val.parse::<u64>() {
            Ok(n) => job.selection.min_size_bytes = Some(n),
            Err(_) => warnings.push(format!("Invalid /MIN size in '{tok}'")),
        },
        "/MAXAGE" => job.selection.max_age = Some(val.to_string()),
        "/MINAGE" => job.selection.min_age = Some(val.to_string()),
        "/MAXLAD" => job.selection.max_last_access = Some(val.to_string()),
        "/MINLAD" => job.selection.min_last_access = Some(val.to_string()),
        "/FFT" => job.selection.fat_file_times = true,
        "/DST" => job.selection.compensate_dst = true,
        "/XJ" => job.selection.exclude_junctions = true,
        "/XJD" => job.selection.exclude_dir_junctions = true,
        "/XJF" => job.selection.exclude_file_symlinks = true,

        // Retry
        "/R" => match val.parse::<u32>() {
            Ok(n) => job.retry.retries = Some(n),
            Err(_) => warnings.push(format!("Invalid /R count in '{tok}'")),
        },
        "/W" => match val.parse::<u32>() {
            Ok(n) => job.retry.wait_seconds = Some(n),
            Err(_) => warnings.push(format!("Invalid /W seconds in '{tok}'")),
        },
        "/REG" => job.retry.save_defaults_to_registry = true,
        "/TBD" => job.retry.wait_for_sharenames = true,
        "/LFSM" => {
            job.retry.low_free_space_mode = true;
            if !val.is_empty() {
                job.retry.low_free_space_floor = Some(val.to_string());
            }
        }

        // Logging
        "/L" => job.logging.list_only = true,
        "/X" => job.logging.report_extra = true,
        "/V" => job.logging.verbose = true,
        "/TS" => job.logging.include_timestamps = true,
        "/FP" => job.logging.full_paths = true,
        "/BYTES" => job.logging.sizes_in_bytes = true,
        "/NS" => job.logging.no_sizes = true,
        "/NC" => job.logging.no_classes = true,
        "/NFL" => job.logging.no_file_list = true,
        "/NDL" => job.logging.no_dir_list = true,
        "/NP" => job.logging.no_progress = true,
        "/ETA" => job.logging.show_eta = true,
        "/LOG" => {
            job.logging.log_mode = LogFileMode::Overwrite;
            job.logging.log_path = Some(val.to_string());
        }
        "/LOG+" => {
            job.logging.log_mode = LogFileMode::Append;
            job.logging.log_path = Some(val.to_string());
        }
        "/UNILOG" => {
            job.logging.log_mode = LogFileMode::UnicodeOverwrite;
            job.logging.log_path = Some(val.to_string());
        }
        "/UNILOG+" => {
            job.logging.log_mode = LogFileMode::UnicodeAppend;
            job.logging.log_path = Some(val.to_string());
        }
        "/TEE" => job.logging.tee = true,
        "/NJH" => job.logging.no_job_header = true,
        "/NJS" => job.logging.no_job_summary = true,
        "/UNICODE" => job.logging.unicode_output = true,

        // Job options
        "/JOB" => job.job_files.load_job = Some(val.to_string()),
        "/SAVE" => job.job_files.save_job = Some(val.to_string()),
        "/QUIT" => job.job_files.quit_after_parse = true,
        "/NOSD" => job.job_files.no_source_dir = true,
        "/NODD" => job.job_files.no_dest_dir = true,

        // Internal RCJ directives occasionally found if pasted
        "/SD" => job.source = val.to_string(),
        "/DD" => job.destination = val.to_string(),

        _ => {
            warnings.push(format!("Unrecognized Robocopy option preserved in extra_args: '{tok}'"));
            job.extra_args.push(tok.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::format_command_line;

    #[test]
    fn roundtrip_complex_command_line() {
        let cmd = r#"robocopy "C:\\" "D:\My Backup" *.docx *.xlsx /MIR /ZB /COPYALL /DCOPY:DAT /MT:16 /IoRate:50M /XO /XJ /R:2 /W:3 /LOG+:C:\logs\sync.log /TEE /XF *.tmp ~$* /XD .git node_modules"#;
        let parsed = parse_command_line(cmd);
        assert!(parsed.warnings.is_empty(), "warnings: {:?}", parsed.warnings);
        assert_eq!(parsed.job.source, r"C:\");
        assert_eq!(parsed.job.destination, r"D:\My Backup");
        assert_eq!(parsed.job.file_specs, vec!["*.docx", "*.xlsx"]);
        assert!(parsed.job.copy.mirror);
        assert!(parsed.job.copy.restartable_or_backup);
        assert!(parsed.job.copy.copy_flags.is_all());
        assert!(parsed.job.copy.dir_copy_flags.timestamps);
        assert_eq!(parsed.job.copy.multithreaded, Some(16));
        assert_eq!(parsed.job.throttle.io_rate.as_deref(), Some("50M"));
        assert!(parsed.job.selection.exclude_older);
        assert!(parsed.job.selection.exclude_junctions);
        assert_eq!(parsed.job.retry.retries, Some(2));
        assert_eq!(parsed.job.retry.wait_seconds, Some(3));
        assert_eq!(parsed.job.logging.log_mode, LogFileMode::Append);
        assert_eq!(parsed.job.selection.exclude_files, vec!["*.tmp", "~$*"]);
        assert_eq!(parsed.job.selection.exclude_dirs, vec![".git", "node_modules"]);

        let formatted = format_command_line(&parsed.job);
        let reparsed = parse_command_line(&formatted);
        assert_eq!(reparsed.job, parsed.job);
    }
}
