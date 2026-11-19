use crate::state::{AppState, BatchItemStatus};
use crate::views::win32_subheader;
use eframe::egui::{self, Color32, RichText, Stroke};
use egui_extras::{Column, TableBuilder};

pub fn render_batch_queue(ui: &mut egui::Ui, state: &mut AppState, ctx: &egui::Context) {
    ui.vertical(|ui| {
        win32_subheader(ui, "Multi-Job Batch Execution Queue", |ui| {
            ui.checkbox(&mut state.batch_stop_on_error, "Stop on Error (exit >= 8)");
            if ui
                .add_enabled(!state.is_running, egui::Button::new("Add Current Job"))
                .clicked()
            {
                state.add_current_job_to_queue();
            }
            if ui
                .add_enabled(!state.is_running && !state.batch_items.is_empty(), egui::Button::new("Dry-Run Queue (/L)"))
                .clicked()
            {
                state.start_batch_queue(true, ctx);
            }
            if ui
                .add_enabled(!state.is_running && !state.batch_items.is_empty(), egui::Button::new("Run Queue"))
                .clicked()
            {
                state.start_batch_queue(false, ctx);
            }
            if state.batch_running
                && ui
                    .button(RichText::new("Stop").color(Color32::from_rgb(185, 28, 28)))
                    .clicked()
            {
                state.stop_execution();
            }
            if ui
                .add_enabled(!state.is_running && !state.batch_items.is_empty(), egui::Button::new("Clear"))
                .clicked()
            {
                state.batch_items.clear();
            }
        });

        ui.add_space(4.0);

        egui::Frame::NONE
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(127, 157, 185)))
            .inner_margin(egui::Margin::same(4))
            .show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                if state.batch_items.is_empty() {
                    ui.label("Batch queue is empty. Configure a job and click 'Queue +' (or F6) to queue multiple Robocopy jobs.");
                    return;
                }

                let mut move_up: Option<usize> = None;
                let mut move_down: Option<usize> = None;
                let mut load_idx: Option<usize> = None;
                let mut dup_idx: Option<usize> = None;
                let mut del_idx: Option<usize> = None;

                let len = state.batch_items.len();
                TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .column(Column::initial(36.0))
                    .column(Column::initial(160.0).at_least(110.0))
                    .column(Column::initial(180.0).at_least(120.0))
                    .column(Column::initial(180.0).at_least(120.0))
                    .column(Column::initial(130.0).at_least(90.0))
                    .column(Column::remainder().at_least(160.0))
                    .header(22.0, |mut h| {
                        h.col(|ui| { ui.strong("#"); });
                        h.col(|ui| { ui.strong("Job Name"); });
                        h.col(|ui| { ui.strong("Source"); });
                        h.col(|ui| { ui.strong("Destination"); });
                        h.col(|ui| { ui.strong("Status"); });
                        h.col(|ui| { ui.strong("Actions"); });
                    })
                    .body(|body| {
                        body.rows(22.0, len, |mut row| {
                            let idx = row.index();
                            let item = &state.batch_items[idx];
                            row.col(|ui| {
                                ui.label(format!("{}", idx + 1));
                            });
                            row.col(|ui| {
                                ui.label(&item.job.name);
                            });
                            row.col(|ui| {
                                ui.label(RichText::new(&item.job.source).monospace().size(11.0));
                            });
                            row.col(|ui| {
                                ui.label(RichText::new(&item.job.destination).monospace().size(11.0));
                            });
                            row.col(|ui| {
                                let (txt, color) = match item.status {
                                    BatchItemStatus::Pending => ("Pending".to_string(), Color32::from_rgb(90, 90, 90)),
                                    BatchItemStatus::Running => ("Running...".to_string(), Color32::from_rgb(29, 78, 216)),
                                    BatchItemStatus::Succeeded(c) => (format!("OK ({c})"), Color32::from_rgb(21, 128, 61)),
                                    BatchItemStatus::Failed(c) => (format!("FAIL ({c})"), Color32::from_rgb(185, 28, 28)),
                                };
                                ui.label(RichText::new(txt).strong().color(color))
                                    .on_hover_text(&item.summary_note);
                            });
                            row.col(|ui| {
                                ui.horizontal(|ui| {
                                    if idx > 0 && ui.small_button("Up").clicked() {
                                        move_up = Some(idx);
                                    }
                                    if idx + 1 < len && ui.small_button("Dn").clicked() {
                                        move_down = Some(idx);
                                    }
                                    if ui.small_button("Edit").clicked() {
                                        load_idx = Some(idx);
                                    }
                                    if ui.small_button("Dup").clicked() {
                                        dup_idx = Some(idx);
                                    }
                                    if !state.is_running && ui.small_button("Del").clicked() {
                                        del_idx = Some(idx);
                                    }
                                });
                            });
                        });
                    });

                if let Some(i) = move_up
                    && i > 0
                {
                    state.batch_items.swap(i, i - 1);
                }
                if let Some(i) = move_down
                    && i + 1 < state.batch_items.len()
                {
                    state.batch_items.swap(i, i + 1);
                }
                if let Some(i) = load_idx {
                    let j = state.batch_items[i].job.clone();
                    state.set_job(j);
                    state.status_message = format!("Loaded queued job #{} into Designer.", i + 1);
                }
                if let Some(i) = dup_idx {
                    let mut cloned = state.batch_items[i].clone();
                    cloned.id = state.next_batch_id;
                    state.next_batch_id += 1;
                    cloned.status = BatchItemStatus::Pending;
                    cloned.summary_note = "Duplicated".to_string();
                    state.batch_items.insert(i + 1, cloned);
                }
                if let Some(i) = del_idx {
                    state.batch_items.remove(i);
                }
            });
    });
}
