#![forbid(unsafe_code)]

use robocopy2rust_core::{
    ProfileStore, RobocopyJob, ScheduleCadence, ScheduleSpec, ValidationSeverity, build_args, build_batch_script,
    build_powershell_script, build_schtasks_command, builtin_presets, decode_exit_code, find_preset,
    format_command_line, parse_command_line, parse_rcj, serialize_rcj, validate_job,
};
use robocopy2rust_report::{ReportFormat, parse_log_file_to_run_result, write_report};
use robocopy2rust_runner::{CancelToken, detect_environment, run_job_blocking};
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::path::Path;
use tracing::{debug, error, info};

pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
pub const SERVER_NAME: &str = "robocopy2rust-mcp";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_tools_manifest() -> Value {
    json!([
        {
            "name": "robocopy2rust_environment",
            "description": "Detect host OS, Robocopy executable availability, and configuration directory.",
            "annotations": { "readOnlyHint": true, "destructiveHint": false },
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "robocopy2rust_presets",
            "description": "List all built-in Robocopy presets (safe backup, mirror, dry-run, NVMe, WAN, enterprise, move, LFSM).",
            "annotations": { "readOnlyHint": true, "destructiveHint": false },
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "robocopy2rust_build_command",
            "description": "Build a validated Robocopy command line plus optional .rcj, .bat, .ps1, and Windows Task Scheduler schtasks commands.",
            "annotations": { "readOnlyHint": true, "destructiveHint": false },
            "inputSchema": {
                "type": "object",
                "properties": {
                    "job": { "type": "object", "description": "Optional full RobocopyJob JSON object" },
                    "preset": { "type": "string", "description": "Optional built-in preset ID or name" },
                    "source": { "type": "string", "description": "Source directory path" },
                    "destination": { "type": "string", "description": "Destination directory path" },
                    "file_specs": { "type": "array", "items": { "type": "string" } },
                    "dry_run": { "type": "boolean", "description": "Enable /L list-only mode" },
                    "include_scripts": { "type": "boolean", "description": "Include .rcj, .bat, and .ps1 outputs" },
                    "task_name": { "type": "string", "description": "Optional Task Scheduler task name to generate schtasks command" },
                    "cadence": { "type": "string", "description": "Schedule cadence: daily, weekly, hourly, onstart, onlogon" },
                    "start_time": { "type": "string", "description": "HH:MM start time (default 02:00)" }
                }
            }
        },
        {
            "name": "robocopy2rust_parse_command",
            "description": "Parse a raw Robocopy command line or an .rcj job file path into a structured RobocopyJob and validation diagnostics.",
            "annotations": { "readOnlyHint": true, "destructiveHint": false },
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Raw robocopy command-line string" },
                    "rcj_path": { "type": "string", "description": "Path to a .rcj Robocopy job file" }
                }
            }
        },
        {
            "name": "robocopy2rust_decode_exit_code",
            "description": "Decode a Robocopy bitmask exit code (0-16) into severity, success boolean, and bit descriptions.",
            "annotations": { "readOnlyHint": true, "destructiveHint": false },
            "inputSchema": {
                "type": "object",
                "properties": {
                    "code": { "type": "integer", "description": "Robocopy exit code (0..=16)" }
                },
                "required": ["code"]
            }
        },
        {
            "name": "robocopy2rust_profiles",
            "description": "Manage saved persistent Robocopy job profiles (list, get, save, delete).",
            "annotations": { "readOnlyHint": false, "destructiveHint": false },
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["list", "get", "save", "delete"] },
                    "name": { "type": "string", "description": "Profile name or ID" },
                    "description": { "type": "string", "description": "Profile description for save" },
                    "job": { "type": "object", "description": "RobocopyJob JSON object for save" },
                    "store_dir": { "type": "string", "description": "Optional custom profile storage directory" }
                },
                "required": ["action"]
            }
        },
        {
            "name": "robocopy2rust_dry_run",
            "description": "Run a safe non-destructive /L list-only Robocopy simulation and return parsed summary counts and file actions.",
            "annotations": { "readOnlyHint": true, "destructiveHint": false },
            "inputSchema": {
                "type": "object",
                "properties": {
                    "job": { "type": "object" },
                    "command": { "type": "string" },
                    "source": { "type": "string" },
                    "destination": { "type": "string" },
                    "preset": { "type": "string" }
                }
            }
        },
        {
            "name": "robocopy2rust_run",
            "description": "Execute a Robocopy job on the host OS. Refuses jobs with /MIR, /PURGE, /MOVE, or /MOV unless confirm_destructive is explicitly true.",
            "annotations": { "readOnlyHint": false, "destructiveHint": true },
            "inputSchema": {
                "type": "object",
                "properties": {
                    "job": { "type": "object" },
                    "command": { "type": "string" },
                    "source": { "type": "string" },
                    "destination": { "type": "string" },
                    "preset": { "type": "string" },
                    "confirm_destructive": { "type": "boolean", "description": "Explicitly confirm destructive /MIR, /PURGE, /MOVE, or /MOV execution" }
                }
            }
        },
        {
            "name": "robocopy2rust_report",
            "description": "Generate an air-gapped offline HTML, JSON, or Markdown audit report from a Robocopy log file or dry-run job.",
            "annotations": { "readOnlyHint": false, "destructiveHint": false },
            "inputSchema": {
                "type": "object",
                "properties": {
                    "log_path": { "type": "string", "description": "Path to existing Robocopy log file" },
                    "output": { "type": "string", "description": "Output file path or directory when format is 'all'" },
                    "format": { "type": "string", "description": "html, json, markdown, or all" },
                    "job": { "type": "object", "description": "Optional job to dry-run if log_path is omitted" },
                    "source": { "type": "string" },
                    "destination": { "type": "string" }
                },
                "required": ["output"]
            }
        }
    ])
}

fn resolve_job_from_args(args: &Value) -> Result<RobocopyJob, String> {
    let mut job = if args.get("job").is_some_and(Value::is_object) {
        serde_json::from_value::<RobocopyJob>(args["job"].clone()).map_err(|e| format!("Invalid job JSON: {e}"))?
    } else if let Some(cmd) = args["command"].as_str() {
        parse_command_line(cmd).job
    } else if let Some(preset_name) = args["preset"].as_str() {
        find_preset(preset_name)
            .ok_or_else(|| format!("Unknown preset: {preset_name}"))?
            .job
    } else {
        RobocopyJob::default()
    };

    if let Some(src) = args["source"].as_str()
        && !src.trim().is_empty()
    {
        job.source = src.trim().to_string();
    }
    if let Some(dst) = args["destination"].as_str()
        && !dst.trim().is_empty()
    {
        job.destination = dst.trim().to_string();
    }
    if let Some(arr) = args["file_specs"].as_array() {
        let specs: Vec<String> = arr
            .iter()
            .filter_map(|v| v.as_str().map(str::trim))
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        if !specs.is_empty() {
            job.file_specs = specs;
        }
    }
    if let Some(dry) = args["dry_run"].as_bool()
        && dry
    {
        job = job.with_list_only();
    }
    Ok(job)
}

pub fn call_tool(name: &str, args: &Value) -> Result<Value, String> {
    match name {
        "robocopy2rust_environment" => {
            let env = detect_environment();
            serde_json::to_value(env).map_err(|e| e.to_string())
        }
        "robocopy2rust_presets" => {
            let presets = builtin_presets();
            serde_json::to_value(presets).map_err(|e| e.to_string())
        }
        "robocopy2rust_build_command" => {
            let job = resolve_job_from_args(args)?;
            let findings = validate_job(&job);
            let cmd = format_command_line(&job);
            let argv = build_args(&job);
            let include_scripts = args["include_scripts"].as_bool().unwrap_or(false);
            let rcj = if include_scripts {
                Some(serialize_rcj(&job))
            } else {
                None
            };
            let bat = if include_scripts {
                Some(build_batch_script(&job))
            } else {
                None
            };
            let ps1 = if include_scripts {
                Some(build_powershell_script(&job))
            } else {
                None
            };
            let schtasks = args["task_name"].as_str().map(|tn| {
                let cadence = match args["cadence"]
                    .as_str()
                    .unwrap_or("daily")
                    .to_ascii_lowercase()
                    .as_str()
                {
                    "weekly" => ScheduleCadence::Weekly,
                    "hourly" => ScheduleCadence::Hourly,
                    "onstart" => ScheduleCadence::OnStart,
                    "onlogon" => ScheduleCadence::OnLogon,
                    _ => ScheduleCadence::Daily,
                };
                let spec = ScheduleSpec {
                    cadence,
                    start_time: args["start_time"].as_str().unwrap_or("02:00").to_string(),
                    days: None,
                    run_highest: true,
                };
                build_schtasks_command(tn, &cmd, &spec)
            });

            Ok(json!({
                "command_line": cmd,
                "args": argv,
                "destructive": job.has_destructive_flags(),
                "findings": findings,
                "job": job,
                "rcj": rcj,
                "batch_script": bat,
                "powershell_script": ps1,
                "schtasks": schtasks,
            }))
        }
        "robocopy2rust_parse_command" => {
            if let Some(rcj_path) = args["rcj_path"].as_str() {
                let meta = std::fs::metadata(rcj_path).map_err(|e| format!("Failed reading .rcj '{rcj_path}': {e}"))?;
                if meta.len() > 4 * 1024 * 1024 {
                    return Err(format!(".rcj file '{rcj_path}' exceeds maximum allowed size (4 MiB)"));
                }
                let bytes = std::fs::read(rcj_path).map_err(|e| format!("Failed reading .rcj '{rcj_path}': {e}"))?;
                let (job, warnings) = parse_rcj(&bytes).map_err(|e| e.to_string())?;
                let findings = validate_job(&job);
                return Ok(json!({
                    "command_line": format_command_line(&job),
                    "warnings": warnings,
                    "findings": findings,
                    "job": job,
                }));
            }
            let raw = args["command"]
                .as_str()
                .ok_or("Provide either 'command' or 'rcj_path'")?;
            let parsed = parse_command_line(raw);
            let findings = validate_job(&parsed.job);
            Ok(json!({
                "command_line": format_command_line(&parsed.job),
                "warnings": parsed.warnings,
                "findings": findings,
                "job": parsed.job,
            }))
        }
        "robocopy2rust_decode_exit_code" => {
            let code = args["code"].as_i64().ok_or("Missing required integer 'code'")? as i32;
            let info = decode_exit_code(code);
            serde_json::to_value(info).map_err(|e| e.to_string())
        }
        "robocopy2rust_profiles" => {
            let action = args["action"].as_str().ok_or("Missing required string 'action'")?;
            let store = match args["store_dir"].as_str() {
                Some(dir) if !dir.trim().is_empty() => ProfileStore::with_dir(dir),
                _ => ProfileStore::default_store(),
            };
            match action {
                "list" => {
                    let profiles = store.load_profiles().map_err(|e| e.to_string())?;
                    serde_json::to_value(profiles).map_err(|e| e.to_string())
                }
                "get" => {
                    let name = args["name"].as_str().ok_or("Missing 'name' for get")?;
                    let profile = store
                        .find_profile(name)
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| format!("Profile not found: {name}"))?;
                    serde_json::to_value(profile).map_err(|e| e.to_string())
                }
                "save" => {
                    let name = args["name"].as_str().ok_or("Missing 'name' for save")?;
                    let desc = args["description"].as_str().unwrap_or("");
                    let job = if args.get("job").is_some_and(Value::is_object) {
                        serde_json::from_value::<RobocopyJob>(args["job"].clone())
                            .map_err(|e| format!("Invalid job JSON: {e}"))?
                    } else {
                        RobocopyJob::default()
                    };
                    let profiles = store.upsert_profile(name, desc, job).map_err(|e| e.to_string())?;
                    serde_json::to_value(profiles).map_err(|e| e.to_string())
                }
                "delete" => {
                    let name = args["name"].as_str().ok_or("Missing 'name' for delete")?;
                    let (removed, profiles) = store.delete_profile(name).map_err(|e| e.to_string())?;
                    Ok(json!({ "removed": removed, "profiles": profiles }))
                }
                other => Err(format!("Unsupported profiles action: {other}")),
            }
        }
        "robocopy2rust_dry_run" => {
            let job = resolve_job_from_args(args)?.with_list_only();
            let errors: Vec<_> = validate_job(&job)
                .into_iter()
                .filter(|f| f.severity == ValidationSeverity::Error)
                .collect();
            if !errors.is_empty() {
                return Err(format!(
                    "Job validation errors prevented dry-run: {}",
                    serde_json::to_string(&errors).unwrap_or_default()
                ));
            }
            let res = run_job_blocking(&job, None, &CancelToken::new()).map_err(|e| e.to_string())?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "robocopy2rust_run" => {
            let job = resolve_job_from_args(args)?;
            let errors: Vec<_> = validate_job(&job)
                .into_iter()
                .filter(|f| f.severity == ValidationSeverity::Error)
                .collect();
            if !errors.is_empty() {
                return Err(format!(
                    "Job validation errors prevented execution: {}",
                    serde_json::to_string(&errors).unwrap_or_default()
                ));
            }
            let confirm_destructive = args["confirm_destructive"].as_bool().unwrap_or(false);
            if job.has_destructive_flags() && !confirm_destructive {
                let flags = job.destructive_switch_names().join(", ");
                return Err(format!(
                    "SAFETY REFUSAL: Job includes destructive flag(s) ({flags}). Re-run with robocopy2rust_dry_run first or set confirm_destructive=true."
                ));
            }
            let res = run_job_blocking(&job, None, &CancelToken::new()).map_err(|e| e.to_string())?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "robocopy2rust_report" => {
            let out_str = args["output"].as_str().ok_or("Missing required string 'output'")?;
            let fmt_str = args["format"].as_str().unwrap_or("html");
            let format = ReportFormat::parse(fmt_str).map_err(|e| e.to_string())?;

            let result = if let Some(log_path) = args["log_path"].as_str() {
                parse_log_file_to_run_result(Path::new(log_path)).map_err(|e| e.to_string())?
            } else {
                let job = resolve_job_from_args(args)?.with_list_only();
                run_job_blocking(&job, None, &CancelToken::new()).map_err(|e| e.to_string())?
            };

            let written = write_report(&result, format, Path::new(out_str)).map_err(|e| e.to_string())?;
            Ok(json!({
                "status": "success",
                "files_written": written,
                "exit_code": result.exit_code,
                "log_sha256": result.log_sha256,
            }))
        }
        other => Err(format!("Unknown robocopy2rust MCP tool: {other}")),
    }
}

pub fn handle_jsonrpc_request(req: &Value) -> Option<Value> {
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
    debug!(method, "Received MCP JSON-RPC request");

    match method {
        m if m.starts_with("notifications/") => None,
        "ping" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {}
        })),
        "initialize" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": SERVER_NAME,
                    "version": SERVER_VERSION
                }
            }
        })),
        "tools/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": get_tools_manifest()
            }
        })),
        "tools/call" => {
            let params = &req["params"];
            let Some(tool_name) = params.get("name").and_then(Value::as_str).filter(|s| !s.is_empty()) else {
                return Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32602,
                        "message": "Invalid params: missing string 'name'"
                    }
                }));
            };
            let raw_args = params.get("arguments");
            if let Some(v) = raw_args
                && !v.is_object()
                && !v.is_null()
            {
                return Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32602,
                        "message": "Invalid params: 'arguments' must be a JSON object"
                    }
                }));
            }
            let is_known_tool = matches!(
                tool_name,
                "robocopy2rust_environment"
                    | "robocopy2rust_presets"
                    | "robocopy2rust_build_command"
                    | "robocopy2rust_parse_command"
                    | "robocopy2rust_decode_exit_code"
                    | "robocopy2rust_profiles"
                    | "robocopy2rust_dry_run"
                    | "robocopy2rust_run"
                    | "robocopy2rust_report"
            );
            if !is_known_tool {
                return Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("Unknown tool: {tool_name}")
                    }
                }));
            }
            let default_obj = json!({});
            let args = raw_args.filter(|v| v.is_object()).unwrap_or(&default_obj);
            let (text, is_err) = match call_tool(tool_name, args) {
                Ok(val) => (serde_json::to_string_pretty(&val).unwrap_or_default(), false),
                Err(msg) => (msg, true),
            };
            Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [{ "type": "text", "text": text }],
                    "isError": is_err
                }
            }))
        }
        _ => id.filter(|v| !v.is_null()).map(|req_id| {
            json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {method}")
                }
            })
        }),
    }
}

pub fn handle_line(line: &str) -> Option<Value> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    match serde_json::from_str::<Value>(trimmed) {
        Ok(req) => handle_jsonrpc_request(&req),
        Err(e) => Some(json!({
            "jsonrpc": "2.0",
            "id": null,
            "error": { "code": -32700, "message": format!("Parse error: {e}") }
        })),
    }
}

pub fn run_stdio_server() -> Result<(), Box<dyn std::error::Error>> {
    info!(
        server = SERVER_NAME,
        version = SERVER_VERSION,
        "Starting robocopy2rust MCP stdio server"
    );
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line_res in stdin.lock().lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(e) => {
                error!(error = %e, "Failed reading MCP stdin line");
                break;
            }
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(trimmed) {
            Ok(req) => {
                if let Some(resp) = handle_jsonrpc_request(&req) {
                    let out = serde_json::to_string(&resp)?;
                    writeln!(stdout, "{out}")?;
                    stdout.flush()?;
                }
            }
            Err(e) => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32700, "message": format!("Parse error: {e}") }
                });
                writeln!(stdout, "{}", serde_json::to_string(&err_resp)?)?;
                stdout.flush()?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    use tempfile::TempDir;

    #[test]
    fn mcp_initialize_and_tools_list_return_nine_tools() {
        let init_resp = handle_jsonrpc_request(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize"
        }))
        .unwrap();
        assert_eq!(init_resp["result"]["protocolVersion"], MCP_PROTOCOL_VERSION);

        let list_resp = handle_jsonrpc_request(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        }))
        .unwrap();
        let tools = list_resp["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 9);
        let ro_count = tools
            .iter()
            .filter(|t| {
                t.get("annotations")
                    .and_then(|a| a.get("readOnlyHint"))
                    .and_then(Value::as_bool)
                    == Some(true)
            })
            .count();
        assert_eq!(ro_count, 6);
        for t in tools {
            let ann = t.get("annotations").and_then(Value::as_object).unwrap();
            assert!(ann.get("readOnlyHint").and_then(Value::as_bool).is_some());
            assert!(ann.get("destructiveHint").and_then(Value::as_bool).is_some());
        }
        let run_tool = tools.iter().find(|t| t["name"] == "robocopy2rust_run").unwrap();
        assert_eq!(run_tool["annotations"]["destructiveHint"], json!(true));
        assert_eq!(run_tool["annotations"]["readOnlyHint"], json!(false));

        let bad_args_resp = handle_jsonrpc_request(&json!({
            "jsonrpc": "2.0",
            "id": 98,
            "method": "tools/call",
            "params": {
                "name": "robocopy2rust_presets",
                "arguments": "not-an-object"
            }
        }))
        .unwrap();
        assert_eq!(bad_args_resp["error"]["code"], json!(-32602));

        let unknown_tool_resp = handle_jsonrpc_request(&json!({
            "jsonrpc": "2.0",
            "id": 99,
            "method": "tools/call",
            "params": {
                "name": "robocopy2rust_nonexistent",
                "arguments": {}
            }
        }))
        .unwrap();
        assert_eq!(unknown_tool_resp["error"]["code"], json!(-32601));
    }

    #[test]
    fn mcp_blocks_destructive_run_without_explicit_confirmation() {
        let resp = handle_jsonrpc_request(&json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "robocopy2rust_run",
                "arguments": {
                    "preset": "exact-mirror",
                    "source": "C:\\Src",
                    "destination": "D:\\Dst"
                }
            }
        }))
        .unwrap();
        assert_eq!(resp["result"]["isError"], true);
        let msg = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(msg.contains("confirm_destructive=true"));
    }

    #[cfg(windows)]
    #[test]
    fn mcp_dry_run_executes_real_robocopy_safely() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();
        std::fs::write(src.path().join("doc.txt"), b"hello mcp").unwrap();

        let resp = handle_jsonrpc_request(&json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "robocopy2rust_dry_run",
                "arguments": {
                    "source": src.path().display().to_string(),
                    "destination": dst.path().display().to_string()
                }
            }
        }))
        .unwrap();
        assert_eq!(resp["result"]["isError"], false);
        assert!(!dst.path().join("doc.txt").exists());
    }
}
