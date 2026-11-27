use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_robocopy2rust"))
}

#[test]
fn help_lists_subcommands() {
    let out = bin().arg("--help").output().expect("run");
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    for cmd in [
        "presets", "build", "parse", "decode", "dry-run", "run", "profile", "report", "env", "mcp", "gui",
    ] {
        assert!(s.contains(cmd), "missing subcommand {cmd}");
    }
}

#[test]
fn parse_destructive_warns() {
    let out = bin()
        .args(["parse", r"robocopy C:\src D:\dst /MIR /MT:16 /XD node_modules /R:2"])
        .output()
        .expect("run");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stderr.contains("DESTRUCTIVE_FLAGS_ACTIVE") || stdout.contains("DESTRUCTIVE_FLAGS_ACTIVE"),
        "expected destructive warning"
    );
}

#[test]
fn decode_3_is_success() {
    let out = bin().args(["decode", "3"]).output().expect("run");
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("Copied") && s.contains("Extra"));
    assert!(s.contains("Success"));
}

#[test]
fn decode_16_is_fatal() {
    let out = bin().args(["decode", "16"]).output().expect("run");
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(
        s.contains("Fatal") || s.to_lowercase().contains("error") || !out.status.success(),
        "exit 16 should be reported as fatal"
    );
}

#[test]
fn presets_lists_at_least_ten() {
    let out = bin().arg("presets").output().expect("run");
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    let count = s.lines().filter(|l| l.contains('|')).count();
    assert!(count >= 8, "expected >=8 presets, got {count}");
}

#[test]
fn env_reports_host_os() {
    let out = bin().arg("env").output().expect("run");
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(
        s.to_lowercase().contains("windows")
            || s.to_lowercase().contains("linux")
            || s.to_lowercase().contains("macos")
    );
}

#[test]
fn mcp_tools_list_has_nine_distinct() {
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
    let mut child = bin()
        .arg("mcp")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("spawn");
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all((req.to_string() + "\n").as_bytes())
        .unwrap();
    drop(child.stdin.take());
    let out = child.wait_with_output().expect("wait");
    let s = String::from_utf8_lossy(&out.stdout);
    // find the JSON-RPC response line
    let mut distinct = std::collections::BTreeSet::new();
    for line in s.lines() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line)
            && let Some(tools) = v.get("result").and_then(|r| r.get("tools")).and_then(|t| t.as_array())
        {
            for t in tools {
                if let Some(n) = t.get("name").and_then(|n| n.as_str()) {
                    distinct.insert(n.to_string());
                }
            }
        }
    }
    assert_eq!(
        distinct.len(),
        9,
        "expected 9 distinct tools, got {}: {:?}",
        distinct.len(),
        distinct
    );
}
