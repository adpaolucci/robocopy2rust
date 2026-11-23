#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(windows)]
mod win32_app;
#[cfg(windows)]
mod win32_dialogs;
#[cfg(any(windows, test))]
#[cfg_attr(not(windows), allow(dead_code))]
mod win32_state;

use robocopy2rust_core::init_logging;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(not(windows), allow(dead_code))]
pub struct LaunchArgs {
    pub source: Option<String>,
    pub dest: Option<String>,
    pub rcj: Option<PathBuf>,
    pub profile: Option<String>,
}

pub fn parse_launch_args_from<I>(args: I) -> LaunchArgs
where
    I: IntoIterator<Item = String>,
{
    let mut out = LaunchArgs::default();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--source" => {
                if let Some(s) = iter.next() {
                    out.source = Some(s);
                }
            }
            "--dest" => {
                if let Some(d) = iter.next() {
                    out.dest = Some(d);
                }
            }
            "--rcj" => {
                if let Some(r) = iter.next() {
                    out.rcj = Some(PathBuf::from(r));
                }
            }
            "--profile" => {
                if let Some(p) = iter.next() {
                    out.profile = Some(p);
                }
            }
            "--help" | "-h" => {
                println!(
                    "robocopy2rust-win32 [--source DIR] [--dest DIR] [--rcj FILE] [--profile NAME]\n\
                     Launch the native Win32/WinSafe Robocopy2Rust Management Console."
                );
                std::process::exit(0);
            }
            "--version" | "-V" => {
                println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            other if !other.starts_with('-') && other.to_ascii_lowercase().ends_with(".rcj") && out.rcj.is_none() => {
                out.rcj = Some(PathBuf::from(other));
            }
            _ => {}
        }
    }
    out
}

fn main() {
    let _ = init_logging("warn", None);
    let launch_args = parse_launch_args_from(std::env::args().skip(1));

    #[cfg(windows)]
    {
        if let Err(err) = win32_app::run(launch_args) {
            eprintln!("robocopy2rust-win32 fatal error: {err}");
            std::process::exit(1);
        }
    }

    #[cfg(not(windows))]
    {
        let _ = launch_args;
        eprintln!("robocopy2rust-win32 is a Windows-native Win32 binary. Run `robocopy2rust-iced` on Linux/macOS.");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_launch_args_and_positional_rcj() {
        let parsed = parse_launch_args_from(
            [
                "--source",
                r"C:\Src",
                "--dest",
                r"D:\Dst",
                "backup.rcj",
                "--profile",
                "nightly",
            ]
            .into_iter()
            .map(str::to_string),
        );
        assert_eq!(parsed.source.as_deref(), Some(r"C:\Src"));
        assert_eq!(parsed.dest.as_deref(), Some(r"D:\Dst"));
        assert_eq!(parsed.rcj, Some(PathBuf::from("backup.rcj")));
        assert_eq!(parsed.profile.as_deref(), Some("nightly"));
    }
}
