use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    pub os: String,
    pub is_windows: bool,
    pub robocopy_available: bool,
    pub robocopy_path: Option<PathBuf>,
    pub config_dir: PathBuf,
}

pub fn detect_environment() -> EnvironmentInfo {
    let is_windows = cfg!(target_os = "windows");
    let robocopy_path = find_robocopy_executable();
    let config_dir = dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("robocopy2rust");

    EnvironmentInfo {
        os: std::env::consts::OS.to_string(),
        is_windows,
        robocopy_available: robocopy_path.is_some(),
        robocopy_path,
        config_dir,
    }
}

pub fn find_robocopy_executable() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        let sys32 = PathBuf::from(r"C:\Windows\System32\robocopy.exe");
        if sys32.is_file() {
            return Some(sys32);
        }
    }

    if let Some(path_var) = std::env::var_os("PATH") {
        let candidates = if cfg!(target_os = "windows") {
            vec!["robocopy.exe", "robocopy"]
        } else {
            vec!["robocopy"]
        };
        for dir in std::env::split_paths(&path_var) {
            for bin in &candidates {
                let p = dir.join(bin);
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    None
}
