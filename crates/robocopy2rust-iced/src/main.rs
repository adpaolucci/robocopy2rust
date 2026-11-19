#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod message;
mod views;

use app::App;
use robocopy2rust_core::init_logging;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
                    "robocopy2rust-iced [--source DIR] [--dest DIR] [--rcj FILE] [--profile NAME]\n\
                     Launch the cross-platform Iced desktop console for robocopy2rust (Linux, macOS, Windows)."
                );
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

fn main() -> iced::Result {
    let _ = init_logging("warn", None);
    let launch = parse_launch_args_from(std::env::args().skip(1));

    iced::application(move || App::new(launch.clone()), App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .subscription(App::subscription)
        .window_size(iced::Size::new(1360.0, 840.0))
        .centered()
        .run()
}
