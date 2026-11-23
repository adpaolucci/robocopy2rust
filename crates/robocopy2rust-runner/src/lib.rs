#![forbid(unsafe_code)]

pub mod env;
pub mod output_parser;
pub mod runner;

pub use env::{EnvironmentInfo, detect_environment};
pub use output_parser::{
    CountRow, FileActionKind, FileActionRecord, LogLineKind, ParsedLineOutcome, RobocopyOutputParser, RunSummary,
};
pub use runner::{CancelToken, RunResult, RunnerEvent, run_job_blocking, spawn_job_runner};
