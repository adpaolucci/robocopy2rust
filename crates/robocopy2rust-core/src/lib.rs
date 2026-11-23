#![forbid(unsafe_code)]

pub mod command;
pub mod exit_code;
pub mod format;
pub mod logging;
pub mod model;
pub mod parser;
pub mod presets;
pub mod profiles;
pub mod rcj;
pub mod validation;

pub use command::{
    ScheduleCadence, ScheduleSpec, build_args, build_batch_script, build_powershell_script, build_schtasks_command,
    format_command_line, normalize_robocopy_path, quote_windows_arg,
};
pub use exit_code::{ExitCodeInfo, ExitCodeSeverity, decode_exit_code};
pub use format::{encode_hex_lower, format_bytes, format_duration, parse_robocopy_size_token};
pub use logging::init_logging;
pub use model::{
    CopyFlags, CopyOptions, DirCopyFlags, FileAttrFlags, FileSelectionOptions, JobFileOptions, LogFileMode,
    LoggingOptions, RetryOptions, RobocopyJob, SparseMode, ThrottleOptions,
};
pub use parser::{ParseResult, parse_command_line, tokenize_windows_cmdline};
pub use presets::{JobPreset, builtin_presets, find_preset};
pub use profiles::{ProfileStore, RunHistoryEntry, SavedProfile};
pub use rcj::{parse_rcj, serialize_rcj};
pub use validation::{ValidationFinding, ValidationSeverity, validate_job};
