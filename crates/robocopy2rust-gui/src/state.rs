use chrono::Utc;
use eframe::egui;
use robocopy2rust_core::{
    JobPreset, ProfileStore, RobocopyJob, RunHistoryEntry, SavedProfile, ScheduleSpec, ValidationFinding,
    ValidationSeverity, builtin_presets, format_command_line, parse_command_line, parse_rcj, validate_job,
};
use robocopy2rust_report::{ReportFormat, write_report};
use robocopy2rust_runner::{
    CancelToken, EnvironmentInfo, FileActionRecord, RunResult, RunSummary, RunnerEvent, detect_environment,
    spawn_job_runner,
};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

pub const MAX_LOG_BUFFER_LINES: usize = 50_000;

#[derive(Debug, Clone, Default)]
pub struct LaunchArgs {
    pub source: Option<String>,
    pub dest: Option<String>,
    pub rcj: Option<PathBuf>,
    pub profile: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceTab {
    Designer,
    CopyTransfer,
    FileSelection,
    PerformanceRetry,
    LoggingJobs,
    LiveConsole,
    BatchQueue,
    HistoryReports,
}

impl WorkspaceTab {
    pub fn title(self) -> &'static str {
        match self {
            Self::Designer => "Job Designer",
            Self::CopyTransfer => "Copy & Transfer",
            Self::FileSelection => "File Selection",
            Self::PerformanceRetry => "Throttle & Retry",
            Self::LoggingJobs => "Logging & Jobs",
            Self::LiveConsole => "Live Console & Diff",
            Self::BatchQueue => "Batch Queue",
            Self::HistoryReports => "History & Reports",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Designer,
            Self::CopyTransfer,
            Self::FileSelection,
            Self::PerformanceRetry,
            Self::LoggingJobs,
            Self::LiveConsole,
            Self::BatchQueue,
            Self::HistoryReports,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveModal {
    None,
    ConfirmDestructive,
    ImportCommand,
    SaveProfile,
    TaskScheduler,
    ExitCodeReference,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionFilter {
    All,
    CopiedNew,
    Extra,
    Skipped,
    Errors,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleSubTab {
    FileActions,
    RawLog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchItemStatus {
    Pending,
    Running,
    Succeeded(i32),
    Failed(i32),
}

#[derive(Debug, Clone)]
pub struct BatchQueueItem {
    pub id: u64,
    pub job: RobocopyJob,
    pub status: BatchItemStatus,
    pub summary_note: String,
}

pub struct AppState {
    pub env: EnvironmentInfo,
    pub job: RobocopyJob,

    // Synced text inputs for multi-token lists
    pub file_specs_text: String,
    pub exclude_files_text: String,
    pub exclude_dirs_text: String,
    pub extra_args_text: String,

    // Derived diagnostics & command preview cache
    pub cached_command: String,
    pub findings: Vec<ValidationFinding>,

    // Navigation & layout
    pub active_tab: WorkspaceTab,
    pub active_modal: ActiveModal,
    pub show_console_tree: bool,
    pub show_command_bar: bool,

    // Presets & persistence
    pub presets: Vec<JobPreset>,
    pub store: ProfileStore,
    pub profiles: Vec<SavedProfile>,
    pub run_history: Vec<RunHistoryEntry>,
    pub selected_history_idx: Option<usize>,

    // Active runner state
    pub is_running: bool,
    pub runner_rx: Option<Receiver<RunnerEvent>>,
    pub cancel_token: Option<CancelToken>,
    pub live_progress_pct: f32,
    pub live_current_file: Option<String>,
    pub live_summary: RunSummary,
    pub live_log_lines: VecDeque<String>,
    pub live_file_actions: Vec<FileActionRecord>,
    pub live_errors: Vec<String>,
    pub last_result: Option<RunResult>,

    // Live Console view filters
    pub console_subtab: ConsoleSubTab,
    pub action_filter: ActionFilter,
    pub action_search: String,
    pub log_search: String,
    pub auto_scroll_log: bool,

    // Batch Queue state
    pub batch_items: Vec<BatchQueueItem>,
    pub next_batch_id: u64,
    pub batch_running: bool,
    pub batch_dry_run_mode: bool,
    pub batch_stop_on_error: bool,
    pub active_batch_index: Option<usize>,

    // Modals state
    pub pending_destructive_job: Option<RobocopyJob>,
    pub import_cmd_text: String,
    pub import_warnings: Vec<String>,
    pub save_profile_name: String,
    pub save_profile_desc: String,
    pub task_sched_name: String,
    pub task_sched_target: String,
    pub task_sched_spec: ScheduleSpec,
    pub exit_code_calc_input: i32,

    pub status_message: String,
}

impl AppState {
    pub fn new(launch: LaunchArgs) -> Self {
        let env = detect_environment();
        let presets = builtin_presets();
        let store = ProfileStore::default_store();
        let profiles = store.load_profiles().unwrap_or_default();
        let run_history = store.load_history().unwrap_or_default();

        let mut job = presets.first().map(|p| p.job.clone()).unwrap_or_default();
        let mut status_msg = if env.robocopy_available {
            "Ready — select Source & Destination or choose a preset in the Console Tree.".to_string()
        } else {
            "Warning: robocopy.exe not detected on this OS (command builder & parser remain active).".to_string()
        };

        if let Some(rcj_path) = &launch.rcj {
            match std::fs::read(rcj_path)
                .map_err(anyhow::Error::from)
                .and_then(|b| parse_rcj(&b))
            {
                Ok((parsed, _)) => {
                    job = parsed;
                    status_msg = format!("Loaded .rcj job file: {}", rcj_path.display());
                }
                Err(e) => {
                    status_msg = format!("Failed to load .rcj {}: {e}", rcj_path.display());
                }
            }
        } else if let Some(prof_name) = &launch.profile
            && let Ok(Some(found)) = store.find_profile(prof_name)
        {
            job = found.job;
            status_msg = format!("Loaded profile '{}'.", found.name);
        }

        if let Some(src) = launch.source {
            job.source = src;
        }
        if let Some(dst) = launch.dest {
            job.destination = dst;
        }

        let mut state = Self {
            env,
            file_specs_text: join_tokens(&job.file_specs),
            exclude_files_text: join_tokens(&job.selection.exclude_files),
            exclude_dirs_text: join_tokens(&job.selection.exclude_dirs),
            extra_args_text: join_tokens(&job.extra_args),
            cached_command: String::new(),
            findings: Vec::new(),
            job,

            active_tab: WorkspaceTab::Designer,
            active_modal: ActiveModal::None,
            show_console_tree: true,
            show_command_bar: true,

            presets,
            store,
            profiles,
            run_history,
            selected_history_idx: None,

            is_running: false,
            runner_rx: None,
            cancel_token: None,
            live_progress_pct: 0.0,
            live_current_file: None,
            live_summary: RunSummary::default(),
            live_log_lines: VecDeque::new(),
            live_file_actions: Vec::new(),
            live_errors: Vec::new(),
            last_result: None,

            console_subtab: ConsoleSubTab::FileActions,
            action_filter: ActionFilter::All,
            action_search: String::new(),
            log_search: String::new(),
            auto_scroll_log: true,

            batch_items: Vec::new(),
            next_batch_id: 1,
            batch_running: false,
            batch_dry_run_mode: false,
            batch_stop_on_error: true,
            active_batch_index: None,

            pending_destructive_job: None,
            import_cmd_text: String::new(),
            import_warnings: Vec::new(),
            save_profile_name: String::new(),
            save_profile_desc: String::new(),
            task_sched_name: "Robocopy2Rust_DailySync".to_string(),
            task_sched_target: String::new(),
            task_sched_spec: ScheduleSpec::default(),
            exit_code_calc_input: 1,

            status_message: status_msg,
        };
        state.refresh_derived();
        state
    }

    pub fn refresh_derived(&mut self) {
        self.cached_command = format_command_line(&self.job);
        self.findings = validate_job(&self.job);
    }

    pub fn set_job_preserving_paths(&mut self, mut next_job: RobocopyJob) {
        if next_job.source.trim().is_empty() && !self.job.source.trim().is_empty() {
            next_job.source = self.job.source.clone();
        }
        if next_job.destination.trim().is_empty() && !self.job.destination.trim().is_empty() {
            next_job.destination = self.job.destination.clone();
        }
        self.set_job(next_job);
    }

    pub fn set_job(&mut self, next_job: RobocopyJob) {
        self.job = next_job;
        self.file_specs_text = join_tokens(&self.job.file_specs);
        self.exclude_files_text = join_tokens(&self.job.selection.exclude_files);
        self.exclude_dirs_text = join_tokens(&self.job.selection.exclude_dirs);
        self.extra_args_text = join_tokens(&self.job.extra_args);
        self.refresh_derived();
    }

    pub fn sync_text_lists_to_job(&mut self) {
        self.job.file_specs = split_list_tokens(&self.file_specs_text);
        self.job.selection.exclude_files = split_list_tokens(&self.exclude_files_text);
        self.job.selection.exclude_dirs = split_list_tokens(&self.exclude_dirs_text);
        self.job.extra_args = split_list_tokens(&self.extra_args_text);
        self.refresh_derived();
    }

    pub fn has_blocking_errors(&self) -> bool {
        self.findings.iter().any(|f| f.severity == ValidationSeverity::Error)
    }

    pub fn error_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == ValidationSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == ValidationSeverity::Warning)
            .count()
    }

    pub fn trigger_dry_run(&mut self, ctx: &egui::Context) {
        if self.is_running {
            return;
        }
        let dry = self.job.with_list_only();
        self.start_execution(dry, ctx);
    }

    pub fn trigger_execute(&mut self, ctx: &egui::Context) {
        if self.is_running {
            return;
        }
        self.refresh_derived();
        if self.has_blocking_errors() {
            self.status_message = "Cannot run: resolve red configuration errors first.".to_string();
            return;
        }
        if self.job.has_destructive_flags() {
            self.pending_destructive_job = Some(self.job.clone());
            self.active_modal = ActiveModal::ConfirmDestructive;
            return;
        }
        self.start_execution(self.job.clone(), ctx);
    }

    pub fn confirm_pending_destructive_run(&mut self, ctx: &egui::Context) {
        self.active_modal = ActiveModal::None;
        if let Some(job) = self.pending_destructive_job.take() {
            self.start_execution(job, ctx);
        }
    }

    pub fn start_execution(&mut self, job: RobocopyJob, ctx: &egui::Context) {
        let errs: Vec<_> = validate_job(&job)
            .into_iter()
            .filter(|f| f.severity == ValidationSeverity::Error)
            .collect();
        if !errs.is_empty() {
            self.status_message = format!("Validation error: {}", errs[0].message);
            return;
        }

        self.live_progress_pct = 0.0;
        self.live_current_file = None;
        self.live_summary = RunSummary::default();
        self.live_log_lines.clear();
        self.live_file_actions.clear();
        self.live_errors.clear();
        self.last_result = None;
        self.active_tab = WorkspaceTab::LiveConsole;

        let (rx, cancel) = spawn_job_runner(job.clone(), None);
        self.runner_rx = Some(rx);
        self.cancel_token = Some(cancel);
        self.is_running = true;
        self.status_message = if job.logging.list_only {
            format!("Running Dry-Run (/L): {} -> {}", job.source, job.destination)
        } else {
            format!("Running Robocopy: {} -> {}", job.source, job.destination)
        };
        ctx.request_repaint();
    }

    pub fn stop_execution(&mut self) {
        if let Some(token) = &self.cancel_token {
            token.cancel();
            self.batch_running = false;
            self.active_batch_index = None;
            self.status_message = "Cancellation requested — stopping active Robocopy process...".to_string();
        }
    }

    pub fn add_current_job_to_queue(&mut self) {
        let mut job = self.job.clone();
        if job.name.trim().is_empty() {
            job.name = format!("Queued Job #{}", self.next_batch_id);
        }
        let id = self.next_batch_id;
        self.next_batch_id += 1;
        self.batch_items.push(BatchQueueItem {
            id,
            job,
            status: BatchItemStatus::Pending,
            summary_note: "Queued".to_string(),
        });
        self.status_message = format!("Added job to Batch Queue ({} items).", self.batch_items.len());
    }

    pub fn start_batch_queue(&mut self, dry_run: bool, ctx: &egui::Context) {
        if self.is_running || self.batch_items.is_empty() {
            return;
        }
        for item in &mut self.batch_items {
            item.status = BatchItemStatus::Pending;
            item.summary_note = "Pending".to_string();
        }
        self.batch_running = true;
        self.batch_dry_run_mode = dry_run;
        self.launch_batch_item(0, ctx);
    }

    fn launch_batch_item(&mut self, idx: usize, ctx: &egui::Context) {
        if idx >= self.batch_items.len() {
            self.batch_running = false;
            self.active_batch_index = None;
            self.status_message = "Batch queue finished.".to_string();
            return;
        }
        self.active_batch_index = Some(idx);
        self.batch_items[idx].status = BatchItemStatus::Running;
        self.batch_items[idx].summary_note = "Running...".to_string();

        let job = if self.batch_dry_run_mode {
            self.batch_items[idx].job.with_list_only()
        } else {
            self.batch_items[idx].job.clone()
        };
        self.start_execution(job, ctx);
    }

    pub fn poll_runner(&mut self, ctx: &egui::Context) {
        let Some(rx) = self.runner_rx.take() else {
            return;
        };

        let mut finished_result: Option<RunResult> = None;
        while let Ok(ev) = rx.try_recv() {
            match ev {
                RunnerEvent::Started { command_line } => {
                    self.push_log_line(format!("> {command_line}"));
                }
                RunnerEvent::LogLine(line) => {
                    self.push_log_line(line);
                }
                RunnerEvent::FileAction(rec) => {
                    self.push_log_line(rec.raw_line.clone());
                    self.live_current_file = Some(rec.path.clone());
                    self.live_file_actions.push(rec);
                }
                RunnerEvent::Progress { percent, current_file } => {
                    self.live_progress_pct = percent;
                    if current_file.is_some() {
                        self.live_current_file = current_file;
                    }
                }
                RunnerEvent::Error(err) => {
                    self.push_log_line(format!("[ERROR] {err}"));
                    self.live_errors.push(err);
                }
                RunnerEvent::Summary(sum) => {
                    self.live_summary = sum;
                }
                RunnerEvent::Finished(res) => {
                    finished_result = Some(*res);
                }
            }
        }

        if let Some(res) = finished_result {
            self.is_running = false;
            self.cancel_token = None;
            self.live_progress_pct = if res.exit_info.is_success {
                100.0
            } else {
                self.live_progress_pct
            };
            self.live_summary = res.summary.clone();
            self.status_message = format!(
                "{} finished: {} ({} files copied, {} failed)",
                if res.dry_run { "Dry-Run (/L)" } else { "Robocopy" },
                res.exit_info.short_label,
                res.summary.files.copied,
                res.summary.files.failed
            );

            // Persist history entry
            let hist_entry = RunHistoryEntry {
                id: format!("run-{}", Utc::now().timestamp_millis()),
                timestamp: res.ended_at.clone(),
                job_name: res.job_name.clone(),
                source: res.source.clone(),
                destination: res.destination.clone(),
                dry_run: res.dry_run,
                command_line: res.command_line.clone(),
                exit_code: res.exit_code,
                exit_label: res.exit_info.short_label.clone(),
                is_success: res.exit_info.is_success,
                duration_secs: res.duration_secs,
                files_copied: res.summary.files.copied,
                files_failed: res.summary.files.failed,
                bytes_copied: res.summary.bytes.copied,
                summary_text: res.exit_info.summary.clone(),
            };
            if let Ok(updated_hist) = self.store.append_history(hist_entry) {
                self.run_history = updated_hist;
            }

            let was_success = res.exit_info.is_success;
            let code = res.exit_code;
            let note = format!("{} files copied", res.summary.files.copied);
            self.last_result = Some(res);

            if self.batch_running
                && let Some(idx) = self.active_batch_index
            {
                if idx < self.batch_items.len() {
                    self.batch_items[idx].status = if was_success {
                        BatchItemStatus::Succeeded(code)
                    } else {
                        BatchItemStatus::Failed(code)
                    };
                    self.batch_items[idx].summary_note = format!("Code {code} | {note}");
                }
                if !was_success && self.batch_stop_on_error {
                    self.batch_running = false;
                    self.active_batch_index = None;
                    self.status_message = format!("Batch stopped at item #{} (exit code {code}).", idx + 1);
                } else if idx + 1 < self.batch_items.len() {
                    self.launch_batch_item(idx + 1, ctx);
                } else {
                    self.batch_running = false;
                    self.active_batch_index = None;
                    self.status_message = "Batch queue completed.".to_string();
                }
            }
        } else {
            self.runner_rx = Some(rx);
            ctx.request_repaint();
        }
    }

    fn push_log_line(&mut self, line: String) {
        if self.live_log_lines.len() >= MAX_LOG_BUFFER_LINES {
            self.live_log_lines.pop_front();
        }
        self.live_log_lines.push_back(line);
    }

    pub fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        // Hoisted outside ctx.input(...) to prevent egui Context RwLock deadlock on Frame 0.
        let wants_kb = ctx.wants_keyboard_input();
        let (f5, ctrl_f5, f6, esc, ctrl_shift_c) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::F5) && !i.modifiers.ctrl,
                i.key_pressed(egui::Key::F5) && i.modifiers.ctrl,
                i.key_pressed(egui::Key::F6),
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::C) && i.modifiers.ctrl && i.modifiers.shift,
            )
        });

        if esc && self.is_running {
            self.stop_execution();
            return;
        }
        if wants_kb {
            return;
        }
        if ctrl_f5 {
            self.trigger_execute(ctx);
        } else if f5 {
            self.trigger_dry_run(ctx);
        } else if f6 {
            self.add_current_job_to_queue();
        } else if ctrl_shift_c && set_clipboard_text(&self.cached_command) {
            self.status_message = "Copied Robocopy command line to clipboard.".to_string();
        }
    }

    pub fn export_report_for_last(&mut self, format: ReportFormat) {
        let Some(res) = &self.last_result else {
            self.status_message = "Run or dry-run a job first to export a report.".to_string();
            return;
        };
        export_report_with_dialog(res, format, &mut self.status_message);
    }

    pub fn open_import_modal(&mut self) {
        self.import_cmd_text = self.cached_command.clone();
        self.import_warnings.clear();
        self.active_modal = ActiveModal::ImportCommand;
    }

    pub fn apply_import_command(&mut self) {
        let parsed = parse_command_line(&self.import_cmd_text);
        self.import_warnings = parsed.warnings;
        self.set_job(parsed.job);
        self.status_message = if self.import_warnings.is_empty() {
            "Imported Robocopy command line into designer.".to_string()
        } else {
            format!("Imported command line with {} notice(s).", self.import_warnings.len())
        };
        self.active_modal = ActiveModal::None;
    }

    pub fn open_save_profile_modal(&mut self) {
        self.save_profile_name = self.job.name.clone();
        self.save_profile_desc = self.job.description.clone();
        self.active_modal = ActiveModal::SaveProfile;
    }

    pub fn open_task_scheduler_modal(&mut self) {
        self.task_sched_target = self.cached_command.clone();
        self.active_modal = ActiveModal::TaskScheduler;
    }
}

pub fn export_report_with_dialog(res: &RunResult, format: ReportFormat, status_out: &mut String) {
    let dest = if format == ReportFormat::All {
        rfd::FileDialog::new()
            .set_title("Choose Folder for Audit Report Bundle (HTML + JSON + MD)")
            .pick_folder()
    } else {
        let ext = match format {
            ReportFormat::Html => "html",
            ReportFormat::Json => "json",
            ReportFormat::Markdown => "md",
            ReportFormat::All => "html",
        };
        rfd::FileDialog::new()
            .set_title("Save Robocopy2Rust Audit Report")
            .set_file_name(format!("robocopy2rust_report.{ext}"))
            .save_file()
    };

    if let Some(path) = dest {
        match write_report(res, format, &path) {
            Ok(paths) => {
                let joined: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
                *status_out = format!("Saved audit report: {}", joined.join(", "));
            }
            Err(e) => {
                *status_out = format!("Report write failed: {e}");
            }
        }
    }
}

pub fn split_list_tokens(raw: &str) -> Vec<String> {
    robocopy2rust_core::tokenize_windows_cmdline(&raw.replace([',', ';', '\n', '\r'], " "))
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn join_tokens(tokens: &[String]) -> String {
    tokens
        .iter()
        .map(|t| robocopy2rust_core::quote_windows_arg(t))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn set_clipboard_text(text: &str) -> bool {
    if let Ok(mut cb) = arboard::Clipboard::new() {
        return cb.set_text(text.to_string()).is_ok();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcuts_on_default_context_do_not_deadlock() {
        let ctx = egui::Context::default();
        let mut state = AppState::new(LaunchArgs::default());
        state.handle_shortcuts(&ctx);
        assert!(!state.is_running);
    }

    #[test]
    fn splits_and_joins_exclusion_tokens_cleanly() {
        let tokens = split_list_tokens(r#"*.tmp, "My Cache"; node_modules"#);
        assert_eq!(tokens, vec!["*.tmp", "My Cache", "node_modules"]);
        assert_eq!(join_tokens(&tokens), r#"*.tmp "My Cache" node_modules"#);
    }
}
