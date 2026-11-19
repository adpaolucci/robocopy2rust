use crate::LaunchArgs;
use crate::message::{
    ActionFilter, ActiveModal, BatchItemStatus, BatchQueueItem, ConsoleSubTab, Message, WorkspaceTab,
};
use crate::views;
use chrono::Utc;
use iced::widget::{button, column, container, mouse_area, row, rule, space, stack, text, text_input};
use iced::{Alignment, Color, Element, Font, Length, Subscription, Task, Theme};
use robocopy2rust_core::{
    CopyFlags, CopyOptions, FileAttrFlags, FileSelectionOptions, JobPreset, LogFileMode, LoggingOptions, ProfileStore,
    RetryOptions, RobocopyJob, RunHistoryEntry, SavedProfile, ScheduleSpec, ThrottleOptions, ValidationFinding,
    ValidationSeverity, build_batch_script, build_powershell_script, builtin_presets, format_command_line,
    parse_command_line, parse_rcj, quote_windows_arg, serialize_rcj, tokenize_windows_cmdline, validate_job,
};
use robocopy2rust_report::{ReportFormat, write_report};
use robocopy2rust_runner::{
    CancelToken, EnvironmentInfo, FileActionRecord, RunResult, RunSummary, RunnerEvent, detect_environment,
    spawn_job_runner,
};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Duration;

pub const MAX_LOG_BUFFER_LINES: usize = 50_000;

pub struct App {
    pub env: EnvironmentInfo,
    pub job: RobocopyJob,

    // Draft text fields for numeric / multi-token inputs
    pub file_specs_text: String,
    pub exclude_files_text: String,
    pub exclude_dirs_text: String,
    pub extra_args_text: String,
    pub max_levels_text: String,
    pub threads_text: String,
    pub retries_text: String,
    pub wait_text: String,
    pub max_size_text: String,
    pub min_size_text: String,
    pub ipg_text: String,
    pub mon_text: String,
    pub mot_text: String,
    pub add_attrs_text: String,
    pub rem_attrs_text: String,
    pub inc_attrs_text: String,
    pub exc_attrs_text: String,

    // Derived cache
    pub cached_command: String,
    pub findings: Vec<ValidationFinding>,

    // Navigation & theme
    pub dark_mode: bool,
    pub active_tab: WorkspaceTab,
    pub active_modal: ActiveModal,

    // Presets & persistence
    pub presets: Vec<JobPreset>,
    pub store: ProfileStore,
    pub profiles: Vec<SavedProfile>,
    pub run_history: Vec<RunHistoryEntry>,
    pub selected_history_idx: Option<usize>,

    // Runner state
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

    // Live console controls
    pub console_subtab: ConsoleSubTab,
    pub action_filter: ActionFilter,
    pub action_search: String,
    pub log_search: String,

    // Batch Queue
    pub batch_items: Vec<BatchQueueItem>,
    pub batch_running: bool,
    pub batch_dry_run_mode: bool,
    pub batch_stop_on_error: bool,
    pub active_batch_index: Option<usize>,

    // Modals
    pub pending_destructive_job: Option<RobocopyJob>,
    pub import_cmd_text: String,
    pub save_profile_name: String,
    pub save_profile_desc: String,
    pub task_sched_name: String,
    pub task_sched_target: String,
    pub task_sched_spec: ScheduleSpec,
    pub exit_code_input: String,
    pub exit_code_val: i32,

    pub status_message: String,
}

impl App {
    pub fn new(launch: LaunchArgs) -> (Self, Task<Message>) {
        let env = detect_environment();
        let presets = builtin_presets();
        let store = ProfileStore::default_store();
        let profiles = store.load_profiles().unwrap_or_default();
        let run_history = store.load_history().unwrap_or_default();

        let mut job = presets.first().map(|p| p.job.clone()).unwrap_or_default();
        let mut status_msg = if env.robocopy_available {
            "Ready — select Source & Destination or choose a preset.".to_string()
        } else {
            "Robocopy executable not found on this OS (command builder, .rcj, & parser ready).".to_string()
        };

        if let Some(rcj_path) = &launch.rcj {
            match std::fs::read(rcj_path)
                .map_err(anyhow::Error::from)
                .and_then(|b| parse_rcj(&b))
            {
                Ok((parsed, _)) => {
                    job = parsed;
                    status_msg = format!("Loaded .rcj: {}", rcj_path.display());
                }
                Err(e) => {
                    status_msg = format!("Failed loading .rcj: {e}");
                }
            }
        } else if let Some(prof_name) = &launch.profile
            && let Ok(Some(found)) = store.find_profile(prof_name)
        {
            job = found.job;
            status_msg = format!("Loaded profile '{}'.", found.name);
        }

        if let Some(s) = launch.source {
            job.source = s;
        }
        if let Some(d) = launch.dest {
            job.destination = d;
        }

        let mut app = Self {
            env,
            file_specs_text: String::new(),
            exclude_files_text: String::new(),
            exclude_dirs_text: String::new(),
            extra_args_text: String::new(),
            max_levels_text: String::new(),
            threads_text: String::new(),
            retries_text: String::new(),
            wait_text: String::new(),
            max_size_text: String::new(),
            min_size_text: String::new(),
            ipg_text: String::new(),
            mon_text: String::new(),
            mot_text: String::new(),
            add_attrs_text: String::new(),
            rem_attrs_text: String::new(),
            inc_attrs_text: String::new(),
            exc_attrs_text: String::new(),
            cached_command: String::new(),
            findings: Vec::new(),
            job: RobocopyJob::default(),

            dark_mode: false,
            active_tab: WorkspaceTab::Designer,
            active_modal: ActiveModal::None,

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

            batch_items: Vec::new(),
            batch_running: false,
            batch_dry_run_mode: false,
            batch_stop_on_error: true,
            active_batch_index: None,

            pending_destructive_job: None,
            import_cmd_text: String::new(),
            save_profile_name: String::new(),
            save_profile_desc: String::new(),
            task_sched_name: "Robocopy2Rust_DailySync".to_string(),
            task_sched_target: String::new(),
            task_sched_spec: ScheduleSpec::default(),
            exit_code_input: "1".to_string(),
            exit_code_val: 1,

            status_message: status_msg,
        };
        app.set_job(job);
        (app, Task::none())
    }

    pub fn title(&self) -> String {
        if self.is_running {
            format!("Robocopy2Rust ({:.0}%) - {}", self.live_progress_pct, self.job.name)
        } else {
            "Robocopy2Rust Management Console (Iced Cross-Platform)".to_string()
        }
    }

    pub fn theme(&self) -> Theme {
        if self.dark_mode { Theme::Dark } else { Theme::Light }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.is_running {
            iced::time::every(Duration::from_millis(60)).map(|_| Message::PollRunner)
        } else {
            Subscription::none()
        }
    }

    pub fn set_job(&mut self, job: RobocopyJob) {
        self.job = job;
        self.file_specs_text = join_tokens(&self.job.file_specs);
        self.exclude_files_text = join_tokens(&self.job.selection.exclude_files);
        self.exclude_dirs_text = join_tokens(&self.job.selection.exclude_dirs);
        self.extra_args_text = join_tokens(&self.job.extra_args);
        self.max_levels_text = self.job.copy.max_levels.map(|v| v.to_string()).unwrap_or_default();
        self.threads_text = self.job.copy.multithreaded.map(|v| v.to_string()).unwrap_or_default();
        self.retries_text = self.job.retry.retries.map(|v| v.to_string()).unwrap_or_default();
        self.wait_text = self.job.retry.wait_seconds.map(|v| v.to_string()).unwrap_or_default();
        self.max_size_text = self
            .job
            .selection
            .max_size_bytes
            .map(|v| v.to_string())
            .unwrap_or_default();
        self.min_size_text = self
            .job
            .selection
            .min_size_bytes
            .map(|v| v.to_string())
            .unwrap_or_default();
        self.ipg_text = self
            .job
            .copy
            .inter_packet_gap_ms
            .map(|v| v.to_string())
            .unwrap_or_default();
        self.mon_text = self.job.copy.monitor_changes.map(|v| v.to_string()).unwrap_or_default();
        self.mot_text = self.job.copy.monitor_minutes.map(|v| v.to_string()).unwrap_or_default();
        self.add_attrs_text = self.job.copy.add_attributes.to_flag_string();
        self.rem_attrs_text = self.job.copy.remove_attributes.to_flag_string();
        self.inc_attrs_text = self.job.selection.include_attrs.to_flag_string();
        self.exc_attrs_text = self.job.selection.exclude_attrs.to_flag_string();
        self.refresh_derived();
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

    pub fn refresh_derived(&mut self) {
        self.cached_command = format_command_line(&self.job);
        self.findings = validate_job(&self.job);
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

    fn start_execution(&mut self, job: RobocopyJob) {
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
    }

    fn poll_runner(&mut self) {
        let Some(rx) = self.runner_rx.take() else {
            return;
        };

        let mut finished: Option<RunResult> = None;
        while let Ok(ev) = rx.try_recv() {
            match ev {
                RunnerEvent::Started { command_line } => {
                    self.push_log_line(format!("> {command_line}"));
                }
                RunnerEvent::LogLine(line) => self.push_log_line(line),
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
                RunnerEvent::Summary(sum) => self.live_summary = sum,
                RunnerEvent::Finished(res) => finished = Some(*res),
            }
        }

        if let Some(res) = finished {
            self.is_running = false;
            self.cancel_token = None;
            self.live_progress_pct = if res.exit_info.is_success {
                100.0
            } else {
                self.live_progress_pct
            };
            self.live_summary = res.summary.clone();
            self.status_message = format!(
                "{} finished: {} ({} copied, {} failed)",
                if res.dry_run { "Dry-Run (/L)" } else { "Robocopy" },
                res.exit_info.short_label,
                res.summary.files.copied,
                res.summary.files.failed
            );

            let entry = RunHistoryEntry {
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
            if let Ok(updated) = self.store.append_history(entry) {
                self.run_history = updated;
            }

            let was_ok = res.exit_info.is_success;
            let code = res.exit_code;
            let copied = res.summary.files.copied;
            self.last_result = Some(res);

            if self.batch_running
                && let Some(idx) = self.active_batch_index
            {
                if idx < self.batch_items.len() {
                    self.batch_items[idx].status = if was_ok {
                        BatchItemStatus::Succeeded(code)
                    } else {
                        BatchItemStatus::Failed(code)
                    };
                    self.batch_items[idx].summary_note = format!("Code {code} | {copied} files");
                }
                if !was_ok && self.batch_stop_on_error {
                    self.batch_running = false;
                    self.active_batch_index = None;
                    self.status_message = format!("Batch stopped at item #{} (exit code {code}).", idx + 1);
                } else if idx + 1 < self.batch_items.len() {
                    self.launch_batch_item(idx + 1);
                } else {
                    self.batch_running = false;
                    self.active_batch_index = None;
                    self.status_message = "Batch queue completed.".to_string();
                }
            }
        } else {
            self.runner_rx = Some(rx);
        }
    }

    fn launch_batch_item(&mut self, idx: usize) {
        if idx >= self.batch_items.len() {
            self.batch_running = false;
            self.active_batch_index = None;
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
        self.start_execution(job);
    }

    fn push_log_line(&mut self, line: String) {
        if self.live_log_lines.len() >= MAX_LOG_BUFFER_LINES {
            self.live_log_lines.pop_front();
        }
        self.live_log_lines.push_back(line);
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectTab(t) => self.active_tab = t,
            Message::ToggleDarkMode => self.dark_mode = !self.dark_mode,
            Message::OpenModal(m) => {
                match m {
                    ActiveModal::ImportCommand => self.import_cmd_text = self.cached_command.clone(),
                    ActiveModal::SaveProfile => {
                        self.save_profile_name = self.job.name.clone();
                        self.save_profile_desc = self.job.description.clone();
                    }
                    ActiveModal::TaskScheduler => self.task_sched_target = self.cached_command.clone(),
                    _ => {}
                }
                self.active_modal = m;
            }
            Message::CloseModal => {
                self.active_modal = ActiveModal::None;
                self.pending_destructive_job = None;
            }

            Message::SourceChanged(s) => {
                self.job.source = s;
                self.refresh_derived();
            }
            Message::DestChanged(d) => {
                self.job.destination = d;
                self.refresh_derived();
            }
            Message::BrowseSource => {
                if let Some(p) = pick_folder_dialog(&self.job.source, "Select Robocopy Source Folder") {
                    self.job.source = p.display().to_string();
                    self.refresh_derived();
                }
            }
            Message::BrowseDest => {
                if let Some(p) = pick_folder_dialog(&self.job.destination, "Select Robocopy Destination Folder") {
                    self.job.destination = p.display().to_string();
                    self.refresh_derived();
                }
            }
            Message::SwapPaths => {
                std::mem::swap(&mut self.job.source, &mut self.job.destination);
                self.refresh_derived();
            }
            Message::FileSpecsChanged(v) => {
                self.file_specs_text = v;
                self.job.file_specs = split_tokens(&self.file_specs_text);
                self.refresh_derived();
            }

            Message::TriggerDryRun => {
                if !self.is_running {
                    let dry = self.job.with_list_only();
                    self.start_execution(dry);
                }
            }
            Message::TriggerExecute => {
                if !self.is_running {
                    self.refresh_derived();
                    if self.error_count() > 0 {
                        self.status_message = "Resolve red configuration errors before running.".to_string();
                    } else if self.job.has_destructive_flags() {
                        self.pending_destructive_job = Some(self.job.clone());
                        self.active_modal = ActiveModal::ConfirmDestructive;
                    } else {
                        self.start_execution(self.job.clone());
                    }
                }
            }
            Message::ConfirmDestructiveExecute => {
                self.active_modal = ActiveModal::None;
                if let Some(j) = self.pending_destructive_job.take() {
                    self.start_execution(j);
                }
            }
            Message::DryRunPendingInstead => {
                self.active_modal = ActiveModal::None;
                if let Some(j) = self.pending_destructive_job.take() {
                    self.start_execution(j.with_list_only());
                } else {
                    let dry = self.job.with_list_only();
                    self.start_execution(dry);
                }
            }
            Message::StopExecution => {
                if let Some(tok) = &self.cancel_token {
                    tok.cancel();
                    self.batch_running = false;
                    self.active_batch_index = None;
                    self.status_message = "Cancellation requested — stopping active process...".to_string();
                }
            }
            Message::PollRunner => self.poll_runner(),

            Message::ApplyPreset(idx) => {
                if let Some(p) = self.presets.get(idx).cloned() {
                    self.set_job_preserving_paths(p.job);
                    self.status_message = format!("Applied preset '{}' (paths preserved).", p.name);
                }
            }
            Message::LoadProfile(idx) => {
                if let Some(p) = self.profiles.get(idx).cloned() {
                    self.set_job(p.job);
                    self.status_message = format!("Loaded profile '{}'.", p.name);
                }
            }
            Message::DeleteProfile(id) => {
                if let Ok((_, updated)) = self.store.delete_profile(&id) {
                    self.profiles = updated;
                    self.status_message = "Deleted saved profile.".to_string();
                }
            }
            Message::SaveProfileNameChanged(v) => self.save_profile_name = v,
            Message::SaveProfileDescChanged(v) => self.save_profile_desc = v,
            Message::ConfirmSaveProfile => {
                let name = self.save_profile_name.trim().to_string();
                if !name.is_empty() {
                    let desc = self.save_profile_desc.clone();
                    if let Ok(updated) = self.store.upsert_profile(&name, &desc, self.job.clone()) {
                        self.profiles = updated;
                        self.job.name = name.clone();
                        self.job.description = desc;
                        self.status_message = format!("Saved profile '{name}'.");
                    }
                    self.active_modal = ActiveModal::None;
                }
            }
            Message::ResetDefaultJob => {
                self.set_job(RobocopyJob::default());
                self.status_message = "Reset to clean default job.".to_string();
            }

            Message::JobNameChanged(v) => self.job.name = v,
            Message::JobDescChanged(v) => self.job.description = v,
            Message::ImportCmdTextChanged(v) => self.import_cmd_text = v,
            Message::ApplyImportCmd => {
                let parsed = parse_command_line(&self.import_cmd_text);
                self.set_job(parsed.job);
                self.status_message = if parsed.warnings.is_empty() {
                    "Imported Robocopy command line into designer.".to_string()
                } else {
                    format!("Imported command with {} notice(s).", parsed.warnings.len())
                };
                self.active_modal = ActiveModal::None;
            }
            Message::ImportRcjFile => {
                if let Some(path) = rfd::FileDialog::new().add_filter("Robocopy Job", &["rcj"]).pick_file() {
                    match std::fs::read(&path)
                        .map_err(anyhow::Error::from)
                        .and_then(|b| parse_rcj(&b))
                    {
                        Ok((j, w)) => {
                            self.set_job(j);
                            self.status_message = if w.is_empty() {
                                format!("Imported .rcj: {}", path.display())
                            } else {
                                format!("Imported .rcj with {} warning(s).", w.len())
                            };
                        }
                        Err(e) => self.status_message = format!("Failed importing .rcj: {e}"),
                    }
                }
            }
            Message::ExportRcjFile => {
                save_text_with_dialog(
                    "Export .rcj Job File",
                    "robocopy_job.rcj",
                    &[("Robocopy Job", &["rcj"])],
                    &serialize_rcj(&self.job),
                    &mut self.status_message,
                );
            }
            Message::ExportBatchFile => {
                save_text_with_dialog(
                    "Export Windows .bat Script",
                    "robocopy_job.bat",
                    &[("Batch Script", &["bat", "cmd"])],
                    &build_batch_script(&self.job),
                    &mut self.status_message,
                );
            }
            Message::ExportPs1File => {
                save_text_with_dialog(
                    "Export PowerShell .ps1 Script",
                    "robocopy_job.ps1",
                    &[("PowerShell Script", &["ps1"])],
                    &build_powershell_script(&self.job),
                    &mut self.status_message,
                );
            }
            Message::CopyCommandLine => {
                if set_clipboard_text(&self.cached_command) {
                    self.status_message = "Copied Robocopy command line to clipboard.".to_string();
                }
            }
            Message::CopySchtasksCmd => {
                let cmd = robocopy2rust_core::build_schtasks_command(
                    &self.task_sched_name,
                    &self.task_sched_target,
                    &self.task_sched_spec,
                );
                if set_clipboard_text(&cmd) {
                    self.status_message = "Copied schtasks command to clipboard.".to_string();
                }
            }
            Message::TaskNameChanged(v) => self.task_sched_name = v,
            Message::TaskCadenceChanged(c) => self.task_sched_spec.cadence = c.0,
            Message::TaskStartTimeChanged(v) => self.task_sched_spec.start_time = v,
            Message::TaskDaysChanged(v) => {
                self.task_sched_spec.days = if v.trim().is_empty() { None } else { Some(v) };
            }
            Message::TaskRunHighestToggled(b) => self.task_sched_spec.run_highest = b,
            Message::TaskTargetChanged(v) => self.task_sched_target = v,
            Message::ExitCodeInputChanged(v) => {
                if let Ok(n) = v.trim().parse::<i32>() {
                    self.exit_code_val = n.clamp(0, 31);
                }
                self.exit_code_input = v;
            }
            Message::SelectQuickExitCode(n) => {
                self.exit_code_val = n;
                self.exit_code_input = n.to_string();
            }

            // Copy & Transfer switches
            Message::ResetCopyDefaults => {
                self.job.copy = CopyOptions::default();
                self.set_job(self.job.clone());
            }
            Message::ToggleSubdirsS(b) => {
                self.job.copy.subdirectories_non_empty = b;
                if b {
                    self.job.copy.subdirectories_including_empty = false;
                    self.job.copy.mirror = false;
                }
                self.refresh_derived();
            }
            Message::ToggleSubdirsE(b) => {
                self.job.copy.subdirectories_including_empty = b;
                if b {
                    self.job.copy.subdirectories_non_empty = false;
                }
                self.refresh_derived();
            }
            Message::TogglePurge(b) => {
                self.job.copy.purge = b;
                self.refresh_derived();
            }
            Message::ToggleMirror(b) => {
                self.job.copy.mirror = b;
                if b {
                    self.job.copy.subdirectories_including_empty = true;
                    self.job.copy.subdirectories_non_empty = false;
                    self.job.copy.purge = true;
                }
                self.refresh_derived();
            }
            Message::ToggleMoveFiles(b) => {
                self.job.copy.move_files = b;
                self.refresh_derived();
            }
            Message::ToggleMoveAll(b) => {
                self.job.copy.move_files_and_dirs = b;
                self.refresh_derived();
            }
            Message::MaxLevelsChanged(v) => {
                self.max_levels_text = v;
                self.job.copy.max_levels = opt_parse_u32(&self.max_levels_text);
                self.refresh_derived();
            }
            Message::ThreadsChanged(v) => {
                self.threads_text = v;
                self.job.copy.multithreaded = opt_parse_u16(&self.threads_text);
                self.refresh_derived();
            }
            Message::ToggleRestartableZ(b) => {
                self.job.copy.restartable = b;
                self.refresh_derived();
            }
            Message::ToggleBackupB(b) => {
                self.job.copy.backup_mode = b;
                self.refresh_derived();
            }
            Message::ToggleRestartBackupZB(b) => {
                self.job.copy.restartable_or_backup = b;
                self.refresh_derived();
            }
            Message::ToggleUnbufferedJ(b) => {
                self.job.copy.unbuffered_io = b;
                self.refresh_derived();
            }
            Message::ToggleEfsRaw(b) => {
                self.job.copy.efs_raw = b;
                self.refresh_derived();
            }
            Message::ToggleCompress(b) => {
                self.job.copy.network_compress = b;
                self.refresh_derived();
            }
            Message::ToggleNoOffload(b) => {
                self.job.copy.no_offload = b;
                self.refresh_derived();
            }
            Message::ToggleNoClone(b) => {
                self.job.copy.no_clone = b;
                self.refresh_derived();
            }
            Message::SparseModeChanged(c) => {
                self.job.copy.sparse_mode = c.0;
                self.refresh_derived();
            }
            Message::CopyFlagChanged(ch, b) => {
                let f = &mut self.job.copy.copy_flags;
                match ch {
                    'D' => f.data = b,
                    'A' => f.attributes = b,
                    'T' => f.timestamps = b,
                    'S' => f.security = b,
                    'O' => f.owner = b,
                    'U' => f.auditing = b,
                    'X' => f.skip_alt_data_streams = b,
                    _ => {}
                }
                self.refresh_derived();
            }
            Message::ToggleNoCopy(b) => {
                self.job.copy.no_copy = b;
                self.refresh_derived();
            }
            Message::CopyPresetDat => {
                self.job.copy.copy_flags = CopyFlags::default();
                self.job.copy.no_copy = false;
                self.refresh_derived();
            }
            Message::CopyPresetSec => {
                self.job.copy.copy_flags = CopyFlags::sec();
                self.job.copy.no_copy = false;
                self.refresh_derived();
            }
            Message::CopyPresetAll => {
                self.job.copy.copy_flags = CopyFlags::copy_all();
                self.job.copy.no_copy = false;
                self.refresh_derived();
            }
            Message::ToggleSecFix(b) => {
                self.job.copy.sec_fix = b;
                self.refresh_derived();
            }
            Message::ToggleTimFix(b) => {
                self.job.copy.tim_fix = b;
                self.refresh_derived();
            }
            Message::DirCopyFlagChanged(ch, b) => {
                let f = &mut self.job.copy.dir_copy_flags;
                match ch {
                    'D' => f.data = b,
                    'A' => f.attributes = b,
                    'T' => f.timestamps = b,
                    'E' => f.extended_attributes = b,
                    'X' => f.skip_alt_data_streams = b,
                    _ => {}
                }
                self.refresh_derived();
            }
            Message::ToggleNoDirCopy(b) => {
                self.job.copy.no_dir_copy = b;
                self.refresh_derived();
            }
            Message::ToggleCreateTree(b) => {
                self.job.copy.create_tree_only = b;
                self.refresh_derived();
            }
            Message::ToggleFatNames(b) => {
                self.job.copy.fat_names = b;
                self.refresh_derived();
            }
            Message::ToggleDisableLongPaths(b) => {
                self.job.copy.disable_long_paths = b;
                self.refresh_derived();
            }
            Message::ToggleCopyJunctions(b) => {
                self.job.copy.copy_junctions = b;
                self.refresh_derived();
            }
            Message::ToggleCopySymlinks(b) => {
                self.job.copy.copy_symlinks = b;
                self.refresh_derived();
            }
            Message::AddAttrsChanged(v) => {
                self.add_attrs_text = v;
                if let Ok(f) = FileAttrFlags::from_flag_string(&self.add_attrs_text) {
                    self.job.copy.add_attributes = f;
                }
                self.refresh_derived();
            }
            Message::RemoveAttrsChanged(v) => {
                self.rem_attrs_text = v;
                if let Ok(f) = FileAttrFlags::from_flag_string(&self.rem_attrs_text) {
                    self.job.copy.remove_attributes = f;
                }
                self.refresh_derived();
            }

            // Selection Switches
            Message::ResetSelectionDefaults => {
                self.job.selection = FileSelectionOptions::default();
                self.set_job(self.job.clone());
            }
            Message::ExcludeFilesChanged(v) => {
                self.exclude_files_text = v;
                self.job.selection.exclude_files = split_tokens(&self.exclude_files_text);
                self.refresh_derived();
            }
            Message::ExcludeDirsChanged(v) => {
                self.exclude_dirs_text = v;
                self.job.selection.exclude_dirs = split_tokens(&self.exclude_dirs_text);
                self.refresh_derived();
            }
            Message::ToggleExcludeJunctions(b) => {
                self.job.selection.exclude_junctions = b;
                self.refresh_derived();
            }
            Message::ToggleExcludeDirJunctions(b) => {
                self.job.selection.exclude_dir_junctions = b;
                self.refresh_derived();
            }
            Message::ToggleExcludeFileSymlinks(b) => {
                self.job.selection.exclude_file_symlinks = b;
                self.refresh_derived();
            }
            Message::ToggleExcludeChanged(b) => {
                self.job.selection.exclude_changed = b;
                self.refresh_derived();
            }
            Message::ToggleExcludeNewer(b) => {
                self.job.selection.exclude_newer = b;
                self.refresh_derived();
            }
            Message::ToggleExcludeOlder(b) => {
                self.job.selection.exclude_older = b;
                self.refresh_derived();
            }
            Message::ToggleExcludeExtra(b) => {
                self.job.selection.exclude_extra = b;
                self.refresh_derived();
            }
            Message::ToggleExcludeLonely(b) => {
                self.job.selection.exclude_lonely = b;
                self.refresh_derived();
            }
            Message::ToggleIncludeSame(b) => {
                self.job.selection.include_same = b;
                self.refresh_derived();
            }
            Message::ToggleIncludeTweaked(b) => {
                self.job.selection.include_tweaked = b;
                self.refresh_derived();
            }
            Message::ToggleIncludeModified(b) => {
                self.job.selection.include_modified = b;
                self.refresh_derived();
            }
            Message::ToggleFatFileTimes(b) => {
                self.job.selection.fat_file_times = b;
                self.refresh_derived();
            }
            Message::ToggleCompensateDst(b) => {
                self.job.selection.compensate_dst = b;
                self.refresh_derived();
            }
            Message::ToggleArchiveOnly(b) => {
                self.job.selection.archive_only = b;
                self.refresh_derived();
            }
            Message::ToggleArchiveReset(b) => {
                self.job.selection.archive_and_reset = b;
                self.refresh_derived();
            }
            Message::IncludeAttrsChanged(v) => {
                self.inc_attrs_text = v;
                if let Ok(f) = FileAttrFlags::from_flag_string(&self.inc_attrs_text) {
                    self.job.selection.include_attrs = f;
                }
                self.refresh_derived();
            }
            Message::ExcludeAttrsChanged(v) => {
                self.exc_attrs_text = v;
                if let Ok(f) = FileAttrFlags::from_flag_string(&self.exc_attrs_text) {
                    self.job.selection.exclude_attrs = f;
                }
                self.refresh_derived();
            }
            Message::MaxSizeChanged(v) => {
                self.max_size_text = v;
                self.job.selection.max_size_bytes = opt_parse_u64(&self.max_size_text);
                self.refresh_derived();
            }
            Message::MinSizeChanged(v) => {
                self.min_size_text = v;
                self.job.selection.min_size_bytes = opt_parse_u64(&self.min_size_text);
                self.refresh_derived();
            }
            Message::MaxAgeChanged(v) => {
                self.job.selection.max_age = opt_nonempty(&v);
                self.refresh_derived();
            }
            Message::MinAgeChanged(v) => {
                self.job.selection.min_age = opt_nonempty(&v);
                self.refresh_derived();
            }
            Message::MaxLadChanged(v) => {
                self.job.selection.max_last_access = opt_nonempty(&v);
                self.refresh_derived();
            }
            Message::MinLadChanged(v) => {
                self.job.selection.min_last_access = opt_nonempty(&v);
                self.refresh_derived();
            }

            // Perf & Retry Switches
            Message::ResetRetryDefaults => {
                self.job.retry = RetryOptions::default();
                self.job.throttle = ThrottleOptions::default();
                self.set_job(self.job.clone());
            }
            Message::RetriesChanged(v) => {
                self.retries_text = v;
                self.job.retry.retries = opt_parse_u32(&self.retries_text);
                self.refresh_derived();
            }
            Message::WaitSecsChanged(v) => {
                self.wait_text = v;
                self.job.retry.wait_seconds = opt_parse_u32(&self.wait_text);
                self.refresh_derived();
            }
            Message::ToggleWaitShares(b) => {
                self.job.retry.wait_for_sharenames = b;
                self.refresh_derived();
            }
            Message::ToggleRegDefaults(b) => {
                self.job.retry.save_defaults_to_registry = b;
                self.refresh_derived();
            }
            Message::ToggleLfsm(b) => {
                self.job.retry.low_free_space_mode = b;
                self.refresh_derived();
            }
            Message::LfsmFloorChanged(v) => {
                self.job.retry.low_free_space_floor = opt_nonempty(&v);
                self.refresh_derived();
            }
            Message::IoMaxSizeChanged(v) => {
                self.job.throttle.io_max_size = opt_nonempty(&v);
                self.refresh_derived();
            }
            Message::IoRateChanged(v) => {
                self.job.throttle.io_rate = opt_nonempty(&v);
                self.refresh_derived();
            }
            Message::ThresholdChanged(v) => {
                self.job.throttle.threshold = opt_nonempty(&v);
                self.refresh_derived();
            }
            Message::IpgChanged(v) => {
                self.ipg_text = v;
                self.job.copy.inter_packet_gap_ms = opt_parse_u32(&self.ipg_text);
                self.refresh_derived();
            }
            Message::MonChangesChanged(v) => {
                self.mon_text = v;
                self.job.copy.monitor_changes = opt_parse_u32(&self.mon_text);
                self.refresh_derived();
            }
            Message::MotMinutesChanged(v) => {
                self.mot_text = v;
                self.job.copy.monitor_minutes = opt_parse_u32(&self.mot_text);
                self.refresh_derived();
            }
            Message::RunHoursChanged(v) => {
                self.job.copy.run_hours = opt_nonempty(&v);
                self.refresh_derived();
            }
            Message::TogglePerFileRunHours(b) => {
                self.job.copy.per_file_run_hours = b;
                self.refresh_derived();
            }

            // Logging & Jobs
            Message::ResetLoggingDefaults => {
                self.job.logging = LoggingOptions::default();
                self.job.job_files = Default::default();
                self.job.extra_args.clear();
                self.set_job(self.job.clone());
            }
            Message::ToggleListOnly(b) => {
                self.job.logging.list_only = b;
                self.refresh_derived();
            }
            Message::ToggleReportExtra(b) => {
                self.job.logging.report_extra = b;
                self.refresh_derived();
            }
            Message::ToggleVerbose(b) => {
                self.job.logging.verbose = b;
                self.refresh_derived();
            }
            Message::ToggleTimestamps(b) => {
                self.job.logging.include_timestamps = b;
                self.refresh_derived();
            }
            Message::ToggleFullPaths(b) => {
                self.job.logging.full_paths = b;
                self.refresh_derived();
            }
            Message::ToggleBytes(b) => {
                self.job.logging.sizes_in_bytes = b;
                self.refresh_derived();
            }
            Message::ToggleUnicodeOutput(b) => {
                self.job.logging.unicode_output = b;
                self.refresh_derived();
            }
            Message::ToggleNoSizes(b) => {
                self.job.logging.no_sizes = b;
                self.refresh_derived();
            }
            Message::ToggleNoClasses(b) => {
                self.job.logging.no_classes = b;
                self.refresh_derived();
            }
            Message::ToggleNoFileList(b) => {
                self.job.logging.no_file_list = b;
                self.refresh_derived();
            }
            Message::ToggleNoDirList(b) => {
                self.job.logging.no_dir_list = b;
                self.refresh_derived();
            }
            Message::ToggleNoProgress(b) => {
                self.job.logging.no_progress = b;
                self.refresh_derived();
            }
            Message::ToggleEta(b) => {
                self.job.logging.show_eta = b;
                self.refresh_derived();
            }
            Message::ToggleNoJobHeader(b) => {
                self.job.logging.no_job_header = b;
                self.refresh_derived();
            }
            Message::ToggleNoJobSummary(b) => {
                self.job.logging.no_job_summary = b;
                self.refresh_derived();
            }
            Message::ToggleTee(b) => {
                self.job.logging.tee = b;
                self.refresh_derived();
            }
            Message::LogModeChanged(c) => {
                self.job.logging.log_mode = c.0;
                self.refresh_derived();
            }
            Message::LogPathChanged(v) => {
                self.job.logging.log_path = opt_nonempty(&v);
                self.refresh_derived();
            }
            Message::BrowseLogFile => {
                if let Some(p) = rfd::FileDialog::new().set_file_name("robocopy.log").save_file() {
                    self.job.logging.log_path = Some(p.display().to_string());
                    if self.job.logging.log_mode == LogFileMode::None {
                        self.job.logging.log_mode = LogFileMode::Overwrite;
                    }
                    self.refresh_derived();
                }
            }
            Message::LoadJobChanged(v) => {
                self.job.job_files.load_job = opt_nonempty(&v);
                self.refresh_derived();
            }
            Message::SaveJobChanged(v) => {
                self.job.job_files.save_job = opt_nonempty(&v);
                self.refresh_derived();
            }
            Message::ToggleQuit(b) => {
                self.job.job_files.quit_after_parse = b;
                self.refresh_derived();
            }
            Message::ToggleNosd(b) => {
                self.job.job_files.no_source_dir = b;
                self.refresh_derived();
            }
            Message::ToggleNodd(b) => {
                self.job.job_files.no_dest_dir = b;
                self.refresh_derived();
            }
            Message::ExtraArgsChanged(v) => {
                self.extra_args_text = v;
                self.job.extra_args = split_tokens(&self.extra_args_text);
                self.refresh_derived();
            }

            // Live Console
            Message::SwitchConsoleSubTab(st) => self.console_subtab = st,
            Message::SetActionFilter(af) => self.action_filter = af,
            Message::ActionSearchChanged(s) => self.action_search = s,
            Message::LogSearchChanged(s) => self.log_search = s,
            Message::CopyLogToClipboard => {
                let full: String = self.live_log_lines.iter().cloned().collect::<Vec<_>>().join("\n");
                if set_clipboard_text(&full) {
                    self.status_message = "Copied console log to clipboard.".to_string();
                }
            }
            Message::ExportReport(fmt) => {
                if let Some(res) = &self.last_result {
                    let target = if fmt == ReportFormat::All {
                        rfd::FileDialog::new()
                            .set_title("Choose Folder for HTML + JSON + MD Report Bundle")
                            .pick_folder()
                    } else {
                        let ext = match fmt {
                            ReportFormat::Html => "html",
                            ReportFormat::Json => "json",
                            ReportFormat::Markdown => "md",
                            ReportFormat::All => "html",
                        };
                        rfd::FileDialog::new()
                            .set_title("Save Audit Report")
                            .set_file_name(format!("robocopy2rust_report.{ext}"))
                            .save_file()
                    };
                    if let Some(p) = target {
                        match write_report(res, fmt, &p) {
                            Ok(written) => {
                                let paths: Vec<String> = written.iter().map(|w| w.display().to_string()).collect();
                                self.status_message = format!("Saved report: {}", paths.join(", "));
                            }
                            Err(e) => self.status_message = format!("Report write failed: {e}"),
                        }
                    }
                } else {
                    self.status_message = "Run or dry-run a job first to export a report.".to_string();
                }
            }

            // Batch Queue
            Message::AddCurrentToQueue => {
                let mut job = self.job.clone();
                if job.name.trim().is_empty() {
                    job.name = format!("Queued Job #{}", self.batch_items.len() + 1);
                }
                self.batch_items.push(BatchQueueItem {
                    job,
                    status: BatchItemStatus::Pending,
                    summary_note: "Queued".to_string(),
                });
                self.status_message = format!("Added job to Batch Queue ({} items).", self.batch_items.len());
            }
            Message::ToggleStopOnError(b) => self.batch_stop_on_error = b,
            Message::StartBatchQueue(dry_run) => {
                if !self.is_running && !self.batch_items.is_empty() {
                    for item in &mut self.batch_items {
                        item.status = BatchItemStatus::Pending;
                        item.summary_note = "Pending".to_string();
                    }
                    self.batch_running = true;
                    self.batch_dry_run_mode = dry_run;
                    self.launch_batch_item(0);
                }
            }
            Message::MoveQueueUp(i) => {
                if i > 0 && i < self.batch_items.len() {
                    self.batch_items.swap(i, i - 1);
                }
            }
            Message::MoveQueueDown(i) => {
                if i + 1 < self.batch_items.len() {
                    self.batch_items.swap(i, i + 1);
                }
            }
            Message::LoadQueueItem(i) => {
                if let Some(item) = self.batch_items.get(i).cloned() {
                    self.set_job(item.job);
                    self.status_message = format!("Loaded queued job #{} into Designer.", i + 1);
                }
            }
            Message::DuplicateQueueItem(i) => {
                if let Some(mut item) = self.batch_items.get(i).cloned() {
                    item.status = BatchItemStatus::Pending;
                    item.summary_note = "Duplicated".to_string();
                    self.batch_items.insert(i + 1, item);
                }
            }
            Message::RemoveQueueItem(i) => {
                if !self.is_running && i < self.batch_items.len() {
                    self.batch_items.remove(i);
                }
            }
            Message::ClearBatchQueue => {
                if !self.is_running {
                    self.batch_items.clear();
                }
            }

            // History
            Message::SelectHistory(i) => self.selected_history_idx = Some(i),
            Message::ReloadHistoryCommand(i) => {
                if let Some(h) = self.run_history.get(i).cloned() {
                    let parsed = parse_command_line(&h.command_line);
                    self.set_job(parsed.job);
                    self.status_message = format!("Reloaded command from '{}'.", h.job_name);
                }
            }
            Message::ClearHistory => {
                let _ = self.store.clear_history();
                self.run_history.clear();
                self.selected_history_idx = None;
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let top_toolbar = self.view_top_toolbar();
        let sidebar = views::sidebar::view_sidebar(self);

        let center: Element<'_, Message> = match self.active_tab {
            WorkspaceTab::Designer => views::designer::view_designer(self),
            WorkspaceTab::CopyTransfer => views::options_tabs::view_copy_transfer(self),
            WorkspaceTab::FileSelection => views::options_tabs::view_file_selection(self),
            WorkspaceTab::PerformanceRetry => views::options_tabs::view_perf_retry(self),
            WorkspaceTab::LoggingJobs => views::options_tabs::view_logging_jobs(self),
            WorkspaceTab::LiveConsole => views::live_console::view_live_console(self),
            WorkspaceTab::BatchQueue => views::batch_queue::view_batch_queue(self),
            WorkspaceTab::HistoryReports => views::history_view::view_history_view(self),
        };

        let body = row![
            container(sidebar)
                .width(Length::Fixed(255.0))
                .height(Length::Fill)
                .padding(8)
                .style(container::bordered_box),
            container(center).width(Length::Fill).height(Length::Fill).padding(10)
        ]
        .spacing(8)
        .height(Length::Fill);

        let cmd_bar = self.view_command_bar();
        let status_bar = self.view_status_bar();

        let main_layout = column![
            top_toolbar,
            rule::horizontal(1),
            body,
            rule::horizontal(1),
            cmd_bar,
            rule::horizontal(1),
            status_bar,
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        if self.active_modal == ActiveModal::None {
            main_layout.into()
        } else {
            let modal_card = views::modals::view_modal(self);
            let backdrop = mouse_area(
                container(modal_card)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .style(|_| container::Style {
                        background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.48).into()),
                        ..Default::default()
                    }),
            )
            .on_press(Message::CloseModal);

            stack![main_layout, backdrop].into()
        }
    }

    fn view_top_toolbar(&self) -> Element<'_, Message> {
        let can_run = !self.is_running;
        let can_exec = can_run && self.error_count() == 0;

        let run_label = if self.job.has_destructive_flags() {
            "Run Job (!)"
        } else {
            "Run Job"
        };
        let run_btn = button(text(run_label).size(13))
            .on_press_maybe(can_exec.then_some(Message::TriggerExecute))
            .style(if self.job.has_destructive_flags() {
                button::danger
            } else {
                button::primary
            });

        let row_controls = row![
            text("Source:").size(13),
            text_input(r"C:\Source\Folder", &self.job.source)
                .on_input(Message::SourceChanged)
                .size(13)
                .width(Length::FillPortion(3)),
            button(text("...").size(13))
                .on_press(Message::BrowseSource)
                .style(button::secondary),
            button(text("<->").size(13))
                .on_press(Message::SwapPaths)
                .style(button::secondary),
            text("Dest:").size(13),
            text_input(r"D:\Backup\Target", &self.job.destination)
                .on_input(Message::DestChanged)
                .size(13)
                .width(Length::FillPortion(3)),
            button(text("...").size(13))
                .on_press(Message::BrowseDest)
                .style(button::secondary),
            text("Files:").size(13),
            text_input("*.*", &self.file_specs_text)
                .on_input(Message::FileSpecsChanged)
                .size(13)
                .width(Length::Fixed(105.0)),
            button(text("Dry-Run (/L)").size(13))
                .on_press_maybe(can_run.then_some(Message::TriggerDryRun))
                .style(button::success),
            run_btn,
            button(text("Queue +").size(13))
                .on_press_maybe(can_run.then_some(Message::AddCurrentToQueue))
                .style(button::secondary),
            button(text("Stop").size(13))
                .on_press_maybe(self.is_running.then_some(Message::StopExecution))
                .style(button::danger),
            button(text(if self.dark_mode { "Light" } else { "Dark" }).size(13))
                .on_press(Message::ToggleDarkMode)
                .style(button::secondary),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        container(row_controls).padding([8, 10]).width(Length::Fill).into()
    }

    fn view_command_bar(&self) -> Element<'_, Message> {
        let bar = row![
            text("CMD:").size(12).font(Font::MONOSPACE),
            text_input("", &self.cached_command)
                .font(Font::MONOSPACE)
                .size(12)
                .width(Length::Fill),
            button(text("Copy").size(12))
                .on_press(Message::CopyCommandLine)
                .style(button::secondary),
            button(text("Import...").size(12))
                .on_press(Message::OpenModal(ActiveModal::ImportCommand))
                .style(button::secondary),
            button(text(".rcj").size(12))
                .on_press(Message::ExportRcjFile)
                .style(button::secondary),
            button(text(".bat").size(12))
                .on_press(Message::ExportBatchFile)
                .style(button::secondary),
            button(text(".ps1").size(12))
                .on_press(Message::ExportPs1File)
                .style(button::secondary),
            button(text("Schedule...").size(12))
                .on_press(Message::OpenModal(ActiveModal::TaskScheduler))
                .style(button::secondary),
            button(text("Exit Codes").size(12))
                .on_press(Message::OpenModal(ActiveModal::ExitCodeReference))
                .style(button::secondary),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        container(bar).padding([6, 10]).width(Length::Fill).into()
    }

    fn view_status_bar(&self) -> Element<'_, Message> {
        let diag = if self.error_count() > 0 {
            format!("{} Err / {} Warn", self.error_count(), self.warning_count())
        } else if self.warning_count() > 0 {
            format!("{} Warn", self.warning_count())
        } else {
            "Config Valid".to_string()
        };

        let run_seg = if self.is_running {
            format!("RUNNING ({:.0}%)", self.live_progress_pct)
        } else if let Some(res) = &self.last_result {
            format!("Last: {}", res.exit_info.short_label)
        } else {
            "Idle".to_string()
        };

        let env_badge = if self.env.robocopy_available {
            "Iced 0.14 (wgpu + tiny-skia fallback) | Robocopy Ready"
        } else {
            "Iced 0.14 (wgpu + tiny-skia fallback) | Builder Mode"
        };

        let vsep = || container(rule::vertical(1)).height(Length::Fixed(16.0));
        let bar = row![
            text(&self.status_message).size(12).width(Length::Fill),
            vsep(),
            text(diag).size(12),
            vsep(),
            text(run_seg).size(12),
            vsep(),
            text(env_badge).size(12),
            space::horizontal().width(Length::Fixed(4.0)),
        ]
        .spacing(10)
        .height(Length::Shrink)
        .align_y(Alignment::Center);

        container(bar)
            .padding([5, 10])
            .width(Length::Fill)
            .height(Length::Shrink)
            .style(container::bordered_box)
            .into()
    }
}

fn pick_folder_dialog(current_input: &str, title: &str) -> Option<PathBuf> {
    let mut dlg = rfd::FileDialog::new().set_title(title);
    let trimmed = current_input.trim().trim_matches('"').trim();
    if !trimmed.is_empty() {
        let p = PathBuf::from(trimmed);
        if p.is_dir() {
            dlg = dlg.set_directory(&p);
        } else if let Some(parent) = p.parent()
            && parent.is_dir()
        {
            dlg = dlg.set_directory(parent);
        }
    }
    dlg.pick_folder()
}

fn save_text_with_dialog(
    title: &str,
    default_name: &str,
    filters: &[(&str, &[&str])],
    content: &str,
    status_out: &mut String,
) {
    let mut dlg = rfd::FileDialog::new().set_title(title).set_file_name(default_name);
    for (name, exts) in filters {
        dlg = dlg.add_filter(*name, exts);
    }
    if let Some(path) = dlg.save_file() {
        match std::fs::write(&path, content) {
            Ok(_) => *status_out = format!("Exported {}", path.display()),
            Err(e) => *status_out = format!("Export failed: {e}"),
        }
    }
}

pub fn split_tokens(raw: &str) -> Vec<String> {
    tokenize_windows_cmdline(&raw.replace([',', ';', '\n', '\r'], " "))
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn join_tokens(tokens: &[String]) -> String {
    tokens
        .iter()
        .map(|t| quote_windows_arg(t))
        .collect::<Vec<_>>()
        .join(" ")
}

fn opt_parse_u32(s: &str) -> Option<u32> {
    s.trim().parse().ok()
}

fn opt_parse_u16(s: &str) -> Option<u16> {
    s.trim().parse().ok()
}

fn opt_parse_u64(s: &str) -> Option<u64> {
    s.trim().parse().ok()
}

fn opt_nonempty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() { None } else { Some(t.to_string()) }
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
    fn preset_application_preserves_paths_and_destructive_guard_opens_modal() {
        let (mut app, _) = App::new(LaunchArgs {
            source: Some(r"C:\Work".to_string()),
            dest: Some(r"D:\MirrorWork".to_string()),
            ..Default::default()
        });

        let mirror_idx = app.presets.iter().position(|p| p.id == "exact-mirror").unwrap();
        let _ = app.update(Message::ApplyPreset(mirror_idx));
        assert_eq!(app.job.source, r"C:\Work");
        assert_eq!(app.job.destination, r"D:\MirrorWork");
        assert!(app.job.copy.mirror);

        // Triggering execution on a /MIR job without /L must open ConfirmDestructive instead of running directly
        let _ = app.update(Message::TriggerExecute);
        assert_eq!(app.active_modal, ActiveModal::ConfirmDestructive);
        assert!(!app.is_running);
    }

    #[test]
    fn command_import_and_batch_queue_actions_work() {
        let (mut app, _) = App::new(LaunchArgs::default());
        let _ = app.update(Message::ImportCmdTextChanged(
            r#"robocopy "C:\Src" "D:\Dst" *.rs /E /MT:16 /R:2 /W:4 /XD target"#.to_string(),
        ));
        let _ = app.update(Message::ApplyImportCmd);
        assert_eq!(app.job.source, r"C:\Src");
        assert_eq!(app.job.copy.multithreaded, Some(16));
        assert_eq!(app.job.selection.exclude_dirs, vec!["target"]);

        let _ = app.update(Message::AddCurrentToQueue);
        let _ = app.update(Message::DuplicateQueueItem(0));
        assert_eq!(app.batch_items.len(), 2);
        let _ = app.update(Message::RemoveQueueItem(0));
        assert_eq!(app.batch_items.len(), 1);
    }
}
