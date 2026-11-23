use crate::model::{LogFileMode, RobocopyJob, SparseMode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduleCadence {
    Daily,
    Weekly,
    Hourly,
    OnStart,
    OnLogon,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleSpec {
    pub cadence: ScheduleCadence,
    pub start_time: String,   // HH:MM
    pub days: Option<String>, // e.g., MON,WED,FRI for Weekly
    pub run_highest: bool,
}

impl Default for ScheduleSpec {
    fn default() -> Self {
        Self {
            cadence: ScheduleCadence::Daily,
            start_time: "02:00".to_string(),
            days: None,
            run_highest: true,
        }
    }
}

/// Normalizes a path for Microsoft Robocopy:
/// strips trailing `\` or `/` on non-root directories (`C:\dir\` -> `C:\dir`),
/// while preserving drive roots (`C:\`) and single root slash (`\`).
pub fn normalize_robocopy_path(path: &str) -> String {
    let trimmed = path.trim().trim_matches('"').trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let bytes = trimmed.as_bytes();
    if bytes.len() == 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && (bytes[2] == b'\\' || bytes[2] == b'/')
    {
        return format!("{}:\\", trimmed.chars().next().unwrap_or('C'));
    }
    if trimmed == "\\" || trimmed == "/" {
        return "\\".to_string();
    }

    let mut out = trimmed.to_string();
    while out.len() > 1 && (out.ends_with('\\') || out.ends_with('/')) {
        let b = out.as_bytes();
        if b.len() == 3 && b[0].is_ascii_alphabetic() && b[1] == b':' {
            break;
        }
        if out == "\\\\" {
            break;
        }
        out.pop();
    }
    out
}

/// Quotes an argument safely for Windows `CommandLineToArgvW` and `cmd.exe`,
/// doubling trailing backslashes immediately before the closing quote (`C:\` -> `"C:\\"`).
pub fn quote_windows_arg(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".to_string();
    }
    let needs_quotes = arg.chars().any(|c| {
        matches!(
            c,
            ' ' | '\t' | '"' | '&' | '|' | '<' | '>' | '^' | '(' | ')' | ';' | ',' | '%' | '!'
        )
    }) || arg.ends_with('\\');

    if !needs_quotes {
        return arg.to_string();
    }

    let mut out = String::with_capacity(arg.len() + 4);
    out.push('"');
    let mut backslashes = 0usize;
    for ch in arg.chars() {
        if ch == '\\' {
            backslashes += 1;
        } else if ch == '"' {
            for _ in 0..(backslashes * 2 + 1) {
                out.push('\\');
            }
            out.push('"');
            backslashes = 0;
        } else {
            for _ in 0..backslashes {
                out.push('\\');
            }
            out.push(ch);
            backslashes = 0;
        }
    }
    for _ in 0..(backslashes * 2) {
        out.push('\\');
    }
    out.push('"');
    out
}

/// Builds the exact ordered argument list to pass to `robocopy.exe` (without the `robocopy` binary name).
pub fn build_args(job: &RobocopyJob) -> Vec<String> {
    let mut args = Vec::new();

    if !job.job_files.no_source_dir && !job.source.trim().is_empty() {
        args.push(normalize_robocopy_path(&job.source));
    }
    if !job.job_files.no_dest_dir && !job.destination.trim().is_empty() {
        args.push(normalize_robocopy_path(&job.destination));
    }

    for spec in &job.file_specs {
        let trimmed = spec.trim();
        if !trimmed.is_empty() {
            args.push(trimmed.to_string());
        }
    }

    // Copy options
    let c = &job.copy;
    if c.mirror {
        args.push("/MIR".to_string());
    } else {
        if c.subdirectories_including_empty {
            args.push("/E".to_string());
        } else if c.subdirectories_non_empty {
            args.push("/S".to_string());
        }
        if c.purge {
            args.push("/PURGE".to_string());
        }
    }

    if let Some(lev) = c.max_levels {
        args.push(format!("/LEV:{lev}"));
    }

    if c.restartable_or_backup {
        args.push("/ZB".to_string());
    } else {
        if c.restartable {
            args.push("/Z".to_string());
        }
        if c.backup_mode {
            args.push("/B".to_string());
        }
    }

    if c.unbuffered_io {
        args.push("/J".to_string());
    }
    if c.efs_raw {
        args.push("/EFSRAW".to_string());
    }

    if c.no_copy {
        args.push("/NOCOPY".to_string());
    } else if !c.copy_flags.is_default() && !c.copy_flags.is_empty() {
        if c.copy_flags.is_all() {
            args.push("/COPYALL".to_string());
        } else if c.copy_flags.is_sec() {
            args.push("/SEC".to_string());
        } else {
            args.push(format!("/COPY:{}", c.copy_flags.to_flag_string()));
        }
    }

    if c.sec_fix {
        args.push("/SECFIX".to_string());
    }
    if c.tim_fix {
        args.push("/TIMFIX".to_string());
    }

    if c.move_files_and_dirs {
        args.push("/MOVE".to_string());
    } else if c.move_files {
        args.push("/MOV".to_string());
    }

    if !c.add_attributes.is_empty() {
        args.push(format!("/A+:{}", c.add_attributes.to_flag_string()));
    }
    if !c.remove_attributes.is_empty() {
        args.push(format!("/A-:{}", c.remove_attributes.to_flag_string()));
    }

    if c.create_tree_only {
        args.push("/CREATE".to_string());
    }
    if c.fat_names {
        args.push("/FAT".to_string());
    }
    if c.disable_long_paths {
        args.push("/256".to_string());
    }
    if let Some(mon) = c.monitor_changes {
        args.push(format!("/MON:{mon}"));
    }
    if let Some(mot) = c.monitor_minutes {
        args.push(format!("/MOT:{mot}"));
    }
    if let Some(rh) = &c.run_hours {
        let trimmed = rh.trim();
        if !trimmed.is_empty() {
            args.push(format!("/RH:{trimmed}"));
        }
    }
    if c.per_file_run_hours {
        args.push("/PF".to_string());
    }
    if let Some(ipg) = c.inter_packet_gap_ms {
        args.push(format!("/IPG:{ipg}"));
    }
    if c.copy_junctions {
        args.push("/SJ".to_string());
    }
    if c.copy_symlinks {
        args.push("/SL".to_string());
    }
    if let Some(mt) = c.multithreaded {
        if mt == 8 {
            args.push("/MT:8".to_string());
        } else {
            args.push(format!("/MT:{mt}"));
        }
    }

    if c.no_dir_copy {
        args.push("/NODCOPY".to_string());
    } else if !c.dir_copy_flags.is_default() && !c.dir_copy_flags.is_empty() {
        args.push(format!("/DCOPY:{}", c.dir_copy_flags.to_flag_string()));
    }

    if c.no_offload {
        args.push("/NOOFFLOAD".to_string());
    }
    if c.network_compress {
        args.push("/COMPRESS".to_string());
    }
    match c.sparse_mode {
        SparseMode::Default => {}
        SparseMode::Enable => args.push("/SPARSE:Y".to_string()),
        SparseMode::Disable => args.push("/SPARSE:N".to_string()),
    }
    if c.no_clone {
        args.push("/NOCLONE".to_string());
    }

    // Throttle options
    if let Some(v) = &job.throttle.io_max_size {
        let t = v.trim();
        if !t.is_empty() {
            args.push(format!("/IoMaxSize:{t}"));
        }
    }
    if let Some(v) = &job.throttle.io_rate {
        let t = v.trim();
        if !t.is_empty() {
            args.push(format!("/IoRate:{t}"));
        }
    }
    if let Some(v) = &job.throttle.threshold {
        let t = v.trim();
        if !t.is_empty() {
            args.push(format!("/Threshold:{t}"));
        }
    }

    // File selection options (scalar switches first, /XF and /XD below)
    let s = &job.selection;
    if s.archive_and_reset {
        args.push("/M".to_string());
    } else if s.archive_only {
        args.push("/A".to_string());
    }
    if !s.include_attrs.is_empty() {
        args.push(format!("/IA:{}", s.include_attrs.to_flag_string()));
    }
    if !s.exclude_attrs.is_empty() {
        args.push(format!("/XA:{}", s.exclude_attrs.to_flag_string()));
    }
    if s.exclude_changed {
        args.push("/XC".to_string());
    }
    if s.exclude_newer {
        args.push("/XN".to_string());
    }
    if s.exclude_older {
        args.push("/XO".to_string());
    }
    if s.exclude_extra {
        args.push("/XX".to_string());
    }
    if s.exclude_lonely {
        args.push("/XL".to_string());
    }
    if s.include_same {
        args.push("/IS".to_string());
    }
    if s.include_tweaked {
        args.push("/IT".to_string());
    }
    if s.include_modified {
        args.push("/IM".to_string());
    }
    if let Some(max) = s.max_size_bytes {
        args.push(format!("/MAX:{max}"));
    }
    if let Some(min) = s.min_size_bytes {
        args.push(format!("/MIN:{min}"));
    }
    if let Some(v) = &s.max_age {
        let t = v.trim();
        if !t.is_empty() {
            args.push(format!("/MAXAGE:{t}"));
        }
    }
    if let Some(v) = &s.min_age {
        let t = v.trim();
        if !t.is_empty() {
            args.push(format!("/MINAGE:{t}"));
        }
    }
    if let Some(v) = &s.max_last_access {
        let t = v.trim();
        if !t.is_empty() {
            args.push(format!("/MAXLAD:{t}"));
        }
    }
    if let Some(v) = &s.min_last_access {
        let t = v.trim();
        if !t.is_empty() {
            args.push(format!("/MINLAD:{t}"));
        }
    }
    if s.fat_file_times {
        args.push("/FFT".to_string());
    }
    if s.compensate_dst {
        args.push("/DST".to_string());
    }
    if s.exclude_junctions {
        args.push("/XJ".to_string());
    } else {
        if s.exclude_dir_junctions {
            args.push("/XJD".to_string());
        }
        if s.exclude_file_symlinks {
            args.push("/XJF".to_string());
        }
    }

    // Retry options
    if let Some(r) = job.retry.retries {
        args.push(format!("/R:{r}"));
    }
    if let Some(w) = job.retry.wait_seconds {
        args.push(format!("/W:{w}"));
    }
    if job.retry.save_defaults_to_registry {
        args.push("/REG".to_string());
    }
    if job.retry.wait_for_sharenames {
        args.push("/TBD".to_string());
    }
    if let Some(floor) = &job.retry.low_free_space_floor {
        let t = floor.trim();
        if !t.is_empty() {
            args.push(format!("/LFSM:{t}"));
        } else if job.retry.low_free_space_mode {
            args.push("/LFSM".to_string());
        }
    } else if job.retry.low_free_space_mode {
        args.push("/LFSM".to_string());
    }

    // Logging options
    let l = &job.logging;
    if l.list_only {
        args.push("/L".to_string());
    }
    if l.report_extra {
        args.push("/X".to_string());
    }
    if l.verbose {
        args.push("/V".to_string());
    }
    if l.include_timestamps {
        args.push("/TS".to_string());
    }
    if l.full_paths {
        args.push("/FP".to_string());
    }
    if l.sizes_in_bytes {
        args.push("/BYTES".to_string());
    }
    if l.no_sizes {
        args.push("/NS".to_string());
    }
    if l.no_classes {
        args.push("/NC".to_string());
    }
    if l.no_file_list {
        args.push("/NFL".to_string());
    }
    if l.no_dir_list {
        args.push("/NDL".to_string());
    }
    if l.no_progress {
        args.push("/NP".to_string());
    }
    if l.show_eta {
        args.push("/ETA".to_string());
    }
    if let Some(log_path) = &l.log_path {
        let p = log_path.trim();
        if !p.is_empty() {
            match l.log_mode {
                LogFileMode::None => {}
                LogFileMode::Overwrite => args.push(format!("/LOG:{p}")),
                LogFileMode::Append => args.push(format!("/LOG+:{p}")),
                LogFileMode::UnicodeOverwrite => args.push(format!("/UNILOG:{p}")),
                LogFileMode::UnicodeAppend => args.push(format!("/UNILOG+:{p}")),
            }
        }
    }
    if l.tee {
        args.push("/TEE".to_string());
    }
    if l.no_job_header {
        args.push("/NJH".to_string());
    }
    if l.no_job_summary {
        args.push("/NJS".to_string());
    }
    if l.unicode_output {
        args.push("/UNICODE".to_string());
    }

    // Job file options
    if let Some(load_job) = &job.job_files.load_job {
        let t = load_job.trim();
        if !t.is_empty() {
            args.push(format!("/JOB:{t}"));
        }
    }
    if let Some(save_job) = &job.job_files.save_job {
        let t = save_job.trim();
        if !t.is_empty() {
            args.push(format!("/SAVE:{t}"));
        }
    }
    if job.job_files.quit_after_parse {
        args.push("/QUIT".to_string());
    }
    if job.job_files.no_source_dir {
        args.push("/NOSD".to_string());
    }
    if job.job_files.no_dest_dir {
        args.push("/NODD".to_string());
    }

    // Multi-argument exclusions (/XF and /XD) placed after scalar switches
    let xf: Vec<&str> = s
        .exclude_files
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .collect();
    if !xf.is_empty() {
        args.push("/XF".to_string());
        for item in xf {
            args.push(item.to_string());
        }
    }

    let xd: Vec<&str> = s
        .exclude_dirs
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .collect();
    if !xd.is_empty() {
        args.push("/XD".to_string());
        for item in xd {
            args.push(normalize_robocopy_path(item));
        }
    }

    for extra in &job.extra_args {
        let t = extra.trim();
        if !t.is_empty() {
            args.push(t.to_string());
        }
    }

    args
}

fn sanitize_single_line(s: &str) -> String {
    s.replace(['\r', '\n'], " ")
}

fn escape_batch_percent(s: &str) -> String {
    sanitize_single_line(s).replace('%', "%%")
}

fn escape_batch_echo(s: &str) -> String {
    let clean = sanitize_single_line(s);
    let mut out = String::with_capacity(clean.len() + 8);
    for ch in clean.chars() {
        match ch {
            '^' => out.push_str("^^"),
            '%' => out.push_str("%%"),
            '&' => out.push_str("^&"),
            '|' => out.push_str("^|"),
            '<' => out.push_str("^<"),
            '>' => out.push_str("^>"),
            '(' => out.push_str("^("),
            ')' => out.push_str("^)"),
            '"' => out.push_str("^\""),
            _ => out.push(ch),
        }
    }
    out
}

/// Formats a complete, Windows-safe `robocopy ...` command string.
pub fn format_command_line(job: &RobocopyJob) -> String {
    let args = build_args(job);
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push("robocopy".to_string());
    for arg in args {
        parts.push(quote_windows_arg(&sanitize_single_line(&arg)));
    }
    parts.join(" ")
}

/// Generates a Windows `.bat` script with proper Robocopy bitmask exit-code handling (`0..=7` = success).
pub fn build_batch_script(job: &RobocopyJob) -> String {
    let cmd = format_command_line(job);
    let exec_cmd = escape_batch_percent(&cmd);
    let echo_cmd = escape_batch_echo(&cmd);
    let raw_name = if job.name.trim().is_empty() {
        "Robocopy2Rust Job"
    } else {
        job.name.trim()
    };
    let rem_name = sanitize_single_line(raw_name);
    let echo_name = escape_batch_echo(raw_name);
    format!(
        "@echo off\r\nsetlocal\r\nrem Generated by robocopy2rust - {rem_name}\r\necho Starting Robocopy job: {echo_name}\r\necho Command: {echo_cmd}\r\n{exec_cmd}\r\nset \"RC=%ERRORLEVEL%\"\r\necho Robocopy exited with bitmask code %RC%\r\nif %RC% GEQ 8 (\r\n    echo [ERROR] Robocopy encountered copy or fatal errors ^(code %RC%^).\r\n    exit /b %RC%\r\n)\r\nexit /b 0\r\n"
    )
}

/// Generates a PowerShell `.ps1` script using an explicit argument array to avoid shell-quoting bugs.
pub fn build_powershell_script(job: &RobocopyJob) -> String {
    let args = build_args(job);
    let raw_name = if job.name.trim().is_empty() {
        "Robocopy2Rust Job"
    } else {
        job.name.trim()
    };
    let comment_name = sanitize_single_line(raw_name);
    let ps_name = comment_name.replace('\'', "''");
    let ps_args: Vec<String> = args
        .into_iter()
        .map(|a| format!("    '{}'", sanitize_single_line(&a).replace('\'', "''")))
        .collect();
    let body = ps_args.join(",\r\n");
    format!(
        "# Generated by robocopy2rust - {comment_name}\r\n$ErrorActionPreference = 'Stop'\r\n$robocopyArgs = @(\r\n{body}\r\n)\r\nWrite-Host 'Starting Robocopy job: {ps_name}'\r\n& robocopy.exe @robocopyArgs\r\n$rc = $LASTEXITCODE\r\nWrite-Host \"Robocopy exited with bitmask code $rc\"\r\nif ($rc -ge 8) {{\r\n    Write-Error \"Robocopy failed with exit code $rc\"\r\n    exit $rc\r\n}}\r\nexit 0\r\n"
    )
}

/// Builds a `schtasks /Create ...` command line to register the job or wrapper script with Windows Task Scheduler.
pub fn build_schtasks_command(task_name: &str, task_run_target: &str, schedule: &ScheduleSpec) -> String {
    let clean_name = if task_name.trim().is_empty() {
        "Robocopy2Rust_Job"
    } else {
        task_name.trim()
    };
    let mut parts = vec![
        "schtasks".to_string(),
        "/Create".to_string(),
        "/TN".to_string(),
        quote_windows_arg(clean_name),
        "/TR".to_string(),
        quote_windows_arg(task_run_target.trim()),
        "/SC".to_string(),
    ];
    match schedule.cadence {
        ScheduleCadence::Daily => {
            parts.push("DAILY".to_string());
            if !schedule.start_time.trim().is_empty() {
                parts.push("/ST".to_string());
                parts.push(schedule.start_time.trim().to_string());
            }
        }
        ScheduleCadence::Weekly => {
            parts.push("WEEKLY".to_string());
            if let Some(days) = &schedule.days
                && !days.trim().is_empty()
            {
                parts.push("/D".to_string());
                parts.push(days.trim().to_string());
            }
            if !schedule.start_time.trim().is_empty() {
                parts.push("/ST".to_string());
                parts.push(schedule.start_time.trim().to_string());
            }
        }
        ScheduleCadence::Hourly => {
            parts.push("HOURLY".to_string());
            if !schedule.start_time.trim().is_empty() {
                parts.push("/ST".to_string());
                parts.push(schedule.start_time.trim().to_string());
            }
        }
        ScheduleCadence::OnStart => parts.push("ONSTART".to_string()),
        ScheduleCadence::OnLogon => parts.push("ONLOGON".to_string()),
    }
    if schedule.run_highest {
        parts.push("/RL".to_string());
        parts.push("HIGHEST".to_string());
    }
    parts.push("/F".to_string());
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_paths_preserving_roots() {
        assert_eq!(normalize_robocopy_path(r"C:\"), r"C:\");
        assert_eq!(normalize_robocopy_path(r"C:/"), r"C:\");
        assert_eq!(normalize_robocopy_path(r"C:\Users\Alice\"), r"C:\Users\Alice");
        assert_eq!(normalize_robocopy_path(r"\\server\share\"), r"\\server\share");
        assert_eq!(normalize_robocopy_path(r"  D:\Backup\Docs/  "), r"D:\Backup\Docs");
    }

    #[test]
    fn quotes_windows_args_and_trailing_backslashes() {
        assert_eq!(quote_windows_arg(r"C:\Data"), r"C:\Data");
        assert_eq!(quote_windows_arg(r"C:\"), r#""C:\\""#);
        assert_eq!(quote_windows_arg(r"C:\My Docs"), r#""C:\My Docs""#);
        assert_eq!(quote_windows_arg(r"C:\My Docs\"), r#""C:\My Docs\\""#);
    }

    #[test]
    fn formats_mirror_and_exclusions_in_expected_order() {
        let mut job = RobocopyJob::empty_cli_defaults();
        job.source = r"C:\Src Folder\".to_string();
        job.destination = r"D:\Dst\".to_string();
        job.file_specs = vec!["*.docx".to_string()];
        job.copy.mirror = true;
        job.copy.subdirectories_including_empty = true;
        job.copy.purge = true;
        job.copy.multithreaded = Some(16);
        job.retry.retries = Some(3);
        job.retry.wait_seconds = Some(5);
        job.selection.exclude_files = vec!["*.tmp".to_string()];
        job.selection.exclude_dirs = vec![".git".to_string(), r"D:\Extra\".to_string()];

        let cmd = format_command_line(&job);
        assert_eq!(
            cmd,
            r#"robocopy "C:\Src Folder" D:\Dst *.docx /MIR /MT:16 /R:3 /W:5 /XF *.tmp /XD .git D:\Extra"#
        );
    }

    #[test]
    fn escapes_batch_and_powershell_scripts_safely() {
        let mut job = RobocopyJob::empty_cli_defaults();
        job.name = "Sync & Calc | ^Test%\r\necho injected".to_string();
        job.source = r"C:\Data\%TEMP%\Src".to_string();
        job.destination = "D:\\Dst\nline2".to_string();

        let bat = build_batch_script(&job);
        assert!(!bat.contains("rem Generated by robocopy2rust - Sync & Calc | ^Test%\r\necho"));
        assert!(bat.contains("echo Starting Robocopy job: Sync ^& Calc ^| ^^Test%%  echo injected"));
        assert!(bat.contains(r"%%TEMP%%"));
        assert!(!bat.contains("\necho injected\r\n"));

        let ps1 = build_powershell_script(&job);
        assert!(ps1.contains("# Generated by robocopy2rust - Sync & Calc | ^Test%  echo injected"));
        assert!(ps1.contains("'D:\\Dst line2'"));
    }
}
