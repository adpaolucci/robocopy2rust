use robocopy2rust_core::{LogFileMode, RobocopyJob, ScheduleCadence, SparseMode};
use robocopy2rust_report::ReportFormat;
use std::fmt;

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
    pub job: RobocopyJob,
    pub status: BatchItemStatus,
    pub summary_note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SparseChoice(pub SparseMode);

impl SparseChoice {
    pub const ALL: &'static [Self] = &[
        Self(SparseMode::Default),
        Self(SparseMode::Enable),
        Self(SparseMode::Disable),
    ];
}

impl fmt::Display for SparseChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            SparseMode::Default => write!(f, "Default"),
            SparseMode::Enable => write!(f, "/SPARSE:Y (Preserve)"),
            SparseMode::Disable => write!(f, "/SPARSE:N (Disable)"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogModeChoice(pub LogFileMode);

impl LogModeChoice {
    pub const ALL: &'static [Self] = &[
        Self(LogFileMode::None),
        Self(LogFileMode::Overwrite),
        Self(LogFileMode::Append),
        Self(LogFileMode::UnicodeOverwrite),
        Self(LogFileMode::UnicodeAppend),
    ];
}

impl fmt::Display for LogModeChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            LogFileMode::None => write!(f, "None"),
            LogFileMode::Overwrite => write!(f, "/LOG: (Overwrite)"),
            LogFileMode::Append => write!(f, "/LOG+: (Append)"),
            LogFileMode::UnicodeOverwrite => write!(f, "/UNILOG: (Unicode)"),
            LogFileMode::UnicodeAppend => write!(f, "/UNILOG+: (Unicode Append)"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CadenceChoice(pub ScheduleCadence);

impl CadenceChoice {
    pub const ALL: &'static [Self] = &[
        Self(ScheduleCadence::Daily),
        Self(ScheduleCadence::Weekly),
        Self(ScheduleCadence::Hourly),
        Self(ScheduleCadence::OnStart),
        Self(ScheduleCadence::OnLogon),
    ];
}

impl fmt::Display for CadenceChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            ScheduleCadence::Daily => write!(f, "DAILY"),
            ScheduleCadence::Weekly => write!(f, "WEEKLY"),
            ScheduleCadence::Hourly => write!(f, "HOURLY"),
            ScheduleCadence::OnStart => write!(f, "ONSTART"),
            ScheduleCadence::OnLogon => write!(f, "ONLOGON"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    // Navigation & Theme
    SelectTab(WorkspaceTab),
    ToggleDarkMode,
    OpenModal(ActiveModal),
    CloseModal,

    // Target Bar
    SourceChanged(String),
    DestChanged(String),
    BrowseSource,
    BrowseDest,
    SwapPaths,
    FileSpecsChanged(String),

    // Execution
    TriggerDryRun,
    TriggerExecute,
    ConfirmDestructiveExecute,
    DryRunPendingInstead,
    StopExecution,
    PollRunner,

    // Presets & Profiles
    ApplyPreset(usize),
    LoadProfile(usize),
    DeleteProfile(String),
    SaveProfileNameChanged(String),
    SaveProfileDescChanged(String),
    ConfirmSaveProfile,
    ResetDefaultJob,

    // Designer & Imports/Exports
    JobNameChanged(String),
    JobDescChanged(String),
    ImportCmdTextChanged(String),
    ApplyImportCmd,
    ImportRcjFile,
    ExportRcjFile,
    ExportBatchFile,
    ExportPs1File,
    CopyCommandLine,
    CopySchtasksCmd,
    TaskNameChanged(String),
    TaskCadenceChanged(CadenceChoice),
    TaskStartTimeChanged(String),
    TaskDaysChanged(String),
    TaskRunHighestToggled(bool),
    TaskTargetChanged(String),
    ExitCodeInputChanged(String),
    SelectQuickExitCode(i32),

    // Copy & Transfer Switches
    ResetCopyDefaults,
    ToggleSubdirsS(bool),
    ToggleSubdirsE(bool),
    TogglePurge(bool),
    ToggleMirror(bool),
    ToggleMoveFiles(bool),
    ToggleMoveAll(bool),
    MaxLevelsChanged(String),
    ThreadsChanged(String),
    ToggleRestartableZ(bool),
    ToggleBackupB(bool),
    ToggleRestartBackupZB(bool),
    ToggleUnbufferedJ(bool),
    ToggleEfsRaw(bool),
    ToggleCompress(bool),
    ToggleNoOffload(bool),
    ToggleNoClone(bool),
    SparseModeChanged(SparseChoice),
    CopyFlagChanged(char, bool),
    ToggleNoCopy(bool),
    CopyPresetDat,
    CopyPresetSec,
    CopyPresetAll,
    ToggleSecFix(bool),
    ToggleTimFix(bool),
    DirCopyFlagChanged(char, bool),
    ToggleNoDirCopy(bool),
    ToggleCreateTree(bool),
    ToggleFatNames(bool),
    ToggleDisableLongPaths(bool),
    ToggleCopyJunctions(bool),
    ToggleCopySymlinks(bool),
    AddAttrsChanged(String),
    RemoveAttrsChanged(String),

    // File Selection Switches
    ResetSelectionDefaults,
    ExcludeFilesChanged(String),
    ExcludeDirsChanged(String),
    ToggleExcludeJunctions(bool),
    ToggleExcludeDirJunctions(bool),
    ToggleExcludeFileSymlinks(bool),
    ToggleExcludeChanged(bool),
    ToggleExcludeNewer(bool),
    ToggleExcludeOlder(bool),
    ToggleExcludeExtra(bool),
    ToggleExcludeLonely(bool),
    ToggleIncludeSame(bool),
    ToggleIncludeTweaked(bool),
    ToggleIncludeModified(bool),
    ToggleFatFileTimes(bool),
    ToggleCompensateDst(bool),
    ToggleArchiveOnly(bool),
    ToggleArchiveReset(bool),
    IncludeAttrsChanged(String),
    ExcludeAttrsChanged(String),
    MaxSizeChanged(String),
    MinSizeChanged(String),
    MaxAgeChanged(String),
    MinAgeChanged(String),
    MaxLadChanged(String),
    MinLadChanged(String),

    // Performance & Retry Switches
    ResetRetryDefaults,
    RetriesChanged(String),
    WaitSecsChanged(String),
    ToggleWaitShares(bool),
    ToggleRegDefaults(bool),
    ToggleLfsm(bool),
    LfsmFloorChanged(String),
    IoMaxSizeChanged(String),
    IoRateChanged(String),
    ThresholdChanged(String),
    IpgChanged(String),
    MonChangesChanged(String),
    MotMinutesChanged(String),
    RunHoursChanged(String),
    TogglePerFileRunHours(bool),

    // Logging & Job File Switches
    ResetLoggingDefaults,
    ToggleListOnly(bool),
    ToggleReportExtra(bool),
    ToggleVerbose(bool),
    ToggleTimestamps(bool),
    ToggleFullPaths(bool),
    ToggleBytes(bool),
    ToggleUnicodeOutput(bool),
    ToggleNoSizes(bool),
    ToggleNoClasses(bool),
    ToggleNoFileList(bool),
    ToggleNoDirList(bool),
    ToggleNoProgress(bool),
    ToggleEta(bool),
    ToggleNoJobHeader(bool),
    ToggleNoJobSummary(bool),
    ToggleTee(bool),
    LogModeChanged(LogModeChoice),
    LogPathChanged(String),
    BrowseLogFile,
    LoadJobChanged(String),
    SaveJobChanged(String),
    ToggleQuit(bool),
    ToggleNosd(bool),
    ToggleNodd(bool),
    ExtraArgsChanged(String),

    // Live Console & Diff
    SwitchConsoleSubTab(ConsoleSubTab),
    SetActionFilter(ActionFilter),
    ActionSearchChanged(String),
    LogSearchChanged(String),
    CopyLogToClipboard,
    ExportReport(ReportFormat),

    // Batch Queue
    AddCurrentToQueue,
    ToggleStopOnError(bool),
    StartBatchQueue(bool),
    MoveQueueUp(usize),
    MoveQueueDown(usize),
    LoadQueueItem(usize),
    DuplicateQueueItem(usize),
    RemoveQueueItem(usize),
    ClearBatchQueue,

    // History
    SelectHistory(usize),
    ReloadHistoryCommand(usize),
    ClearHistory,
}
