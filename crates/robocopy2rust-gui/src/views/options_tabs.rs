use crate::state::AppState;
use crate::views::{
    file_attr_editor, opt_string_field, opt_u16_field, opt_u32_field, opt_u64_field, win32_group_box, win32_subheader,
};
use eframe::egui::{self, Color32, RichText, ScrollArea};
use robocopy2rust_core::{CopyFlags, LogFileMode, SparseMode};

pub fn render_copy_transfer_tab(ui: &mut egui::Ui, state: &mut AppState) {
    let mut changed = false;

    ui.vertical(|ui| {
        win32_subheader(ui, "Copy & Transfer Options (/S, /E, /MIR, /COPY, /DCOPY, /MT, /MOVE, /Z, /B)", |ui| {
            if ui.small_button("Reset Copy Defaults").clicked() {
                state.job.copy = robocopy2rust_core::CopyOptions::default();
                changed = true;
            }
        });
        ui.add_space(4.0);

        ScrollArea::vertical().show(ui, |ui| {
            // 1. Directory Recursion & Deletion Mode
            win32_group_box(ui, "Directory Traversal & Sync Mode", |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .checkbox(&mut state.job.copy.subdirectories_non_empty, "/S (Subdirs, skip empty)")
                        .on_hover_text("Copy subdirectories, but not empty ones.")
                        .changed()
                    {
                        if state.job.copy.subdirectories_non_empty {
                            state.job.copy.subdirectories_including_empty = false;
                            state.job.copy.mirror = false;
                        }
                        changed = true;
                    }
                    if ui
                        .checkbox(
                            &mut state.job.copy.subdirectories_including_empty,
                            "/E (Subdirs including empty)",
                        )
                        .on_hover_text("Copy subdirectories, including empty ones.")
                        .changed()
                    {
                        if state.job.copy.subdirectories_including_empty {
                            state.job.copy.subdirectories_non_empty = false;
                        }
                        changed = true;
                    }
                    if ui
                        .checkbox(&mut state.job.copy.purge, "/PURGE (Delete dest extras)")
                        .on_hover_text("Delete destination files and directories that no longer exist in source.")
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .checkbox(
                            &mut state.job.copy.mirror,
                            RichText::new("/MIR (Mirror = /E + /PURGE)").color(Color32::from_rgb(153, 27, 27)),
                        )
                        .on_hover_text("Mirror a directory tree (equivalent to /E plus /PURGE).")
                        .changed()
                    {
                        if state.job.copy.mirror {
                            state.job.copy.subdirectories_including_empty = true;
                            state.job.copy.subdirectories_non_empty = false;
                            state.job.copy.purge = true;
                        }
                        changed = true;
                    }
                });

                ui.add_space(2.0);
                ui.horizontal_wrapped(|ui| {
                    changed |= opt_u32_field(
                        ui,
                        "/LEV:n (Max depth levels)",
                        "Only copy the top n levels of the source directory tree.",
                        &mut state.job.copy.max_levels,
                        2,
                        1..=256,
                    );
                    ui.separator();
                    if ui
                        .checkbox(&mut state.job.copy.move_files, "/MOV (Move files)")
                        .on_hover_text("Move files (delete from source after copying).")
                        .changed()
                    {
                        changed = true;
                    }
                    if ui
                        .checkbox(
                            &mut state.job.copy.move_files_and_dirs,
                            RichText::new("/MOVE (Move files & dirs)").color(Color32::from_rgb(153, 27, 27)),
                        )
                        .on_hover_text("Move files AND directories (delete from source after copying).")
                        .changed()
                    {
                        changed = true;
                    }
                });
            });

            ui.add_space(6.0);

            // 2. Access Mode, Threading & I/O Behavior
            win32_group_box(ui, "Access Mode, Multi-Threading & Transfer Engine", |ui| {
                ui.horizontal_wrapped(|ui| {
                    changed |= opt_u16_field(
                        ui,
                        "/MT[:n] (Multi-threaded 1..128)",
                        "Do multi-threaded copies with n threads (1..=128, default 8). Incompatible with /IPG, /EFSRAW, /LFSM.",
                        &mut state.job.copy.multithreaded,
                        8,
                        1..=128,
                    );
                    ui.separator();
                    changed |= ui
                        .checkbox(&mut state.job.copy.restartable, "/Z (Restartable)")
                        .on_hover_text("Copy files in restartable mode.")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.copy.backup_mode, "/B (Backup mode)")
                        .on_hover_text("Copy files in Backup mode (requires SeBackupPrivilege/Admin).")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.copy.restartable_or_backup, "/ZB (Restartable else Backup)")
                        .on_hover_text("Use restartable mode; if access denied use Backup mode.")
                        .changed();
                });

                ui.horizontal_wrapped(|ui| {
                    changed |= ui
                        .checkbox(&mut state.job.copy.unbuffered_io, "/J (Unbuffered I/O)")
                        .on_hover_text("Copy using unbuffered I/O (recommended for very large files).")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.copy.efs_raw, "/EFSRAW (EFS RAW)")
                        .on_hover_text("Copy all encrypted files in EFS RAW mode.")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.copy.network_compress, "/COMPRESS (SMB Compress)")
                        .on_hover_text("Request network compression during file transfer if supported.")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.copy.no_offload, "/NOOFFLOAD (No copy offload)")
                        .on_hover_text("Copy files without using the Windows Copy Offload mechanism.")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.copy.no_clone, "/NOCLONE (No block cloning)")
                        .on_hover_text("Do not attempt ReFS block cloning as an optimization.")
                        .changed();
                });

                ui.horizontal(|ui| {
                    ui.label("/SPARSE (Sparse state):");
                    for (mode, lbl) in [
                        (SparseMode::Default, "Default"),
                        (SparseMode::Enable, "/SPARSE:Y (Preserve)"),
                        (SparseMode::Disable, "/SPARSE:N (Disable)"),
                    ] {
                        if ui.selectable_label(state.job.copy.sparse_mode == mode, lbl).clicked() {
                            state.job.copy.sparse_mode = mode;
                            changed = true;
                        }
                    }
                });
            });

            ui.add_space(6.0);

            // 3. Copy Flags (/COPY, /DCOPY)
            win32_group_box(ui, "File & Directory Metadata Flags (/COPY & /DCOPY)", |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("File /COPY:").strong());
                    changed |= ui.checkbox(&mut state.job.copy.copy_flags.data, "D (Data)").changed();
                    changed |= ui.checkbox(&mut state.job.copy.copy_flags.attributes, "A (Attr)").changed();
                    changed |= ui.checkbox(&mut state.job.copy.copy_flags.timestamps, "T (Times)").changed();
                    changed |= ui.checkbox(&mut state.job.copy.copy_flags.security, "S (Security/ACLs)").changed();
                    changed |= ui.checkbox(&mut state.job.copy.copy_flags.owner, "O (Owner)").changed();
                    changed |= ui.checkbox(&mut state.job.copy.copy_flags.auditing, "U (Audit)").changed();
                    changed |= ui
                        .checkbox(&mut state.job.copy.copy_flags.skip_alt_data_streams, "X (Skip ADS)")
                        .on_hover_text("Skip alternate data streams (ignored if /B or /ZB).")
                        .changed();

                    ui.separator();
                    if ui.small_button("DAT (Default)").clicked() {
                        state.job.copy.copy_flags = CopyFlags::default();
                        state.job.copy.no_copy = false;
                        changed = true;
                    }
                    if ui.small_button("/SEC (DATS)").clicked() {
                        state.job.copy.copy_flags = CopyFlags::sec();
                        state.job.copy.no_copy = false;
                        changed = true;
                    }
                    if ui.small_button("/COPYALL").clicked() {
                        state.job.copy.copy_flags = CopyFlags::copy_all();
                        state.job.copy.no_copy = false;
                        changed = true;
                    }
                    changed |= ui
                        .checkbox(&mut state.job.copy.no_copy, "/NOCOPY")
                        .on_hover_text("Copy no file info (useful with /PURGE).")
                        .changed();
                });

                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Dir /DCOPY:").strong());
                    changed |= ui.checkbox(&mut state.job.copy.dir_copy_flags.data, "D (Data)").changed();
                    changed |= ui.checkbox(&mut state.job.copy.dir_copy_flags.attributes, "A (Attr)").changed();
                    changed |= ui.checkbox(&mut state.job.copy.dir_copy_flags.timestamps, "T (Times)").changed();
                    changed |= ui
                        .checkbox(&mut state.job.copy.dir_copy_flags.extended_attributes, "E (EAs)")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.copy.dir_copy_flags.skip_alt_data_streams, "X (Skip ADS)")
                        .changed();
                    ui.separator();
                    changed |= ui
                        .checkbox(&mut state.job.copy.no_dir_copy, "/NODCOPY")
                        .on_hover_text("Copy no directory info (default is /DCOPY:DA).")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.copy.sec_fix, "/SECFIX")
                        .on_hover_text("Fix file security on all files, even skipped files.")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.copy.tim_fix, "/TIMFIX")
                        .on_hover_text("Fix file times on all files, even skipped files.")
                        .changed();
                });
            });

            ui.add_space(6.0);

            // 4. Attributes, Links, Naming & Tree Structure
            win32_group_box(ui, "Attributes, Symbolic Links & Path Rules", |ui| {
                changed |= file_attr_editor(ui, "/A+: (Add Attrs):   ", &mut state.job.copy.add_attributes);
                changed |= file_attr_editor(ui, "/A-: (Remove Attrs):", &mut state.job.copy.remove_attributes);

                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    changed |= ui
                        .checkbox(&mut state.job.copy.create_tree_only, "/CREATE (Zero-length skeleton)")
                        .on_hover_text("Create directory tree and zero-length placeholder files only.")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.copy.fat_names, "/FAT (8.3 filenames)")
                        .on_hover_text("Create destination files using 8.3 FAT file names only.")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.copy.disable_long_paths, "/256 (Disable >256 paths)")
                        .on_hover_text("Turn off very long path (> 256 characters) support.")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.copy.copy_junctions, "/SJ (Copy junctions as junctions)")
                        .on_hover_text("Copy junctions as junctions instead of junction targets.")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.copy.copy_symlinks, "/SL (Copy symlinks as links)")
                        .on_hover_text("Copy symbolic links as links instead of link targets.")
                        .changed();
                });
            });
        });
    });

    if changed {
        state.refresh_derived();
    }
}

pub fn render_file_selection_tab(ui: &mut egui::Ui, state: &mut AppState) {
    let mut changed = false;

    ui.vertical(|ui| {
        win32_subheader(ui, "File Selection, Exclusions, Size, Age & Junction Filters", |ui| {
            if ui.small_button("Reset Selection Defaults").clicked() {
                state.job.selection = robocopy2rust_core::FileSelectionOptions::default();
                state.exclude_files_text.clear();
                state.exclude_dirs_text.clear();
                changed = true;
            }
        });
        ui.add_space(4.0);

        ScrollArea::vertical().show(ui, |ui| {
            // 1. Exclusions (/XF, /XD, /XJ)
            win32_group_box(ui, "Excluded Files, Directories & Reparse Points", |ui| {
                ui.horizontal(|ui| {
                    ui.label("/XF (Exclude Files):");
                    let resp = ui
                        .add_sized(
                            [ui.available_width().max(240.0), 20.0],
                            egui::TextEdit::singleline(&mut state.exclude_files_text)
                                .hint_text(r#"e.g. *.tmp *.log ~$* "Thumbs.db""#),
                        )
                        .on_hover_text("Space or comma separated file names, paths, or wildcards to exclude.");
                    if resp.changed() {
                        state.sync_text_lists_to_job();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("/XD (Exclude Dirs): ");
                    let resp = ui
                        .add_sized(
                            [ui.available_width().max(240.0), 20.0],
                            egui::TextEdit::singleline(&mut state.exclude_dirs_text)
                                .hint_text(r#"e.g. .git node_modules target "System Volume Information""#),
                        )
                        .on_hover_text("Space or comma separated directories or paths to exclude.");
                    if resp.changed() {
                        state.sync_text_lists_to_job();
                    }
                });

                ui.horizontal_wrapped(|ui| {
                    changed |= ui
                        .checkbox(
                            &mut state.job.selection.exclude_junctions,
                            "/XJ (Exclude all symlinks & junctions)",
                        )
                        .on_hover_text("Exclude symbolic links (for both files and dirs) and junction points.")
                        .changed();
                    changed |= ui
                        .checkbox(
                            &mut state.job.selection.exclude_dir_junctions,
                            "/XJD (Exclude dir junctions/symlinks)",
                        )
                        .changed();
                    changed |= ui
                        .checkbox(
                            &mut state.job.selection.exclude_file_symlinks,
                            "/XJF (Exclude file symlinks)",
                        )
                        .changed();
                });
            });

            ui.add_space(6.0);

            // 2. File Classification Toggles (/XC, /XN, /XO, /XX, /XL, /IS, /IT, /IM)
            win32_group_box(ui, "File Comparison Classes & Timestamp Granularity", |ui| {
                ui.horizontal_wrapped(|ui| {
                    changed |= ui
                        .checkbox(&mut state.job.selection.exclude_changed, "/XC (Exclude Changed)")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.selection.exclude_newer, "/XN (Exclude Newer)")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.selection.exclude_older, "/XO (Exclude Older)")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.selection.exclude_extra, "/XX (Exclude Extra)")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.selection.exclude_lonely, "/XL (Exclude Lonely)")
                        .changed();
                });
                ui.horizontal_wrapped(|ui| {
                    changed |= ui
                        .checkbox(&mut state.job.selection.include_same, "/IS (Include Same)")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.selection.include_tweaked, "/IT (Include Tweaked)")
                        .changed();
                    changed |= ui
                        .checkbox(
                            &mut state.job.selection.include_modified,
                            "/IM (Include Modified change-times)",
                        )
                        .changed();
                    ui.separator();
                    changed |= ui
                        .checkbox(&mut state.job.selection.fat_file_times, "/FFT (2s FAT granularity)")
                        .on_hover_text(
                            "Assume FAT file times (2-second granularity) — useful for NAS/Linux Samba shares.",
                        )
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.selection.compensate_dst, "/DST (Compensate 1h DST)")
                        .changed();
                });
            });

            ui.add_space(6.0);

            // 3. Attribute Filters (/A, /M, /IA, /XA)
            win32_group_box(ui, "Archive Bit & NTFS Attribute Filters", |ui| {
                ui.horizontal_wrapped(|ui| {
                    changed |= ui
                        .checkbox(&mut state.job.selection.archive_only, "/A (Archive bit set)")
                        .changed();
                    changed |= ui
                        .checkbox(
                            &mut state.job.selection.archive_and_reset,
                            "/M (Archive bit set & reset)",
                        )
                        .changed();
                });
                changed |= file_attr_editor(ui, "/IA: (Include Attrs):", &mut state.job.selection.include_attrs);
                changed |= file_attr_editor(ui, "/XA: (Exclude Attrs):", &mut state.job.selection.exclude_attrs);
            });

            ui.add_space(6.0);

            // 4. Size & Age Bounds (/MAX, /MIN, /MAXAGE, /MINAGE, /MAXLAD, /MINLAD)
            win32_group_box(ui, "File Size (Bytes) & Age / Last Access Date Filters", |ui| {
                ui.horizontal_wrapped(|ui| {
                    changed |= opt_u64_field(
                        ui,
                        "/MAX:n (Max bytes)",
                        "Exclude files bigger than n bytes.",
                        &mut state.job.selection.max_size_bytes,
                        10_485_760,
                    );
                    ui.separator();
                    changed |= opt_u64_field(
                        ui,
                        "/MIN:n (Min bytes)",
                        "Exclude files smaller than n bytes.",
                        &mut state.job.selection.min_size_bytes,
                        1024,
                    );
                });

                ui.add_space(3.0);
                ui.horizontal_wrapped(|ui| {
                    changed |= opt_string_field(
                        ui,
                        "/MAXAGE:",
                        "days or YYYYMMDD",
                        "Exclude files older than n days (<1900) or YYYYMMDD date.",
                        &mut state.job.selection.max_age,
                        115.0,
                    );
                    changed |= opt_string_field(
                        ui,
                        "/MINAGE:",
                        "days or YYYYMMDD",
                        "Exclude files newer than n days (<1900) or YYYYMMDD date.",
                        &mut state.job.selection.min_age,
                        115.0,
                    );
                });
                ui.horizontal_wrapped(|ui| {
                    changed |= opt_string_field(
                        ui,
                        "/MAXLAD:",
                        "days or YYYYMMDD",
                        "Exclude files unused since n days or YYYYMMDD.",
                        &mut state.job.selection.max_last_access,
                        115.0,
                    );
                    changed |= opt_string_field(
                        ui,
                        "/MINLAD:",
                        "days or YYYYMMDD",
                        "Exclude files used since n days or YYYYMMDD.",
                        &mut state.job.selection.min_last_access,
                        115.0,
                    );
                });
            });
        });
    });

    if changed {
        state.refresh_derived();
    }
}

pub fn render_perf_retry_tab(ui: &mut egui::Ui, state: &mut AppState) {
    let mut changed = false;

    ui.vertical(|ui| {
        win32_subheader(ui, "Performance Throttling, Monitoring, Run Hours & Retry Policies", |ui| {
            if ui.small_button("Reset Retry & Throttle Defaults").clicked() {
                state.job.retry = robocopy2rust_core::RetryOptions::default();
                state.job.throttle = robocopy2rust_core::ThrottleOptions::default();
                changed = true;
            }
        });
        ui.add_space(4.0);

        ScrollArea::vertical().show(ui, |ui| {
            // 1. Retry & Low Free Space
            win32_group_box(ui, "Retry & Disk Space Protection (/R, /W, /REG, /TBD, /LFSM)", |ui| {
                ui.horizontal_wrapped(|ui| {
                    changed |= opt_u32_field(
                        ui,
                        "/R:n (Retries)",
                        "Number of retries on failed copies (Robocopy native default without /R is 1,000,000!).",
                        &mut state.job.retry.retries,
                        3,
                        0..=1_000_000,
                    );
                    ui.separator();
                    changed |= opt_u32_field(
                        ui,
                        "/W:n (Wait seconds)",
                        "Wait time in seconds between retries.",
                        &mut state.job.retry.wait_seconds,
                        5,
                        0..=3600,
                    );
                    ui.separator();
                    changed |= ui
                        .checkbox(&mut state.job.retry.wait_for_sharenames, "/TBD (Wait for sharenames)")
                        .on_hover_text("Wait for sharenames to be defined (retry error 67).")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.retry.save_defaults_to_registry, "/REG (Save /R & /W to Registry)")
                        .changed();
                });

                ui.add_space(3.0);
                ui.horizontal_wrapped(|ui| {
                    changed |= ui
                        .checkbox(&mut state.job.retry.low_free_space_mode, "/LFSM (Low Free Space Mode)")
                        .on_hover_text("Pause copying if destination volume free space falls below floor (incompatible with /MT and /EFSRAW).")
                        .changed();
                    changed |= opt_string_field(
                        ui,
                        "/LFSM:n[KMG] Floor:",
                        "e.g. 500M or 2G",
                        "Optional explicit free space floor (default 10% of destination volume).",
                        &mut state.job.retry.low_free_space_floor,
                        120.0,
                    );
                });
            });

            ui.add_space(6.0);

            // 2. I/O Throttling & Packet Gap
            win32_group_box(ui, "I/O Throttling & Bandwidth Control (/IoMaxSize, /IoRate, /Threshold, /IPG)", |ui| {
                ui.horizontal_wrapped(|ui| {
                    changed |= opt_string_field(
                        ui,
                        "/IoMaxSize:",
                        "e.g. 64K or 1M",
                        "Requested max I/O size per read/write cycle in n[KMG] bytes.",
                        &mut state.job.throttle.io_max_size,
                        110.0,
                    );
                    changed |= opt_string_field(
                        ui,
                        "/IoRate:",
                        "e.g. 25M",
                        "Requested max I/O rate per second in n[KMG] bytes/sec.",
                        &mut state.job.throttle.io_rate,
                        110.0,
                    );
                    changed |= opt_string_field(
                        ui,
                        "/Threshold:",
                        "e.g. 10M",
                        "Minimum file size threshold for engaging throttling in n[KMG] bytes.",
                        &mut state.job.throttle.threshold,
                        110.0,
                    );
                });

                ui.add_space(3.0);
                changed |= opt_u32_field(
                    ui,
                    "/IPG:n (Inter-Packet Gap ms)",
                    "Inter-packet gap in milliseconds to free bandwidth on slow WAN links (incompatible with /MT).",
                    &mut state.job.copy.inter_packet_gap_ms,
                    20,
                    1..=10_000,
                );
            });

            ui.add_space(6.0);

            // 3. Continuous Monitoring & Run Hours
            win32_group_box(ui, "Continuous Monitoring & Scheduled Run Windows (/MON, /MOT, /RH, /PF)", |ui| {
                ui.horizontal_wrapped(|ui| {
                    changed |= opt_u32_field(
                        ui,
                        "/MON:n (Re-run after n changes)",
                        "Monitor source; run again when more than n changes seen.",
                        &mut state.job.copy.monitor_changes,
                        1,
                        1..=100_000,
                    );
                    ui.separator();
                    changed |= opt_u32_field(
                        ui,
                        "/MOT:m (Re-run every m minutes)",
                        "Monitor source; run again in m minutes if changed.",
                        &mut state.job.copy.monitor_minutes,
                        5,
                        1..=1440,
                    );
                });

                ui.add_space(3.0);
                ui.horizontal_wrapped(|ui| {
                    changed |= opt_string_field(
                        ui,
                        "/RH:hhmm-hhmm (Run Hours):",
                        "e.g. 2200-0600",
                        "24-hour window during which new copies may start.",
                        &mut state.job.copy.run_hours,
                        120.0,
                    );
                    changed |= ui
                        .checkbox(&mut state.job.copy.per_file_run_hours, "/PF (Check run hours Per File)")
                        .on_hover_text("Check run hours on a per-file rather than per-pass basis.")
                        .changed();
                });
            });
        });
    });

    if changed {
        state.refresh_derived();
    }
}

pub fn render_logging_jobs_tab(ui: &mut egui::Ui, state: &mut AppState) {
    let mut changed = false;

    ui.vertical(|ui| {
        win32_subheader(ui, "Logging Output, Robocopy Job Directives & Extra Switches", |ui| {
            if ui.small_button("Reset Logging Defaults").clicked() {
                state.job.logging = robocopy2rust_core::LoggingOptions::default();
                state.job.job_files = robocopy2rust_core::JobFileOptions::default();
                state.extra_args_text.clear();
                changed = true;
            }
        });
        ui.add_space(4.0);

        ScrollArea::vertical().show(ui, |ui| {
            // 1. Output Verbosity & Format
            win32_group_box(ui, "Console & Log Output Flags", |ui| {
                ui.horizontal_wrapped(|ui| {
                    changed |= ui
                        .checkbox(&mut state.job.logging.list_only, "/L (List only - Dry Run)")
                        .on_hover_text("List only: do not copy, timestamp, or delete any files.")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.logging.report_extra, "/X (Report all extras)")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.logging.verbose, "/V (Verbose skipped files)")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.logging.include_timestamps, "/TS (Source timestamps)")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.logging.full_paths, "/FP (Full pathnames)")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.logging.sizes_in_bytes, "/BYTES (Sizes in bytes)")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.logging.unicode_output, "/UNICODE (Unicode console)")
                        .changed();
                });

                ui.horizontal_wrapped(|ui| {
                    changed |= ui.checkbox(&mut state.job.logging.no_sizes, "/NS (No sizes)").changed();
                    changed |= ui
                        .checkbox(&mut state.job.logging.no_classes, "/NC (No classes)")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.logging.no_file_list, "/NFL (No file list)")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.logging.no_dir_list, "/NDL (No dir list)")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.logging.no_progress, "/NP (No % progress)")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.logging.show_eta, "/ETA (Show ETA)")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.logging.no_job_header, "/NJH (No header)")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.logging.no_job_summary, "/NJS (No summary)")
                        .changed();
                    changed |= ui
                        .checkbox(&mut state.job.logging.tee, "/TEE (Log + console)")
                        .changed();
                });
            });

            ui.add_space(6.0);

            // 2. Log File Target
            win32_group_box(ui, "External Log File (/LOG, /LOG+, /UNILOG, /UNILOG+)", |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Mode:");
                    for (mode, lbl) in [
                        (LogFileMode::None, "None"),
                        (LogFileMode::Overwrite, "/LOG:"),
                        (LogFileMode::Append, "/LOG+:"),
                        (LogFileMode::UnicodeOverwrite, "/UNILOG:"),
                        (LogFileMode::UnicodeAppend, "/UNILOG+:"),
                    ] {
                        if ui.selectable_label(state.job.logging.log_mode == mode, lbl).clicked() {
                            state.job.logging.log_mode = mode;
                            changed = true;
                        }
                    }
                });

                ui.horizontal(|ui| {
                    changed |= opt_string_field(
                        ui,
                        "Log File Path:",
                        r"C:\logs\robocopy.log",
                        "File path for Robocopy native log output.",
                        &mut state.job.logging.log_path,
                        340.0,
                    );
                    if ui.button("Browse...").clicked()
                        && let Some(p) = rfd::FileDialog::new().set_file_name("robocopy.log").save_file()
                    {
                        state.job.logging.log_path = Some(p.display().to_string());
                        if state.job.logging.log_mode == LogFileMode::None {
                            state.job.logging.log_mode = LogFileMode::Overwrite;
                        }
                        changed = true;
                    }
                });
            });

            ui.add_space(6.0);

            // 3. Job File Directives & Extra Args
            win32_group_box(ui, "Robocopy .rcj Job Directives & Custom Extra Switches", |ui| {
                ui.horizontal_wrapped(|ui| {
                    changed |= opt_string_field(
                        ui,
                        "/JOB:",
                        "jobname_or_path",
                        "Take parameters from named JOB file.",
                        &mut state.job.job_files.load_job,
                        180.0,
                    );
                    changed |= opt_string_field(
                        ui,
                        "/SAVE:",
                        "jobname_or_path",
                        "Save parameters to named JOB file.",
                        &mut state.job.job_files.save_job,
                        180.0,
                    );
                });

                ui.horizontal_wrapped(|ui| {
                    changed |= ui
                        .checkbox(&mut state.job.job_files.quit_after_parse, "/QUIT")
                        .changed();
                    changed |= ui.checkbox(&mut state.job.job_files.no_source_dir, "/NOSD").changed();
                    changed |= ui.checkbox(&mut state.job.job_files.no_dest_dir, "/NODD").changed();
                });

                ui.add_space(3.0);
                ui.horizontal(|ui| {
                    ui.label("Extra Raw Args:");
                    if ui
                        .add_sized(
                            [ui.available_width().max(220.0), 20.0],
                            egui::TextEdit::singleline(&mut state.extra_args_text)
                                .hint_text("Any additional raw switches passed verbatim"),
                        )
                        .changed()
                    {
                        state.sync_text_lists_to_job();
                    }
                });
            });
        });
    });

    if changed {
        state.refresh_derived();
    }
}
