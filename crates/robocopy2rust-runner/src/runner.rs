use crate::env::find_robocopy_executable;
use crate::output_parser::{FileActionRecord, ParsedLineOutcome, RobocopyOutputParser, RunSummary};
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use robocopy2rust_core::{ExitCodeInfo, RobocopyJob, build_args, decode_exit_code, format_command_line};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    pub job_name: String,
    pub source: String,
    pub destination: String,
    pub dry_run: bool,
    pub command_line: String,
    pub args: Vec<String>,
    pub started_at: String,
    pub ended_at: String,
    pub duration_secs: f64,
    pub exit_code: i32,
    pub exit_info: ExitCodeInfo,
    pub cancelled: bool,
    pub summary: RunSummary,
    pub file_actions: Vec<FileActionRecord>,
    pub errors: Vec<String>,
    pub log_lines: Vec<String>,
    pub log_sha256: String,
}

#[derive(Debug, Clone)]
pub enum RunnerEvent {
    Started { command_line: String },
    LogLine(String),
    FileAction(FileActionRecord),
    Progress { percent: f32, current_file: Option<String> },
    Error(String),
    Summary(RunSummary),
    Finished(Box<RunResult>),
}

type KillCallback = Box<dyn FnMut() + Send>;

#[derive(Clone, Default)]
pub struct CancelToken {
    cancelled: Arc<AtomicBool>,
    child_pid: Arc<Mutex<Option<u32>>>,
    killer: Arc<Mutex<Option<KillCallback>>>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Ok(mut guard) = self.killer.lock()
            && let Some(mut kill_fn) = guard.take()
        {
            kill_fn();
        }
    }

    pub fn pid(&self) -> Option<u32> {
        self.child_pid.lock().ok().and_then(|g| *g)
    }
}

/// Spawns Robocopy in a background worker thread and streams real-time [`RunnerEvent`]s
/// over an `mpsc::Receiver` while returning a [`CancelToken`] for immediate cancellation.
pub fn spawn_job_runner(job: RobocopyJob, binary_override: Option<PathBuf>) -> (Receiver<RunnerEvent>, CancelToken) {
    let (tx, rx) = mpsc::channel();
    let cancel = CancelToken::new();
    let cancel_worker = cancel.clone();

    std::thread::spawn(move || {
        let res = execute_job_internal(&job, binary_override.as_deref(), &cancel_worker, |ev| {
            let _ = tx.send(ev);
        });
        match res {
            Ok(final_res) => {
                let _ = tx.send(RunnerEvent::Finished(Box::new(final_res)));
            }
            Err(err) => {
                let now = Utc::now().to_rfc3339();
                let err_msg = err.to_string();
                let _ = tx.send(RunnerEvent::Error(err_msg.clone()));
                let mut hasher = Sha256::new();
                hasher.update(err_msg.as_bytes());
                let digest = robocopy2rust_core::format::encode_hex_lower(&hasher.finalize());
                let fallback = RunResult {
                    job_name: job.name.clone(),
                    source: job.source.clone(),
                    destination: job.destination.clone(),
                    dry_run: job.logging.list_only,
                    command_line: format_command_line(&job),
                    args: build_args(&job),
                    started_at: now.clone(),
                    ended_at: now,
                    duration_secs: 0.0,
                    exit_code: 16,
                    exit_info: decode_exit_code(16),
                    cancelled: cancel_worker.is_cancelled(),
                    summary: RunSummary::default(),
                    file_actions: Vec::new(),
                    errors: vec![err_msg.clone()],
                    log_lines: vec![err_msg],
                    log_sha256: digest,
                };
                let _ = tx.send(RunnerEvent::Finished(Box::new(fallback)));
            }
        }
    });

    (rx, cancel)
}

/// Executes a Robocopy job synchronously, returning the parsed [`RunResult`].
pub fn run_job_blocking(job: &RobocopyJob, binary_override: Option<&Path>, cancel: &CancelToken) -> Result<RunResult> {
    execute_job_internal(job, binary_override, cancel, |_| {})
}

fn resolve_binary(override_path: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = override_path {
        return Ok(p.to_path_buf());
    }
    find_robocopy_executable()
        .ok_or_else(|| anyhow!("Microsoft Robocopy executable (robocopy.exe) was not found on this host OS."))
}

fn build_process_command(binary: &Path, args: &[String]) -> Command {
    let mut cmd = Command::new(binary);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn execute_job_internal(
    job: &RobocopyJob,
    binary_override: Option<&Path>,
    cancel: &CancelToken,
    mut emit: impl FnMut(RunnerEvent),
) -> Result<RunResult> {
    let binary = resolve_binary(binary_override)?;
    let args = build_args(job);
    let command_line = format_command_line(job);

    let started_at = Utc::now().to_rfc3339();
    let timer = Instant::now();
    emit(RunnerEvent::Started {
        command_line: command_line.clone(),
    });

    let mut cmd = build_process_command(&binary, &args);
    let mut child: Child = cmd
        .spawn()
        .with_context(|| format!("Failed to launch {}", binary.display()))?;

    let pid = child.id();
    if let Ok(mut g) = cancel.child_pid.lock() {
        *g = Some(pid);
    }

    // Store a platform killer closure so CancelToken::cancel can terminate the running child
    if let Ok(mut k) = cancel.killer.lock() {
        *k = Some(Box::new(move || {
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x0800_0000;
                let _ = Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/T", "/F"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .output();
            }
            #[cfg(not(windows))]
            {
                let _ = Command::new("kill").args(["-TERM", &pid.to_string()]).output();
            }
        }));
    }
    if cancel.is_cancelled() {
        cancel.cancel();
    }

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let (line_tx, line_rx) = mpsc::channel::<(bool, String)>();

    let stdout_handle = stdout.map(|out| {
        let tx = line_tx.clone();
        std::thread::spawn(move || stream_lines(out, false, tx))
    });
    let stderr_handle = stderr.map(|err| {
        let tx = line_tx.clone();
        std::thread::spawn(move || stream_lines(err, true, tx))
    });
    drop(line_tx);

    let mut parser = RobocopyOutputParser::new();
    let mut file_actions = Vec::new();
    let mut errors = Vec::new();
    let mut log_lines = Vec::new();
    let mut hasher = Sha256::new();

    for (is_err, line) in line_rx {
        if cancel.is_cancelled() {
            let _ = child.kill();
        }
        hasher.update(line.as_bytes());
        hasher.update(b"\n");

        if is_err {
            if !line.trim().is_empty() {
                errors.push(line.clone());
                log_lines.push(format!("[stderr] {line}"));
                emit(RunnerEvent::Error(line));
            }
            continue;
        }

        let outcome = parser.feed_line(&line);
        match outcome {
            ParsedLineOutcome::Progress(pct) => {
                emit(RunnerEvent::Progress {
                    percent: pct,
                    current_file: parser.current_file.clone(),
                });
                continue;
            }
            ParsedLineOutcome::FileAction(rec) => {
                log_lines.push(line);
                file_actions.push(rec.clone());
                emit(RunnerEvent::FileAction(rec));
            }
            ParsedLineOutcome::Error(err_txt) => {
                log_lines.push(line);
                errors.push(err_txt.clone());
                emit(RunnerEvent::Error(err_txt));
            }
            ParsedLineOutcome::SummaryUpdated => {
                log_lines.push(line);
                emit(RunnerEvent::Summary(parser.summary.clone()));
            }
            _ => {
                log_lines.push(line.clone());
                emit(RunnerEvent::LogLine(line));
            }
        }
    }

    if let Some(h) = stdout_handle {
        let _ = h.join();
    }
    if let Some(h) = stderr_handle {
        let _ = h.join();
    }

    let status = child.wait()?;
    if let Ok(mut k) = cancel.killer.lock() {
        *k = None;
    }
    if let Ok(mut g) = cancel.child_pid.lock() {
        *g = None;
    }

    let cancelled = cancel.is_cancelled();
    let exit_code = if cancelled { -1 } else { status.code().unwrap_or(16) };
    let exit_info = decode_exit_code(exit_code);
    let ended_at = Utc::now().to_rfc3339();
    let duration_secs = timer.elapsed().as_secs_f64();
    let log_sha256 = robocopy2rust_core::format::encode_hex_lower(&hasher.finalize());

    Ok(RunResult {
        job_name: job.name.clone(),
        source: job.source.clone(),
        destination: job.destination.clone(),
        dry_run: job.logging.list_only,
        command_line,
        args,
        started_at,
        ended_at,
        duration_secs,
        exit_code,
        exit_info,
        cancelled,
        summary: parser.summary,
        file_actions,
        errors,
        log_lines,
        log_sha256,
    })
}

/// Reads an arbitrary byte stream and emits lines split on either `\r` or `\n` (collapsing `\r\n`)
/// so Robocopy's in-place percentage updates stream immediately without waiting for file completion.
fn stream_lines<R: Read>(reader: R, is_err: bool, tx: mpsc::Sender<(bool, String)>) {
    let mut buf_reader = BufReader::new(reader);
    let mut chunk = [0u8; 4096];
    let mut line_buf: Vec<u8> = Vec::with_capacity(512);
    let mut last_was_cr = false;

    loop {
        match buf_reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                for &b in &chunk[..n] {
                    if b == b'\r' {
                        let s = String::from_utf8_lossy(&line_buf).into_owned();
                        line_buf.clear();
                        let _ = tx.send((is_err, s));
                        last_was_cr = true;
                    } else if b == b'\n' {
                        if last_was_cr && line_buf.is_empty() {
                            last_was_cr = false;
                            continue;
                        }
                        last_was_cr = false;
                        let s = String::from_utf8_lossy(&line_buf).into_owned();
                        line_buf.clear();
                        let _ = tx.send((is_err, s));
                    } else {
                        last_was_cr = false;
                        line_buf.push(b);
                    }
                }
            }
            Err(_) => break,
        }
    }

    if !line_buf.is_empty() {
        let s = String::from_utf8_lossy(&line_buf).into_owned();
        let _ = tx.send((is_err, s));
    }
}

#[cfg(all(test, windows))]
mod windows_integration_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn executes_real_robocopy_copy_sync_dryrun_and_mirror() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        std::fs::write(src_dir.path().join("alpha.txt"), b"alpha payload").unwrap();
        std::fs::write(src_dir.path().join("skip.tmp"), b"ignore me").unwrap();
        let sub = src_dir.path().join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("beta.txt"), b"beta payload").unwrap();
        let excluded_dir = src_dir.path().join("ignored_dir");
        std::fs::create_dir_all(&excluded_dir).unwrap();
        std::fs::write(excluded_dir.join("secret.txt"), b"secret").unwrap();

        // 1. Dry-run (/L) must NOT copy files
        let mut dry_job = RobocopyJob {
            name: "Integration Dry-Run".to_string(),
            source: src_dir.path().display().to_string(),
            destination: dst_dir.path().display().to_string(),
            ..Default::default()
        };
        dry_job.selection.exclude_files = vec!["*.tmp".to_string()];
        dry_job.selection.exclude_dirs = vec!["ignored_dir".to_string()];
        dry_job.retry.retries = Some(1);
        dry_job.retry.wait_seconds = Some(1);
        dry_job = dry_job.with_list_only();

        let dry_res = run_job_blocking(&dry_job, None, &CancelToken::new()).unwrap();
        assert_eq!(dry_res.exit_code, 1, "dry-run should report code 1 (files would copy)");
        assert!(
            !dst_dir.path().join("alpha.txt").exists(),
            "dry-run must not mutate dest"
        );
        assert_eq!(dry_res.summary.files.copied, 2);

        // 2. Real copy (code 1)
        let mut copy_job = dry_job.clone();
        copy_job.logging.list_only = false;
        let copy_res = run_job_blocking(&copy_job, None, &CancelToken::new()).unwrap();
        assert_eq!(copy_res.exit_code, 1);
        assert!(copy_res.exit_info.is_success);
        assert!(dst_dir.path().join("alpha.txt").is_file());
        assert!(dst_dir.path().join("sub").join("beta.txt").is_file());
        assert!(!dst_dir.path().join("skip.tmp").exists());
        assert!(!dst_dir.path().join("ignored_dir").exists());
        assert_eq!(copy_res.summary.files.copied, 2);

        // 3. Re-run synced (code 0)
        let sync_res = run_job_blocking(&copy_job, None, &CancelToken::new()).unwrap();
        assert_eq!(sync_res.exit_code, 0);
        assert_eq!(sync_res.summary.files.copied, 0);

        // 4. Extra file detected (code 2) then purged via /MIR (code 2)
        std::fs::write(dst_dir.path().join("orphan.txt"), b"orphan").unwrap();
        let extra_res = run_job_blocking(&copy_job, None, &CancelToken::new()).unwrap();
        assert_eq!(extra_res.exit_code, 2);
        assert_eq!(extra_res.summary.files.extras, 1);

        let mut mirror_job = copy_job.clone();
        mirror_job.copy.mirror = true;
        let mirror_res = run_job_blocking(&mirror_job, None, &CancelToken::new()).unwrap();
        assert!(mirror_res.exit_info.is_success);
        assert!(
            !dst_dir.path().join("orphan.txt").exists(),
            "/MIR must purge orphan file"
        );
    }
}
