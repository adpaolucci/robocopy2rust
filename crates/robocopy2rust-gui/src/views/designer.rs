use crate::state::{AppState, WorkspaceTab};
use crate::views::{win32_group_box, win32_subheader};
use eframe::egui::{self, Color32, RichText, ScrollArea, Stroke};
use robocopy2rust_core::ValidationSeverity;

pub fn render_designer(ui: &mut egui::Ui, state: &mut AppState, ctx: &egui::Context) {
    ui.vertical(|ui| {
        win32_subheader(ui, "Job Designer & Diagnostics Overview", |ui| {
            if ui.small_button("Save Profile...").clicked() {
                state.open_save_profile_modal();
            }
            if ui.small_button("Parse Command...").clicked() {
                state.open_import_modal();
            }
        });

        ui.add_space(4.0);

        ScrollArea::vertical().show(ui, |ui| {
            // 1. Job Identity & Quick Summary
            win32_group_box(ui, "Active Job Profile & Paths", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Job Name:");
                    ui.add_sized([220.0, 20.0], egui::TextEdit::singleline(&mut state.job.name));
                    ui.add_space(10.0);
                    ui.label("Notes:");
                    ui.add_sized(
                        [ui.available_width().max(150.0), 20.0],
                        egui::TextEdit::singleline(&mut state.job.description),
                    );
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    let mode_label = if state.job.logging.list_only {
                        RichText::new("SAFE DRY-RUN (/L)").strong().color(Color32::from_rgb(21, 128, 61))
                    } else if state.job.has_destructive_flags() {
                        RichText::new(format!("DESTRUCTIVE ({})", state.job.destructive_switch_names().join(", ")))
                            .strong()
                            .color(Color32::from_rgb(185, 28, 28))
                    } else {
                        RichText::new("STANDARD INCREMENTAL COPY").strong().color(Color32::from_rgb(29, 78, 216))
                    };
                    ui.label("Mode:");
                    ui.label(mode_label);

                    ui.separator();
                    let threads_txt = state
                        .job
                        .copy
                        .multithreaded
                        .map(|t| format!("{t} threads (/MT:{t})"))
                        .unwrap_or_else(|| "Single-threaded".to_string());
                    ui.label(format!("Threads: {threads_txt}"));

                    ui.separator();
                    ui.label(format!(
                        "Retries: /R:{} /W:{}s",
                        state
                            .job
                            .retry
                            .retries
                            .map(|r| r.to_string())
                            .unwrap_or_else(|| "default (1M)".to_string()),
                        state
                            .job
                            .retry
                            .wait_seconds
                            .map(|w| w.to_string())
                            .unwrap_or_else(|| "default (30)".to_string())
                    ));
                });

                if state.job.has_destructive_flags() {
                    ui.add_space(4.0);
                    egui::Frame::NONE
                        .fill(Color32::from_rgb(254, 242, 242))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(220, 38, 38)))
                        .inner_margin(egui::Margin::same(6))
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "WARNING: {} active. Run Dry-Run (/L) before executing to inspect planned deletions.",
                                        state.job.destructive_switch_names().join(", ")
                                    ))
                                    .strong()
                                    .color(Color32::from_rgb(153, 27, 27)),
                                );
                                if ui.button("Run Safe Dry-Run (/L) Now").clicked() {
                                    state.trigger_dry_run(ctx);
                                }
                            });
                        });
                }
            });

            ui.add_space(6.0);

            // 2. Real-time Validation Diagnostics
            let diag_title = format!(
                "Real-Time Validation Diagnostics ({} errors, {} warnings)",
                state.error_count(),
                state.warning_count()
            );
            win32_group_box(ui, &diag_title, |ui| {
                if state.findings.is_empty() {
                    ui.label(
                        RichText::new("[OK] No configuration conflicts or path hazards detected.")
                            .color(Color32::from_rgb(21, 128, 61)),
                    );
                } else {
                    for f in &state.findings {
                        let (tag, color) = match f.severity {
                            ValidationSeverity::Error => ("[ERROR]", Color32::from_rgb(185, 28, 28)),
                            ValidationSeverity::Warning => ("[WARN] ", Color32::from_rgb(180, 83, 9)),
                            ValidationSeverity::Info => ("[INFO] ", Color32::from_rgb(29, 78, 216)),
                        };
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(tag).strong().color(color));
                            ui.label(RichText::new(&f.message).color(Color32::BLACK));
                            if let Some(sug) = &f.suggestion {
                                ui.label(
                                    RichText::new(format!("({sug})"))
                                        .size(11.0)
                                        .color(Color32::from_rgb(90, 90, 90)),
                                );
                            }
                        });
                    }
                }
            });

            ui.add_space(6.0);

            // 3. Built-in Quick-Start Presets Grid
            win32_group_box(ui, "Quick-Start Templates (Preserves Current Source & Destination)", |ui| {
                let presets = state.presets.clone();
                let mut selected_preset = None;

                egui::Grid::new("presets_grid")
                    .num_columns(2)
                    .spacing([8.0, 8.0])
                    .show(ui, |ui| {
                        for (idx, preset) in presets.iter().enumerate() {
                            let border = if preset.destructive {
                                Color32::from_rgb(190, 120, 120)
                            } else {
                                Color32::from_rgb(160, 160, 160)
                            };
                            egui::Frame::NONE
                                .fill(Color32::WHITE)
                                .stroke(Stroke::new(1.0_f32, border))
                                .inner_margin(egui::Margin::same(8))
                                .show(ui, |ui| {
                                    ui.set_min_width(340.0);
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(&preset.name)
                                                .strong()
                                                .color(if preset.destructive {
                                                    Color32::from_rgb(153, 27, 27)
                                                } else {
                                                    Color32::BLACK
                                                }),
                                        );
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.small_button("Apply").clicked() {
                                                selected_preset = Some(preset.clone());
                                            }
                                            ui.label(
                                                RichText::new(&preset.category)
                                                    .size(10.5)
                                                    .color(Color32::from_rgb(100, 100, 100)),
                                            );
                                        });
                                    });
                                    ui.label(
                                        RichText::new(&preset.summary)
                                            .size(11.0)
                                            .color(Color32::from_rgb(70, 70, 70)),
                                    );
                                });
                            if idx % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });

                if let Some(p) = selected_preset {
                    state.set_job_preserving_paths(p.job);
                    state.status_message = format!("Applied preset '{}'.", p.name);
                }
            });

            ui.add_space(6.0);

            // 4. Quick Navigation Footer
            win32_group_box(ui, "Switch Configuration Sections", |ui| {
                ui.horizontal_wrapped(|ui| {
                    for tab in [
                        WorkspaceTab::CopyTransfer,
                        WorkspaceTab::FileSelection,
                        WorkspaceTab::PerformanceRetry,
                        WorkspaceTab::LoggingJobs,
                        WorkspaceTab::LiveConsole,
                        WorkspaceTab::BatchQueue,
                    ] {
                        if ui.button(format!("Open {}", tab.title())).clicked() {
                            state.active_tab = tab;
                        }
                    }
                });
            });
        });
    });
}
