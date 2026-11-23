#![forbid(unsafe_code)]

use robocopy2rust_mcp::{SERVER_NAME, SERVER_VERSION, get_tools_manifest, run_stdio_server};
use std::process;

fn main() {
    let mut show_tools = false;
    let mut log_level = "warn".to_string();
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--tools" => show_tools = true,
            "--log-level" => {
                if let Some(lvl) = args.next() {
                    log_level = lvl;
                }
            }
            "--help" | "-h" => {
                println!(
                    "{SERVER_NAME} {SERVER_VERSION}\n\
                     Model Context Protocol (MCP) JSON-RPC 2.0 stdio server for robocopy2rust\n\n\
                     USAGE:\n\
                       {SERVER_NAME} [OPTIONS]\n\n\
                     OPTIONS:\n\
                       --tools              Print available MCP tools in JSON schema format and exit\n\
                       --log-level <LEVEL>  Log level filter sent to stderr [default: warn]\n\
                       -h, --help           Print help\n\
                       -V, --version        Print version"
                );
                process::exit(0);
            }
            "--version" | "-V" => {
                println!("{SERVER_NAME} {SERVER_VERSION}");
                process::exit(0);
            }
            _ => {}
        }
    }

    if show_tools {
        let manifest = get_tools_manifest();
        println!("{}", serde_json::to_string_pretty(&manifest).unwrap_or_default());
        process::exit(0);
    }

    let _ = robocopy2rust_core::logging::init_logging(&log_level, None);
    if let Err(e) = run_stdio_server() {
        eprintln!("robocopy2rust-mcp fatal error: {e}");
        process::exit(1);
    }
}
