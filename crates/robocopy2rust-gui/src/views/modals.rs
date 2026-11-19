use crate::state::{ActiveModal, AppState, set_clipboard_text};
use eframe::egui::{self, Color32, RichText, Stroke};
use robocopy2rust_core::{ScheduleCadence, build_schtasks_command, decode_exit_code};

pub fn render_modals(ctx: &egui::Context, state: &mut AppState) {
    match state.active_modal {
        ActiveModal::None => {}
        ActiveModal::ConfirmDestructive => render_confirm_destructive(ctx, state),
        ActiveModal::ImportCommand => render_import_command(ctx, state),
        ActiveModal::SaveProfile => render_save_profile(ctx, state),
        ActiveModal::TaskScheduler => render_task_scheduler(ctx, state),
        ActiveModal::ExitCodeReference => render_exit_code_modal(ctx, state),
        ActiveModal::About => render_about_modal(ctx, state),
    }
}

fn render_confirm_destructive(ctx: &egui::Context, state: &mut AppState) {
    let Some(pending) = state.pending_destructive_job.clone() else {
        state.active_modal = ActiveModal::None;
        return;
    };
    let flags = pending.destructive_switch_names().join(", ");

    egui::Window::new("Confirm Destructive Robocopy Execution")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_width(470.0);
            egui::Frame::NONE
                .fill(Color32::from_rgb(254, 242, 242))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(220, 38, 38)))
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(format!("Destructive option(s) active: {flags}"))
                            .strong()
                            .color(Color32::from_rgb(153, 27, 27)),
                    );
                    ui.label("Files in destination (/MIR /PURGE) or source (/MOV /MOVE) may be permanently deleted.");
                });

            ui.add_space(6.0);
            ui.label(format!("Source:      {}", pending.source));
            ui.label(format!("Destination: {}", pending.destination));
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Run Safe Dry-Run (/L) First").clicked() {
                    state.active_modal = ActiveModal::None;
                    state.pending_destructive_job = None;
                    state.trigger_dry_run(ctx);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(
                            RichText::new("Execute Destructive Job")
                                .strong()
                                .color(Color32::from_rgb(185, 28, 28)),
                        )
                        .clicked()
                    {
                        state.confirm_pending_destructive_run(ctx);
                    }
                    if ui.button("Cancel").clicked() {
                        state.active_modal = ActiveModal::None;
                        state.pending_destructive_job = None;
                    }
                });
            });
        });
}

fn render_import_command(ctx: &egui::Context, state: &mut AppState) {
    egui::Window::new("Import / Parse Robocopy Command Line")
        .collapsible(false)
        .resizable(true)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_width(540.0);
            ui.label("Paste any Microsoft Robocopy command string to populate the GUI switches:");
            ui.add_space(4.0);
            ui.add_sized(
                [520.0, 90.0],
                egui::TextEdit::multiline(&mut state.import_cmd_text).font(egui::TextStyle::Monospace),
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Parse & Load into Designer").clicked() {
                    state.apply_import_command();
                }
                if ui.button("Cancel").clicked() {
                    state.active_modal = ActiveModal::None;
                }
            });
        });
}

fn render_save_profile(ctx: &egui::Context, state: &mut AppState) {
    egui::Window::new("Save Robocopy Job Profile")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_width(400.0);
            ui.horizontal(|ui| {
                ui.label("Profile Name:");
                ui.add_sized([280.0, 20.0], egui::TextEdit::singleline(&mut state.save_profile_name));
            });
            ui.horizontal(|ui| {
                ui.label("Description: ");
                ui.add_sized([280.0, 20.0], egui::TextEdit::singleline(&mut state.save_profile_desc));
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !state.save_profile_name.trim().is_empty(),
                        egui::Button::new("Save Profile"),
                    )
                    .clicked()
                {
                    let name = state.save_profile_name.clone();
                    let desc = state.save_profile_desc.clone();
                    if let Ok(updated) = state.store.upsert_profile(&name, &desc, state.job.clone()) {
                        state.profiles = updated;
                        state.job.name = name.clone();
                        state.job.description = desc;
                        state.status_message = format!("Saved profile '{name}'.");
                    }
                    state.active_modal = ActiveModal::None;
                }
                if ui.button("Cancel").clicked() {
                    state.active_modal = ActiveModal::None;
                }
            });
        });
}

fn render_task_scheduler(ctx: &egui::Context, state: &mut AppState) {
    egui::Window::new("Windows Task Scheduler (schtasks) Generator")
        .collapsible(false)
        .resizable(true)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_width(540.0);
            ui.horizontal(|ui| {
                ui.label("Task Name:");
                ui.add_sized([300.0, 20.0], egui::TextEdit::singleline(&mut state.task_sched_name));
            });
            ui.horizontal(|ui| {
                ui.label("Cadence:");
                for (cad, lbl) in [
                    (ScheduleCadence::Daily, "DAILY"),
                    (ScheduleCadence::Weekly, "WEEKLY"),
                    (ScheduleCadence::Hourly, "HOURLY"),
                    (ScheduleCadence::OnStart, "ONSTART"),
                    (ScheduleCadence::OnLogon, "ONLOGON"),
                ] {
                    if ui.selectable_label(state.task_sched_spec.cadence == cad, lbl).clicked() {
                        state.task_sched_spec.cadence = cad;
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Start Time (HH:MM):");
                ui.add_sized(
                    [70.0, 20.0],
                    egui::TextEdit::singleline(&mut state.task_sched_spec.start_time),
                );
                ui.checkbox(
                    &mut state.task_sched_spec.run_highest,
                    "Run with highest privileges (/RL HIGHEST)",
                );
            });
            if state.task_sched_spec.cadence == ScheduleCadence::Weekly {
                let mut days = state
                    .task_sched_spec
                    .days
                    .clone()
                    .unwrap_or_else(|| "MON,WED,FRI".to_string());
                ui.horizontal(|ui| {
                    ui.label("Days (/D):");
                    if ui
                        .add_sized([180.0, 20.0], egui::TextEdit::singleline(&mut days))
                        .changed()
                    {
                        state.task_sched_spec.days = if days.trim().is_empty() { None } else { Some(days) };
                    }
                });
            }
            ui.horizontal(|ui| {
                ui.label("Target (/TR):");
                ui.add_sized([420.0, 20.0], egui::TextEdit::singleline(&mut state.task_sched_target));
            });

            let generated =
                build_schtasks_command(&state.task_sched_name, &state.task_sched_target, &state.task_sched_spec);
            ui.add_space(4.0);
            ui.label(RichText::new("Generated schtasks Command:").strong());
            let mut ro = generated.clone();
            ui.add_sized(
                [520.0, 64.0],
                egui::TextEdit::multiline(&mut ro).font(egui::TextStyle::Monospace),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Copy schtasks Command").clicked() && set_clipboard_text(&generated) {
                    state.status_message = "Copied schtasks command to clipboard.".to_string();
                }
                if ui.button("Close").clicked() {
                    state.active_modal = ActiveModal::None;
                }
            });
        });
}

fn render_exit_code_modal(ctx: &egui::Context, state: &mut AppState) {
    egui::Window::new("Robocopy Bitmask Exit Code Decoder (0-16)")
        .collapsible(false)
        .resizable(true)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_width(500.0);
            ui.horizontal(|ui| {
                ui.label("Test Exit Code:");
                ui.add(egui::DragValue::new(&mut state.exit_code_calc_input).range(0..=31));
                for quick in [0, 1, 2, 3, 4, 8, 16] {
                    if ui.small_button(quick.to_string()).clicked() {
                        state.exit_code_calc_input = quick;
                    }
                }
            });

            let info = decode_exit_code(state.exit_code_calc_input);
            ui.add_space(4.0);
            let color = if info.is_success {
                Color32::from_rgb(21, 128, 61)
            } else {
                Color32::from_rgb(185, 28, 28)
            };
            ui.label(RichText::new(&info.short_label).strong().color(color));
            ui.label(&info.summary);
            ui.separator();
            for b in &info.bit_descriptions {
                ui.label(format!("• {b}"));
            }
            ui.add_space(6.0);
            ui.label(
                RichText::new(
                    "Rule of thumb: Exit codes 0..=7 indicate success; exit codes >= 8 indicate copy or fatal errors.",
                )
                .size(11.0)
                .color(Color32::from_rgb(80, 80, 80)),
            );
            ui.add_space(6.0);
            if ui.button("Close").clicked() {
                state.active_modal = ActiveModal::None;
            }
        });
}

fn render_about_modal(ctx: &egui::Context, state: &mut AppState) {
    egui::Window::new("About Robocopy2Rust Management Console")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_width(420.0);
            ui.label(RichText::new("Robocopy2Rust Management Console v0.1.0").strong().size(14.0));
            ui.label("Part of the GUAP Ecosystem (guap.ca)");
            ui.add_space(4.0);
            ui.label("100% Pure-Rust Classic Win32/MMC GUI wrapper, job orchestrator, .rcj/.bat/.ps1 generator, offline audit reporter, and MCP stdio server for Microsoft Robocopy.");
            ui.add_space(4.0);
            ui.label(format!("Host OS: {}", state.env.os));
            ui.label(format!(
                "Robocopy Executable: {}",
                state
                    .env
                    .robocopy_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "Not found".to_string())
            ));
            ui.label(format!("Config Store: {}", state.env.config_dir.display()));
            ui.add_space(8.0);
            if ui.button("Close").clicked() {
                state.active_modal = ActiveModal::None;
            }
        });
}
