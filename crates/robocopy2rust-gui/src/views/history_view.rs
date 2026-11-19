use crate::state::AppState;
use crate::views::{win32_group_box, win32_subheader};
use eframe::egui::{self, Color32, RichText, Stroke};
use egui_extras::{Column, TableBuilder};
use robocopy2rust_core::format::{format_bytes, format_duration};
use robocopy2rust_core::parse_command_line;
use robocopy2rust_report::ReportFormat;

pub fn render_history_view(ui: &mut egui::Ui, state: &mut AppState) {
    ui.vertical(|ui| {
        win32_subheader(ui, "Persistent Run History & Offline Audit Reports", |ui| {
            if state.last_result.is_some() {
                if ui.small_button("Export Last Run HTML...").clicked() {
                    state.export_report_for_last(ReportFormat::Html);
                }
                if ui.small_button("Export Last Run JSON...").clicked() {
                    state.export_report_for_last(ReportFormat::Json);
                }
                if ui.small_button("Export Last Run MD...").clicked() {
                    state.export_report_for_last(ReportFormat::Markdown);
                }
            }
            if !state.run_history.is_empty() && ui.small_button("Clear History").clicked() {
                let _ = state.store.clear_history();
                state.run_history.clear();
                state.selected_history_idx = None;
            }
        });

        ui.add_space(4.0);

        if let Some(idx) = state.selected_history_idx
            && let Some(entry) = state.run_history.get(idx).cloned()
        {
            win32_group_box(ui, "Selected Run Record", |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&entry.job_name).strong());
                    ui.separator();
                    ui.label(format!("Timestamp: {}", entry.timestamp));
                    ui.separator();
                    ui.label(format!("Exit: {}", entry.exit_label));
                    ui.separator();
                    if ui.small_button("Reload Command into Designer").clicked() {
                        let parsed = parse_command_line(&entry.command_line);
                        state.set_job(parsed.job);
                        state.status_message = format!("Loaded command from run '{}'.", entry.job_name);
                    }
                });
                ui.label(RichText::new(&entry.command_line).monospace().size(11.0));
                ui.label(
                    RichText::new(&entry.summary_text)
                        .size(11.0)
                        .color(Color32::from_rgb(70, 70, 70)),
                );
            });
            ui.add_space(4.0);
        }

        egui::Frame::NONE
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(127, 157, 185)))
            .inner_margin(egui::Margin::same(4))
            .show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                if state.run_history.is_empty() {
                    ui.label("No historical runs recorded yet. Execute or dry-run a job to populate audit history.");
                    return;
                }

                TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .column(Column::initial(155.0))
                    .column(Column::initial(140.0))
                    .column(Column::initial(75.0))
                    .column(Column::initial(110.0))
                    .column(Column::initial(85.0))
                    .column(Column::initial(80.0))
                    .column(Column::remainder())
                    .header(20.0, |mut h| {
                        h.col(|ui| {
                            ui.strong("Timestamp");
                        });
                        h.col(|ui| {
                            ui.strong("Job");
                        });
                        h.col(|ui| {
                            ui.strong("Mode");
                        });
                        h.col(|ui| {
                            ui.strong("Exit Code");
                        });
                        h.col(|ui| {
                            ui.strong("Files / Fails");
                        });
                        h.col(|ui| {
                            ui.strong("Bytes");
                        });
                        h.col(|ui| {
                            ui.strong("Paths");
                        });
                    })
                    .body(|body| {
                        body.rows(20.0, state.run_history.len(), |mut row| {
                            let idx = row.index();
                            let e = &state.run_history[idx];
                            let selected = state.selected_history_idx == Some(idx);

                            row.col(|ui| {
                                if ui
                                    .selectable_label(selected, RichText::new(&e.timestamp).size(11.0))
                                    .clicked()
                                {
                                    state.selected_history_idx = Some(idx);
                                }
                            });
                            row.col(|ui| {
                                ui.label(RichText::new(&e.job_name).size(11.0));
                            });
                            row.col(|ui| {
                                ui.label(if e.dry_run { "Dry-Run" } else { "Live" });
                            });
                            row.col(|ui| {
                                let color = if e.is_success {
                                    Color32::from_rgb(21, 128, 61)
                                } else {
                                    Color32::from_rgb(185, 28, 28)
                                };
                                ui.label(RichText::new(&e.exit_label).strong().size(11.0).color(color));
                            });
                            row.col(|ui| {
                                ui.label(format!("{} / {}", e.files_copied, e.files_failed));
                            });
                            row.col(|ui| {
                                ui.label(format!(
                                    "{} ({})",
                                    format_bytes(e.bytes_copied),
                                    format_duration(e.duration_secs)
                                ));
                            });
                            row.col(|ui| {
                                ui.label(
                                    RichText::new(format!("{} -> {}", e.source, e.destination))
                                        .monospace()
                                        .size(10.5),
                                );
                            });
                        });
                    });
            });
    });
}
