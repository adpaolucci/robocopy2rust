use crate::state::{AppState, WorkspaceTab};
use eframe::egui::{self, Color32, RichText, ScrollArea, Stroke};

pub fn render_console_tree(ui: &mut egui::Ui, state: &mut AppState) {
    ui.vertical(|ui| {
        egui::Frame::NONE
            .fill(Color32::from_rgb(235, 235, 235))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(180, 180, 180)))
            .inner_margin(egui::Margin::symmetric(5, 3))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Console Tree").size(11.0).strong().color(Color32::BLACK));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Save...").clicked() {
                            state.open_save_profile_modal();
                        }
                    });
                });
            });

        ui.add_space(2.0);

        egui::Frame::NONE
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(127, 157, 185)))
            .inner_margin(egui::Margin::same(4))
            .show(ui, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.set_width(ui.available_width());

                    // 1. Workspace Views
                    ui.label(
                        RichText::new("[-] Robocopy Console")
                            .size(11.5)
                            .strong()
                            .color(Color32::BLACK),
                    );
                    ui.add_space(2.0);
                    for tab in WorkspaceTab::all() {
                        let active = state.active_tab == *tab;
                        let bg = if active {
                            Color32::from_rgb(49, 106, 197)
                        } else {
                            Color32::TRANSPARENT
                        };
                        let fg = if active { Color32::WHITE } else { Color32::BLACK };
                        let suffix = match tab {
                            WorkspaceTab::BatchQueue if !state.batch_items.is_empty() => {
                                format!(" ({})", state.batch_items.len())
                            }
                            WorkspaceTab::LiveConsole if !state.live_file_actions.is_empty() => {
                                format!(" ({})", state.live_file_actions.len())
                            }
                            WorkspaceTab::HistoryReports if !state.run_history.is_empty() => {
                                format!(" ({})", state.run_history.len())
                            }
                            _ => String::new(),
                        };

                        egui::Frame::NONE
                            .fill(bg)
                            .inner_margin(egui::Margin::symmetric(5, 2))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add_space(6.0);
                                    if ui
                                        .selectable_label(
                                            active,
                                            RichText::new(format!("{}{}", tab.title(), suffix)).size(11.5).color(fg),
                                        )
                                        .clicked()
                                    {
                                        state.active_tab = *tab;
                                    }
                                });
                            });
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // 2. Built-in Presets
                    ui.label(
                        RichText::new("[-] Built-in Presets")
                            .size(11.5)
                            .strong()
                            .color(Color32::BLACK),
                    );
                    ui.add_space(2.0);
                    let mut chosen_preset = None;
                    for preset in &state.presets {
                        let label = if preset.destructive {
                            format!("[!] {}", preset.name)
                        } else {
                            preset.name.clone()
                        };
                        let color = if preset.destructive {
                            Color32::from_rgb(153, 27, 27)
                        } else {
                            Color32::BLACK
                        };
                        if ui
                            .selectable_label(false, RichText::new(label).size(11.0).color(color))
                            .on_hover_text(&preset.summary)
                            .clicked()
                        {
                            chosen_preset = Some(preset.clone());
                        }
                    }
                    if let Some(p) = chosen_preset {
                        state.set_job_preserving_paths(p.job);
                        state.status_message = format!("Applied preset '{}' (paths preserved).", p.name);
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // 3. Saved Profiles
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("[-] Saved Profiles ({})", state.profiles.len()))
                                .size(11.5)
                                .strong()
                                .color(Color32::BLACK),
                        );
                    });
                    ui.add_space(2.0);

                    if state.profiles.is_empty() {
                        ui.label(
                            RichText::new("  No saved profiles yet.")
                                .size(10.5)
                                .color(Color32::from_rgb(110, 110, 110)),
                        );
                    } else {
                        let mut load_profile = None;
                        let mut delete_id = None;
                        for prof in &state.profiles {
                            ui.horizontal(|ui| {
                                if ui
                                    .selectable_label(false, RichText::new(&prof.name).size(11.0))
                                    .on_hover_text(format!("{} -> {}", prof.job.source, prof.job.destination))
                                    .clicked()
                                {
                                    load_profile = Some(prof.clone());
                                }
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.small_button("x").on_hover_text("Delete profile").clicked() {
                                        delete_id = Some(prof.id.clone());
                                    }
                                });
                            });
                        }
                        if let Some(prof) = load_profile {
                            state.set_job(prof.job);
                            state.status_message = format!("Loaded saved profile '{}'.", prof.name);
                        }
                        if let Some(id) = delete_id
                            && let Ok((_, updated)) = state.store.delete_profile(&id)
                        {
                            state.profiles = updated;
                            state.status_message = "Deleted saved profile.".to_string();
                        }
                    }
                });
            });
    });
}
