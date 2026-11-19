use anyhow::{Context, Result, bail};
use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use robocopy2rust_core::format::{format_bytes, format_duration};
use robocopy2rust_core::{
    ProfileStore, RobocopyJob, RunHistoryEntry, ScheduleSpec, ValidationSeverity, build_batch_script,
    build_powershell_script, build_schtasks_command, builtin_presets, decode_exit_code, find_preset,
    format_command_line, init_logging, parse_command_line, parse_rcj, serialize_rcj, validate_job,
};
use robocopy2rust_report::{ReportFormat, parse_log_file_to_run_result, write_report};
use robocopy2rust_runner::{CancelToken, RunResult, RunnerEvent, detect_environment, spawn_job_runner};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

fn install_signal_handler(cancel: CancelToken) {
    let _ = ctrlc::set_handler(move || {
        if SHUTDOWN_FLAG.swap(true, Ordering::SeqCst) {
            eprintln!("\n[!] Second Ctrl+C received — terminating.");
            std::process::exit(130);
        } else {
            eprintln!("\n[!] Interrupt received — cancelling active Robocopy process...");
            cancel.cancel();
        }
    });
}

#[derive(Parser, Debug)]
#[command(
    name = "robocopy2rust",
    author = "Robocopy2Rust Contributors",
    version,
    about = "100% pure-Rust Microsoft Robocopy GUI launcher, CLI job builder, validator, runner, and MCP server",
    long_about = "Build, validate, import (.rcj / CLI), dry-run, execute, schedule, and audit Microsoft Robocopy jobs in pure Rust.\n\nInvoke without arguments or run `robocopy2rust gui` to open the Classic Win32/MMC Desktop Console."
)]
pub struct Cli {
    /// Logging verbosity level (error, warn, info, debug, trace)
    #[arg(short, long, global = true, default_value = "warn")]
    pub log_level: String,

    /// Optional log output file path
    #[arg(long, global = true)]
    pub log_file: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List all built-in Robocopy job presets
    Presets {
        #[arg(long)]
        json: bool,
    },
    /// Build a validated Robocopy command and optional .rcj / .bat / .ps1 / schtasks exports
    Build(BuildArgs),
    /// Parse a raw Robocopy command line or .rcj file into structured JSON and validation diagnostics
    Parse(ParseArgs),
    /// Decode a Microsoft Robocopy bitmask exit code (0..=16)
    Decode {
        code: i32,
        #[arg(long)]
        json: bool,
    },
    /// Run a safe non-destructive /L dry-run simulation
    DryRun(ExecArgs),
    /// Execute a Robocopy job with streaming telemetry and bitmask exit decoding
    Run(RunArgs),
    /// Manage saved persistent Robocopy job profiles
    Profile(ProfileArgs),
    /// Generate offline HTML, JSON, or Markdown audit reports from an existing Robocopy log file
    Report(ReportArgs),
    /// Detect host OS, Robocopy binary availability, and configuration directory
    Env {
        #[arg(long)]
        json: bool,
    },
    /// Start the Model Context Protocol (MCP) JSON-RPC 2.0 stdio server
    Mcp,
    /// Launch the native Classic Win32/MMC Desktop Console (robocopy2rust-gui)
    Gui(GuiArgs),
}

#[derive(Args, Debug, Clone)]
pub struct JobSourceArgs {
    #[arg(short, long)]
    pub source: Option<String>,

    #[arg(short, long)]
    pub dest: Option<String>,

    #[arg(long)]
    pub preset: Option<String>,

    #[arg(long)]
    pub profile: Option<String>,

    #[arg(long)]
    pub rcj: Option<PathBuf>,

    #[arg(long, num_args = 1..)]
    pub files: Vec<String>,

    #[arg(long)]
    pub mirror: bool,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long)]
    pub threads: Option<u16>,

    #[arg(long)]
    pub retries: Option<u32>,

    #[arg(long)]
    pub wait: Option<u32>,
}

#[derive(Args, Debug)]
pub struct BuildArgs {
    #[command(flatten)]
    pub job_args: JobSourceArgs,

    #[arg(long)]
    pub export_rcj: Option<PathBuf>,

    #[arg(long)]
    pub export_bat: Option<PathBuf>,

    #[arg(long)]
    pub export_ps1: Option<PathBuf>,

    #[arg(long)]
    pub export_schtasks: bool,

    #[arg(long, default_value = "Robocopy2Rust_Job")]
    pub task_name: String,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ParseArgs {
    /// Raw Robocopy command line string, or path to an .rcj file when --rcj is passed
    pub input: String,

    #[arg(long)]
    pub rcj: bool,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ExecArgs {
    #[command(flatten)]
    pub job_args: JobSourceArgs,

    #[arg(long)]
    pub report: Option<PathBuf>,

    #[arg(long, default_value = "html")]
    pub report_format: String,

    #[arg(long)]
    pub json: bool,

    #[arg(long)]
    pub raw_exit_code: bool,
}

#[derive(Args, Debug)]
pub struct RunArgs {
    #[command(flatten)]
    pub exec: ExecArgs,

    /// Confirm destructive switches (/MIR, /PURGE, /MOVE, /MOV)
    #[arg(short = 'y', long)]
    pub yes: bool,
}

#[derive(Args, Debug)]
pub struct ProfileArgs {
    #[command(subcommand)]
    pub action: ProfileAction,
}

#[derive(Subcommand, Debug)]
pub enum ProfileAction {
    List {
        #[arg(long)]
        json: bool,
    },
    Show {
        name: String,
        #[arg(long)]
        json: bool,
    },
    Save {
        name: String,
        #[arg(long, default_value = "")]
        description: String,
        #[command(flatten)]
        job_args: JobSourceArgs,
    },
    Delete {
        name: String,
    },
    ImportRcj {
        name: String,
        rcj_path: PathBuf,
        #[arg(long, default_value = "")]
        description: String,
    },
    ExportRcj {
        name: String,
        out_path: PathBuf,
    },
}

#[derive(Args, Debug)]
pub struct ReportArgs {
    #[arg(long)]
    pub log_file: PathBuf,

    #[arg(long)]
    pub output: PathBuf,

    #[arg(long, default_value = "html")]
    pub format: String,
}

#[derive(Args, Debug)]
pub struct GuiArgs {
    #[arg(long)]
    pub source: Option<String>,

    #[arg(long)]
    pub dest: Option<String>,

    #[arg(long)]
    pub rcj: Option<PathBuf>,

    #[arg(long)]
    pub profile: Option<String>,
}

fn assemble_job(args: &JobSourceArgs) -> Result<RobocopyJob> {
    let mut job = if let Some(rcj_path) = &args.rcj {
        let bytes = std::fs::read(rcj_path).with_context(|| format!("Failed to read {}", rcj_path.display()))?;
        let (parsed, warnings) = parse_rcj(&bytes)?;
        for w in warnings {
            eprintln!("[warn] .rcj: {w}");
        }
        parsed
    } else if let Some(prof_name) = &args.profile {
        let store = ProfileStore::default_store();
        let prof = store
            .find_profile(prof_name)?
            .ok_or_else(|| anyhow::anyhow!("Saved profile not found: {prof_name}"))?;
        prof.job
    } else if let Some(preset_name) = &args.preset {
        find_preset(preset_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown built-in preset: {preset_name}"))?
            .job
    } else {
        RobocopyJob::default()
    };

    if let Some(s) = &args.source {
        job.source = s.clone();
    }
    if let Some(d) = &args.dest {
        job.destination = d.clone();
    }
    if !args.files.is_empty() {
        job.file_specs = args.files.clone();
    }
    if args.mirror {
        job.copy.mirror = true;
        job.copy.subdirectories_including_empty = true;
        job.copy.purge = true;
    }
    if args.dry_run {
        job = job.with_list_only();
    }
    if let Some(mt) = args.threads {
        job.copy.multithreaded = Some(mt);
    }
    if let Some(r) = args.retries {
        job.retry.retries = Some(r);
    }
    if let Some(w) = args.wait {
        job.retry.wait_seconds = Some(w);
    }
    Ok(job)
}

fn print_validation_findings(job: &RobocopyJob) -> bool {
    let findings = validate_job(job);
    let mut has_error = false;
    for f in &findings {
        let tag = match f.severity {
            ValidationSeverity::Error => {
                has_error = true;
                "ERROR"
            }
            ValidationSeverity::Warning => "WARN",
            ValidationSeverity::Info => "INFO",
        };
        eprintln!("[{tag}] {}: {}", f.code, f.message);
        if let Some(sug) = &f.suggestion {
            eprintln!("       Suggestion: {sug}");
        }
    }
    has_error
}

fn execute_and_report(job: RobocopyJob, exec: &ExecArgs) -> Result<i32> {
    if print_validation_findings(&job) {
        bail!("Validation errors prevented Robocopy execution.");
    }

    let (rx, cancel) = spawn_job_runner(job.clone(), None);
    install_signal_handler(cancel);

    let mut final_result: Option<RunResult> = None;
    for ev in rx {
        match ev {
            RunnerEvent::Started { command_line } => {
                if !exec.json {
                    println!("> {command_line}");
                }
            }
            RunnerEvent::LogLine(line) => {
                if !exec.json {
                    println!("{line}");
                }
            }
            RunnerEvent::FileAction(rec) => {
                if !exec.json {
                    println!("{}", rec.raw_line);
                }
            }
            RunnerEvent::Error(err) => {
                if !exec.json {
                    eprintln!("[robocopy error] {err}");
                }
            }
            RunnerEvent::Finished(res) => {
                final_result = Some(*res);
            }
            RunnerEvent::Progress { .. } | RunnerEvent::Summary(_) => {}
        }
    }

    let res = final_result.ok_or_else(|| anyhow::anyhow!("Runner exited without producing a result"))?;

    let store = ProfileStore::default_store();
    let _ = store.append_history(RunHistoryEntry {
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
    });

    if let Some(report_out) = &exec.report {
        let fmt = ReportFormat::parse(&exec.report_format)?;
        let written = write_report(&res, fmt, report_out)?;
        if !exec.json {
            for p in written {
                println!("Report written: {}", p.display());
            }
        }
    }

    if exec.json {
        println!("{}", serde_json::to_string_pretty(&res)?);
    } else {
        println!();
        println!(
            "=== Robocopy2Rust Finished in {} ===",
            format_duration(res.duration_secs)
        );
        println!("Exit Code : {} ({})", res.exit_code, res.exit_info.short_label);
        println!("Summary   : {}", res.exit_info.summary);
        println!(
            "Totals    : Files copied {}/{}, Failed {}, Extras {}, Bytes copied {}",
            res.summary.files.copied,
            res.summary.files.total,
            res.summary.files.failed,
            res.summary.files.extras,
            format_bytes(res.summary.bytes.copied),
        );
        println!("Log SHA256: {}", res.log_sha256);
    }

    if exec.raw_exit_code {
        Ok(res.exit_code)
    } else if res.exit_info.is_success {
        Ok(0)
    } else {
        Ok(res.exit_code.max(1))
    }
}

fn launch_gui(args: &GuiArgs) -> Result<()> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf));
    let gui_name = if cfg!(windows) {
        "robocopy2rust-gui.exe"
    } else {
        "robocopy2rust-gui"
    };
    let candidate = exe_dir
        .as_ref()
        .map(|d| d.join(gui_name))
        .filter(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from(gui_name));

    let mut cmd = std::process::Command::new(&candidate);
    if let Some(s) = &args.source {
        cmd.arg("--source").arg(s);
    }
    if let Some(d) = &args.dest {
        cmd.arg("--dest").arg(d);
    }
    if let Some(rcj) = &args.rcj {
        cmd.arg("--rcj").arg(rcj);
    }
    if let Some(prof) = &args.profile {
        cmd.arg("--profile").arg(prof);
    }
    cmd.spawn()
        .with_context(|| format!("Failed to launch GUI executable {}", candidate.display()))?;
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let _ = init_logging(&cli.log_level, cli.log_file.as_deref());

    let code = match run_cli(cli) {
        Ok(exit_code) => exit_code,
        Err(err) => {
            eprintln!("Error: {err:#}");
            1
        }
    };
    if code != 0 {
        std::process::exit(code);
    }
}

fn run_cli(cli: Cli) -> Result<i32> {
    let Some(command) = cli.command else {
        launch_gui(&GuiArgs {
            source: None,
            dest: None,
            rcj: None,
            profile: None,
        })?;
        return Ok(0);
    };

    match command {
        Commands::Presets { json: as_json } => {
            let presets = builtin_presets();
            if as_json {
                println!("{}", serde_json::to_string_pretty(&presets)?);
            } else {
                for p in presets {
                    let tag = if p.destructive { " [DESTRUCTIVE]" } else { "" };
                    println!("{:<22} | {}{}", p.id, p.name, tag);
                    println!("   {}", p.summary);
                }
            }
            Ok(0)
        }
        Commands::Build(b) => {
            let job = assemble_job(&b.job_args)?;
            let findings = validate_job(&job);
            let cmd_line = format_command_line(&job);

            if let Some(p) = &b.export_rcj {
                std::fs::write(p, serialize_rcj(&job))?;
            }
            if let Some(p) = &b.export_bat {
                std::fs::write(p, build_batch_script(&job))?;
            }
            if let Some(p) = &b.export_ps1 {
                std::fs::write(p, build_powershell_script(&job))?;
            }

            let schtasks = if b.export_schtasks {
                Some(build_schtasks_command(
                    &b.task_name,
                    &cmd_line,
                    &ScheduleSpec::default(),
                ))
            } else {
                None
            };

            if b.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "command_line": cmd_line,
                        "findings": findings,
                        "schtasks": schtasks,
                        "job": job,
                    }))?
                );
            } else {
                println!("{cmd_line}");
                if let Some(st) = schtasks {
                    println!("\nTask Scheduler Command:\n{st}");
                }
                let _ = print_validation_findings(&job);
            }
            Ok(0)
        }
        Commands::Parse(p) => {
            let (job, warnings) = if p.rcj {
                let bytes = std::fs::read(&p.input)?;
                parse_rcj(&bytes)?
            } else {
                let parsed = parse_command_line(&p.input);
                (parsed.job, parsed.warnings)
            };
            let findings = validate_job(&job);
            if p.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "command_line": format_command_line(&job),
                        "warnings": warnings,
                        "findings": findings,
                        "job": job,
                    }))?
                );
            } else {
                println!("Normalized Command:\n  {}", format_command_line(&job));
                for w in warnings {
                    eprintln!("[parse warn] {w}");
                }
                let _ = print_validation_findings(&job);
            }
            Ok(0)
        }
        Commands::Decode { code, json: as_json } => {
            let info = decode_exit_code(code);
            if as_json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("Exit Code : {} ({})", info.code, info.short_label);
                println!("Success   : {}", info.is_success);
                println!("Summary   : {}", info.summary);
                for b in &info.bit_descriptions {
                    println!("  - {b}");
                }
            }
            Ok(0)
        }
        Commands::DryRun(exec) => {
            let job = assemble_job(&exec.job_args)?.with_list_only();
            execute_and_report(job, &exec)
        }
        Commands::Run(run) => {
            let job = assemble_job(&run.exec.job_args)?;
            if job.has_destructive_flags() && !run.yes {
                let flags = job.destructive_switch_names().join(", ");
                bail!(
                    "Refusing destructive Robocopy execution ({flags}) without --yes. Run `robocopy2rust dry-run` first or pass --yes."
                );
            }
            execute_and_report(job, &run.exec)
        }
        Commands::Profile(prof) => {
            let store = ProfileStore::default_store();
            match prof.action {
                ProfileAction::List { json: as_json } => {
                    let list = store.load_profiles()?;
                    if as_json {
                        println!("{}", serde_json::to_string_pretty(&list)?);
                    } else if list.is_empty() {
                        println!("No saved profiles in {}", store.profiles_path().display());
                    } else {
                        for p in list {
                            println!("{} ({}) -> {} => {}", p.name, p.id, p.job.source, p.job.destination);
                        }
                    }
                }
                ProfileAction::Show { name, json: as_json } => {
                    let p = store
                        .find_profile(&name)?
                        .ok_or_else(|| anyhow::anyhow!("Profile not found: {name}"))?;
                    if as_json {
                        println!("{}", serde_json::to_string_pretty(&p)?);
                    } else {
                        println!("Name       : {}", p.name);
                        println!("Description: {}", p.description);
                        println!("Command    : {}", format_command_line(&p.job));
                    }
                }
                ProfileAction::Save {
                    name,
                    description,
                    job_args,
                } => {
                    let job = assemble_job(&job_args)?;
                    let list = store.upsert_profile(&name, &description, job)?;
                    println!("Saved profile '{name}' ({} total profiles).", list.len());
                }
                ProfileAction::Delete { name } => {
                    let (removed, _) = store.delete_profile(&name)?;
                    if removed {
                        println!("Deleted profile '{name}'.");
                    } else {
                        bail!("Profile not found: {name}");
                    }
                }
                ProfileAction::ImportRcj {
                    name,
                    rcj_path,
                    description,
                } => {
                    let bytes = std::fs::read(&rcj_path)?;
                    let (job, _) = parse_rcj(&bytes)?;
                    store.upsert_profile(&name, &description, job)?;
                    println!("Imported {} into profile '{name}'.", rcj_path.display());
                }
                ProfileAction::ExportRcj { name, out_path } => {
                    let p = store
                        .find_profile(&name)?
                        .ok_or_else(|| anyhow::anyhow!("Profile not found: {name}"))?;
                    std::fs::write(&out_path, serialize_rcj(&p.job))?;
                    println!("Exported profile '{name}' to {}", out_path.display());
                }
            }
            Ok(0)
        }
        Commands::Report(rep) => {
            let fmt = ReportFormat::parse(&rep.format)?;
            let res = parse_log_file_to_run_result(&rep.log_file)?;
            let written = write_report(&res, fmt, &rep.output)?;
            for p in written {
                println!("Generated report: {}", p.display());
            }
            Ok(0)
        }
        Commands::Env { json: as_json } => {
            let env = detect_environment();
            if as_json {
                println!("{}", serde_json::to_string_pretty(&env)?);
            } else {
                println!("OS                 : {}", env.os);
                println!("Robocopy Available : {}", env.robocopy_available);
                println!(
                    "Robocopy Path      : {}",
                    env.robocopy_path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "<not found>".to_string())
                );
                println!("Config Directory   : {}", env.config_dir.display());
            }
            Ok(0)
        }
        Commands::Mcp => {
            robocopy2rust_mcp::run_stdio_server().map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok(0)
        }
        Commands::Gui(g) => {
            launch_gui(&g)?;
            Ok(0)
        }
    }
}
