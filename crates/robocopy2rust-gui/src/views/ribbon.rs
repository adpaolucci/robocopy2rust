use crate::state::{ActiveModal, AppState, WorkspaceTab, set_clipboard_text};
use eframe::egui::{self, Color32, RichText, Stroke};
use robocopy2rust_core::{RobocopyJob, build_batch_script, build_powershell_script, parse_rcj, serialize_rcj};
use robocopy2rust_report::ReportFormat;
use std::path::PathBuf;

pub fn pick_folder(current: &str, title: &str) -> Option<PathBuf> {
    let mut dlg = rfd::FileDialog::new().set_title(title);
    let trimmed = current.trim().trim_matches('"').trim();
    if !trimmed.is_empty() {
        let p = PathBuf::from(trimmed);
        if p.is_dir() {
            dlg = dlg.set_directory(&p);
        } else if let Some(parent) = p.parent()
            && parent.is_dir()
        {
            dlg = dlg.set_directory(parent);
        }
    }
    dlg.pick_folder()
}

pub fn render_ribbon(ctx: &egui::Context, state: &mut AppState) {
    // 1. Classic Win32 Menu Bar
    egui::TopBottomPanel::top("top_menu_bar")
        .frame(
            egui::Frame::NONE
                .fill(Color32::from_rgb(240, 240, 240))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(200, 200, 200)))
                .inner_margin(egui::Margin::symmetric(4, 2)),
        )
        .show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Select Source Folder...").clicked() {
                        ui.close();
                        if let Some(p) = pick_folder(&state.job.source, "Select Robocopy Source Folder") {
                            state.job.source = p.display().to_string();
                            state.refresh_derived();
                        }
                    }
                    if ui.button("Select Destination Folder...").clicked() {
                        ui.close();
                        if let Some(p) = pick_folder(&state.job.destination, "Select Robocopy Destination Folder") {
                            state.job.destination = p.display().to_string();
                            state.refresh_derived();
                        }
                    }
                    ui.separator();
                    if ui.button("Import .rcj Job File...").clicked() {
                        ui.close();
                        if let Some(path) = rfd::FileDialog::new().add_filter("Robocopy Job", &["rcj"]).pick_file() {
                            match std::fs::read(&path)
                                .map_err(anyhow::Error::from)
                                .and_then(|b| parse_rcj(&b))
                            {
                                Ok((job, warnings)) => {
                                    state.set_job(job);
                                    state.status_message = if warnings.is_empty() {
                                        format!("Imported .rcj: {}", path.display())
                                    } else {
                                        format!("Imported .rcj with {} warning(s).", warnings.len())
                                    };
                                }
                                Err(e) => {
                                    state.status_message = format!("Failed importing .rcj: {e}");
                                }
                            }
                        }
                    }
                    if ui.button("Import / Parse Command Line...").clicked() {
                        ui.close();
                        state.open_import_modal();
                    }
                    ui.separator();
                    if ui.button("Export .rcj Job File...").clicked() {
                        ui.close();
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name("robocopy_job.rcj")
                            .add_filter("Robocopy Job", &["rcj"])
                            .save_file()
                        {
                            match std::fs::write(&path, serialize_rcj(&state.job)) {
                                Ok(_) => state.status_message = format!("Exported .rcj: {}", path.display()),
                                Err(e) => state.status_message = format!("Export failed: {e}"),
                            }
                        }
                    }
                    if ui.button("Export .bat Script...").clicked() {
                        ui.close();
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name("robocopy_job.bat")
                            .add_filter("Batch Script", &["bat", "cmd"])
                            .save_file()
                        {
                            match std::fs::write(&path, build_batch_script(&state.job)) {
                                Ok(_) => state.status_message = format!("Exported .bat: {}", path.display()),
                                Err(e) => state.status_message = format!("Export failed: {e}"),
                            }
                        }
                    }
                    if ui.button("Export .ps1 PowerShell Script...").clicked() {
                        ui.close();
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name("robocopy_job.ps1")
                            .add_filter("PowerShell Script", &["ps1"])
                            .save_file()
                        {
                            match std::fs::write(&path, build_powershell_script(&state.job)) {
                                Ok(_) => state.status_message = format!("Exported .ps1: {}", path.display()),
                                Err(e) => state.status_message = format!("Export failed: {e}"),
                            }
                        }
                    }
                    ui.separator();
                    if ui.button("Export Last Run HTML Report...").clicked() {
                        ui.close();
                        state.export_report_for_last(ReportFormat::Html);
                    }
                    if ui.button("Export Last Run Report Bundle (All)...").clicked() {
                        ui.close();
                        state.export_report_for_last(ReportFormat::All);
                    }
                    ui.separator();
                    if ui.button("Exit  (Alt+F4)").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Job", |ui| {
                    if ui.button("New Default Job").clicked() {
                        ui.close();
                        state.set_job(RobocopyJob::default());
                        state.status_message = "Reset to clean default Robocopy job.".to_string();
                    }
                    if ui.button("Save As Profile...").clicked() {
                        ui.close();
                        state.open_save_profile_modal();
                    }
                    if ui.button("Swap Source <-> Destination").clicked() {
                        ui.close();
                        std::mem::swap(&mut state.job.source, &mut state.job.destination);
                        state.refresh_derived();
                    }
                    ui.separator();
                    if ui.button("Copy Command Line  (Ctrl+Shift+C)").clicked() {
                        ui.close();
                        if set_clipboard_text(&state.cached_command) {
                            state.status_message = "Copied command line to clipboard.".to_string();
                        }
                    }
                    if ui.button("Add Current Job to Batch Queue  (F6)").clicked() {
                        ui.close();
                        state.add_current_job_to_queue();
                    }
                });

                ui.menu_button("Execution", |ui| {
                    if ui
                        .add_enabled(!state.is_running, egui::Button::new("Dry-Run Preview (/L)  (F5)"))
                        .clicked()
                    {
                        ui.close();
                        state.trigger_dry_run(ctx);
                    }
                    if ui
                        .add_enabled(!state.is_running, egui::Button::new("Execute Job  (Ctrl+F5)"))
                        .clicked()
                    {
                        ui.close();
                        state.trigger_execute(ctx);
                    }
                    if ui
                        .add_enabled(state.is_running, egui::Button::new("Stop Active Job  (Esc)"))
                        .clicked()
                    {
                        ui.close();
                        state.stop_execution();
                    }
                });

                ui.menu_button("Tools", |ui| {
                    if ui.button("Windows Task Scheduler (schtasks)...").clicked() {
                        ui.close();
                        state.open_task_scheduler_modal();
                    }
                    if ui.button("Exit Code Bitmask Decoder (0-16)...").clicked() {
                        ui.close();
                        state.active_modal = ActiveModal::ExitCodeReference;
                    }
                });

                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut state.show_console_tree, "Console Tree (Left Pane)");
                    ui.checkbox(&mut state.show_command_bar, "Live Command Preview Bar");
                    ui.separator();
                    for tab in WorkspaceTab::all() {
                        if ui.selectable_label(state.active_tab == *tab, tab.title()).clicked() {
                            ui.close();
                            state.active_tab = *tab;
                        }
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("Exit Code Reference (0-16)...").clicked() {
                        ui.close();
                        state.active_modal = ActiveModal::ExitCodeReference;
                    }
                    if ui.button("About Robocopy2Rust...").clicked() {
                        ui.close();
                        state.active_modal = ActiveModal::About;
                    }
                });
            });
        });

    // 2. Classic Win32 Target & Action Toolbar
    egui::TopBottomPanel::top("target_toolbar")
        .frame(
            egui::Frame::NONE
                .fill(Color32::from_rgb(236, 233, 216))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(172, 168, 153)))
                .inner_margin(egui::Margin::symmetric(6, 4)),
        )
        .show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Source:").size(11.5).strong().color(Color32::BLACK));
                if ui
                    .add_sized(
                        [215.0, 20.0],
                        egui::TextEdit::singleline(&mut state.job.source).hint_text(r"C:\Source\Folder"),
                    )
                    .changed()
                {
                    state.refresh_derived();
                }
                if ui.button("...").on_hover_text("Browse Source Folder").clicked()
                    && let Some(p) = pick_folder(&state.job.source, "Select Robocopy Source Folder")
                {
                    state.job.source = p.display().to_string();
                    state.refresh_derived();
                }

                if ui.button("<->").on_hover_text("Swap Source and Destination").clicked() {
                    std::mem::swap(&mut state.job.source, &mut state.job.destination);
                    state.refresh_derived();
                }

                ui.label(RichText::new("Dest:").size(11.5).strong().color(Color32::BLACK));
                if ui
                    .add_sized(
                        [215.0, 20.0],
                        egui::TextEdit::singleline(&mut state.job.destination).hint_text(r"D:\Backup\Target"),
                    )
                    .changed()
                {
                    state.refresh_derived();
                }
                if ui.button("...").on_hover_text("Browse Destination Folder").clicked()
                    && let Some(p) = pick_folder(&state.job.destination, "Select Robocopy Destination Folder")
                {
                    state.job.destination = p.display().to_string();
                    state.refresh_derived();
                }

                ui.label(RichText::new("Files:").size(11.5).strong().color(Color32::BLACK));
                if ui
                    .add_sized(
                        [115.0, 20.0],
                        egui::TextEdit::singleline(&mut state.file_specs_text).hint_text("*.*"),
                    )
                    .on_hover_text("Space/comma-separated file patterns (default *.*)")
                    .changed()
                {
                    state.sync_text_lists_to_job();
                }

                ui.separator();

                if ui
                    .add_enabled(!state.is_running, egui::Button::new("Dry-Run (/L)"))
                    .on_hover_text("Preview planned copies/deletions without modifying files (F5)")
                    .clicked()
                {
                    state.trigger_dry_run(ctx);
                }

                let run_btn_text = if state.job.has_destructive_flags() {
                    RichText::new("Run Job (!)")
                        .strong()
                        .color(Color32::from_rgb(153, 27, 27))
                } else {
                    RichText::new("Run Job").strong().color(Color32::from_rgb(0, 84, 153))
                };
                if ui
                    .add_enabled(
                        !state.is_running && !state.has_blocking_errors(),
                        egui::Button::new(run_btn_text),
                    )
                    .on_hover_text("Execute Robocopy job (Ctrl+F5)")
                    .clicked()
                {
                    state.trigger_execute(ctx);
                }

                if ui
                    .add_enabled(!state.is_running, egui::Button::new("Queue +"))
                    .on_hover_text("Append current job to Batch Queue (F6)")
                    .clicked()
                {
                    state.add_current_job_to_queue();
                }

                if state.is_running
                    && ui
                        .button(RichText::new("Stop").strong().color(Color32::from_rgb(180, 0, 0)))
                        .on_hover_text("Terminate running Robocopy process (Esc)")
                        .clicked()
                {
                    state.stop_execution();
                }
            });
        });
}
