use crate::model::{
    CopyFlags, CopyOptions, DirCopyFlags, FileSelectionOptions, LoggingOptions, RetryOptions, RobocopyJob,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobPreset {
    pub id: String,
    pub name: String,
    pub category: String,
    pub summary: String,
    pub destructive: bool,
    pub job: RobocopyJob,
}

pub fn builtin_presets() -> Vec<JobPreset> {
    vec![
        JobPreset {
            id: "safe-backup".to_string(),
            name: "Safe Incremental Backup".to_string(),
            category: "Backup".to_string(),
            summary: "Copies all subdirectories (/E), preserves timestamps, skips older/extra deletions, 8 threads, /R:3 /W:5.".to_string(),
            destructive: false,
            job: RobocopyJob {
                name: "Safe Incremental Backup".to_string(),
                description: "Non-destructive recursive incremental backup".to_string(),
                copy: CopyOptions {
                    subdirectories_including_empty: true,
                    multithreaded: Some(8),
                    ..CopyOptions::default()
                },
                selection: FileSelectionOptions {
                    exclude_junctions: true,
                    ..FileSelectionOptions::default()
                },
                retry: RetryOptions {
                    retries: Some(3),
                    wait_seconds: Some(5),
                    ..RetryOptions::default()
                },
                logging: LoggingOptions {
                    show_eta: true,
                    ..LoggingOptions::default()
                },
                ..RobocopyJob::default()
            },
        },
        JobPreset {
            id: "exact-mirror".to_string(),
            name: "Exact Directory Mirror".to_string(),
            category: "Mirror".to_string(),
            summary: "Exact 1:1 mirror (/MIR) that deletes destination files no longer in source. Excludes junctions (/XJ).".to_string(),
            destructive: true,
            job: RobocopyJob {
                name: "Exact Directory Mirror".to_string(),
                description: "Mirrors source tree to destination and purges removed files".to_string(),
                copy: CopyOptions {
                    mirror: true,
                    subdirectories_including_empty: true,
                    purge: true,
                    multithreaded: Some(8),
                    ..CopyOptions::default()
                },
                selection: FileSelectionOptions {
                    exclude_junctions: true,
                    ..FileSelectionOptions::default()
                },
                retry: RetryOptions {
                    retries: Some(3),
                    wait_seconds: Some(5),
                    ..RetryOptions::default()
                },
                ..RobocopyJob::default()
            },
        },
        JobPreset {
            id: "dry-run-preview".to_string(),
            name: "Dry-Run Preview (/L)".to_string(),
            category: "Audit".to_string(),
            summary: "Simulates directory tree diff (/L /E /V /FP /BYTES) without writing or deleting any files.".to_string(),
            destructive: false,
            job: RobocopyJob {
                name: "Dry-Run Preview".to_string(),
                description: "Read-only simulation listing every planned copy, skip, or extra item".to_string(),
                copy: CopyOptions {
                    subdirectories_including_empty: true,
                    multithreaded: Some(8),
                    ..CopyOptions::default()
                },
                selection: FileSelectionOptions {
                    exclude_junctions: true,
                    ..FileSelectionOptions::default()
                },
                logging: LoggingOptions {
                    list_only: true,
                    verbose: true,
                    full_paths: true,
                    sizes_in_bytes: true,
                    report_extra: true,
                    ..LoggingOptions::default()
                },
                ..RobocopyJob::default()
            },
        },
        JobPreset {
            id: "fast-nvme".to_string(),
            name: "Fast LAN / NVMe Bulk Transfer".to_string(),
            category: "Performance".to_string(),
            summary: "High-throughput 32-thread unbuffered transfer (/E /J /MT:32 /NP /R:2 /W:2) ideal for large media & SSDs.".to_string(),
            destructive: false,
            job: RobocopyJob {
                name: "Fast LAN / NVMe Bulk Transfer".to_string(),
                description: "Maximum throughput multi-threaded unbuffered copy".to_string(),
                copy: CopyOptions {
                    subdirectories_including_empty: true,
                    unbuffered_io: true,
                    multithreaded: Some(32),
                    ..CopyOptions::default()
                },
                selection: FileSelectionOptions {
                    exclude_junctions: true,
                    ..FileSelectionOptions::default()
                },
                retry: RetryOptions {
                    retries: Some(2),
                    wait_seconds: Some(2),
                    ..RetryOptions::default()
                },
                logging: LoggingOptions {
                    no_progress: true,
                    ..LoggingOptions::default()
                },
                ..RobocopyJob::default()
            },
        },
        JobPreset {
            id: "enterprise-migration".to_string(),
            name: "Enterprise File Server Migration".to_string(),
            category: "Enterprise".to_string(),
            summary: "Preserves full NTFS ACLs, owners, auditing, and directory timestamps (/E /ZB /COPYALL /DCOPY:DAT /XJ).".to_string(),
            destructive: false,
            job: RobocopyJob {
                name: "Enterprise File Server Migration".to_string(),
                description: "Full-fidelity NTFS security & metadata migration".to_string(),
                copy: CopyOptions {
                    subdirectories_including_empty: true,
                    restartable_or_backup: true,
                    copy_flags: CopyFlags::copy_all(),
                    dir_copy_flags: DirCopyFlags {
                        data: true,
                        attributes: true,
                        timestamps: true,
                        ..DirCopyFlags::default()
                    },
                    multithreaded: Some(16),
                    ..CopyOptions::default()
                },
                selection: FileSelectionOptions {
                    exclude_junctions: true,
                    ..FileSelectionOptions::default()
                },
                retry: RetryOptions {
                    retries: Some(5),
                    wait_seconds: Some(10),
                    ..RetryOptions::default()
                },
                logging: LoggingOptions {
                    full_paths: true,
                    include_timestamps: true,
                    ..LoggingOptions::default()
                },
                ..RobocopyJob::default()
            },
        },
        JobPreset {
            id: "move-archive".to_string(),
            name: "Move & Archive (/MOVE)".to_string(),
            category: "Move".to_string(),
            summary: "Copies files and removes source files/directories once verified (/E /MOVE /R:3 /W:5).".to_string(),
            destructive: true,
            job: RobocopyJob {
                name: "Move & Archive".to_string(),
                description: "Moves files and subdirectories from source to destination".to_string(),
                copy: CopyOptions {
                    subdirectories_including_empty: true,
                    move_files_and_dirs: true,
                    multithreaded: Some(8),
                    ..CopyOptions::default()
                },
                selection: FileSelectionOptions {
                    exclude_junctions: true,
                    ..FileSelectionOptions::default()
                },
                retry: RetryOptions {
                    retries: Some(3),
                    wait_seconds: Some(5),
                    ..RetryOptions::default()
                },
                ..RobocopyJob::default()
            },
        },
        JobPreset {
            id: "wan-sync".to_string(),
            name: "Low-Bandwidth WAN Sync".to_string(),
            category: "Network".to_string(),
            summary: "Restartable SMB-compressed transfer with 20ms packet pacing and FAT timestamp tolerance (/E /Z /COMPRESS /IPG:20 /FFT).".to_string(),
            destructive: false,
            job: RobocopyJob {
                name: "Low-Bandwidth WAN Sync".to_string(),
                description: "Bandwidth-friendly restartable copy over VPN/WAN links".to_string(),
                copy: CopyOptions {
                    subdirectories_including_empty: true,
                    restartable: true,
                    network_compress: true,
                    inter_packet_gap_ms: Some(20),
                    multithreaded: None, // /MT incompatible with /IPG
                    ..CopyOptions::default()
                },
                selection: FileSelectionOptions {
                    fat_file_times: true,
                    exclude_junctions: true,
                    ..FileSelectionOptions::default()
                },
                retry: RetryOptions {
                    retries: Some(10),
                    wait_seconds: Some(15),
                    wait_for_sharenames: true,
                    ..RetryOptions::default()
                },
                ..RobocopyJob::default()
            },
        },
        JobPreset {
            id: "lfsm-safe".to_string(),
            name: "Low Free Space Safe Copy (/LFSM)".to_string(),
            category: "Safety".to_string(),
            summary: "Pauses automatically if destination free space drops below 1GB (/E /LFSM:1G /R:5 /W:10).".to_string(),
            destructive: false,
            job: RobocopyJob {
                name: "Low Free Space Safe Copy".to_string(),
                description: "Prevents filling destination volume past 1 GB floor".to_string(),
                copy: CopyOptions {
                    subdirectories_including_empty: true,
                    multithreaded: None, // /MT incompatible with /LFSM
                    ..CopyOptions::default()
                },
                selection: FileSelectionOptions {
                    exclude_junctions: true,
                    ..FileSelectionOptions::default()
                },
                retry: RetryOptions {
                    retries: Some(5),
                    wait_seconds: Some(10),
                    low_free_space_mode: true,
                    low_free_space_floor: Some("1G".to_string()),
                    ..RetryOptions::default()
                },
                ..RobocopyJob::default()
            },
        },
    ]
}

pub fn find_preset(id_or_name: &str) -> Option<JobPreset> {
    let needle = id_or_name.trim().to_ascii_lowercase();
    builtin_presets()
        .into_iter()
        .find(|p| p.id.to_ascii_lowercase() == needle || p.name.to_ascii_lowercase() == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{ValidationSeverity, validate_job};

    #[test]
    fn builtin_presets_have_no_configuration_errors_once_paths_set() {
        let presets = builtin_presets();
        assert_eq!(presets.len(), 8);
        for mut preset in presets {
            preset.job.source = r"C:\Src".to_string();
            preset.job.destination = r"D:\Dst".to_string();
            let errors: Vec<_> = validate_job(&preset.job)
                .into_iter()
                .filter(|f| f.severity == ValidationSeverity::Error)
                .collect();
            assert!(errors.is_empty(), "preset {} had errors: {errors:?}", preset.id);
        }
    }
}
