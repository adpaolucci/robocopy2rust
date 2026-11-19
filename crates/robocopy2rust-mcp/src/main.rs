fn main() {
    let _ = robocopy2rust_core::logging::init_logging("warn", None);
    if let Err(e) = robocopy2rust_mcp::run_stdio_server() {
        eprintln!("robocopy2rust-mcp fatal error: {e}");
        std::process::exit(1);
    }
}
