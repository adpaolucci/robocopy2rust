use crate::state::{ActionFilter, AppState, ConsoleSubTab, set_clipboard_text};
use crate::views::{win32_group_box, win32_subheader};
use eframe::egui::{self, Color32, RichText, ScrollArea, Stroke};
use egui_extras::{Column, TableBuilder};
use robocopy2rust_core::format::{format_bytes, format_duration};
use robocopy2rust_report::ReportFormat;
use robocopy2rust_runner::FileActionKind;

pub fn render_live_console(ui: &mut egui::Ui, state: &mut AppState, ctx: &egui::Context) {
    ui.vertical(|ui| {
        let last_dur = state.last_result.as_ref().map(|r| r.duration_secs);
        win32_subheader(ui, "Live Execution Monitor, File Action Diff & Console Stream", |ui| {
            if let Some(duration_secs) = last_dur {
                if ui.small_button("Export HTML...").clicked() {
                    state.export_report_for_last(ReportFormat::Html);
                }
                if ui.small_button("Export All (HTML+JSON+MD)...").clicked() {
                    state.export_report_for_last(ReportFormat::All);
                }
                ui.label(
                    RichText::new(format!("Duration: {}", format_duration(duration_secs)))
                        .size(11.0)
                        .color(Color32::from_rgb(70, 70, 70)),
                );
            }
        });

        ui.add_space(4.0);

        // 1. Progress & Current File Bar
        win32_group_box(ui, "Live Transfer Progress & Summary Counters", |ui| {
            ui.horizontal(|ui| {
                let frac = (state.live_progress_pct / 100.0).clamp(0.0, 1.0);
                let pb = egui::ProgressBar::new(frac).desired_width(260.0).show_percentage();
                ui.add(pb);

                if state.is_running {
                    if ui
                        .button(RichText::new("Stop Job").strong().color(Color32::from_rgb(185, 28, 28)))
                        .clicked()
                    {
                        state.stop_execution();
                    }
                } else {
                    if ui.button("Dry-Run (/L)").clicked() {
                        state.trigger_dry_run(ctx);
                    }
                    if ui.button("Re-Run Job").clicked() {
                        state.trigger_execute(ctx);
                    }
                }

                let active_file = state
                    .live_current_file
                    .as_deref()
                    .unwrap_or("Waiting for file activity...");
                ui.label(RichText::new(active_file).monospace().size(11.0));
            });

            ui.add_space(4.0);

            // Summary Counters Matrix
            let sum = &state.live_summary;
            ui.horizontal_wrapped(|ui| {
                stat_pill(
                    ui,
                    "Dirs Copied",
                    &format!("{}/{}", sum.dirs.copied, sum.dirs.total),
                    Color32::BLACK,
                );
                stat_pill(
                    ui,
                    "Files Copied",
                    &format!("{}/{}", sum.files.copied, sum.files.total),
                    Color32::from_rgb(21, 128, 61),
                );
                stat_pill(
                    ui,
                    "Skipped",
                    &sum.files.skipped.to_string(),
                    Color32::from_rgb(80, 80, 80),
                );
                stat_pill(
                    ui,
                    "Extras",
                    &sum.files.extras.to_string(),
                    Color32::from_rgb(29, 78, 216),
                );
                stat_pill(
                    ui,
                    "Failed",
                    &sum.files.failed.to_string(),
                    if sum.files.failed > 0 || !state.live_errors.is_empty() {
                        Color32::from_rgb(185, 28, 28)
                    } else {
                        Color32::BLACK
                    },
                );
                stat_pill(ui, "Bytes Copied", &format_bytes(sum.bytes.copied), Color32::BLACK);
                if let Some(mbm) = sum.speed_mb_per_min {
                    stat_pill(ui, "Speed", &format!("{mbm:.2} MB/min"), Color32::BLACK);
                }
            });

            // Decoded Exit Code Banner once complete
            if let Some(res) = &state.last_result {
                ui.add_space(4.0);
                let (bg, border, fg) = if res.exit_info.is_success {
                    (
                        Color32::from_rgb(240, 253, 244),
                        Color32::from_rgb(22, 163, 74),
                        Color32::from_rgb(21, 128, 61),
                    )
                } else {
                    (
                        Color32::from_rgb(254, 242, 242),
                        Color32::from_rgb(220, 38, 38),
                        Color32::from_rgb(153, 27, 27),
                    )
                };
                egui::Frame::NONE
                    .fill(bg)
                    .stroke(Stroke::new(1.0_f32, border))
                    .inner_margin(egui::Margin::same(6))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(format!("Exit Code {}: {}", res.exit_code, res.exit_info.short_label))
                                    .strong()
                                    .color(fg),
                            );
                            ui.label(RichText::new(&res.exit_info.summary).size(11.0).color(Color32::BLACK));
                        });
                    });
            }
        });

        ui.add_space(4.0);

        // 2. Sub-tab Switcher & Filters
        ui.horizontal(|ui| {
            if ui
                .selectable_label(
                    state.console_subtab == ConsoleSubTab::FileActions,
                    format!("File Actions / Dry-Run Diff ({})", state.live_file_actions.len()),
                )
                .clicked()
            {
                state.console_subtab = ConsoleSubTab::FileActions;
            }
            if ui
                .selectable_label(
                    state.console_subtab == ConsoleSubTab::RawLog,
                    format!("Raw Console Stream ({} lines)", state.live_log_lines.len()),
                )
                .clicked()
            {
                state.console_subtab = ConsoleSubTab::RawLog;
            }

            ui.separator();

            if state.console_subtab == ConsoleSubTab::FileActions {
                for (f, lbl) in [
                    (ActionFilter::All, "All"),
                    (ActionFilter::CopiedNew, "Copied/New"),
                    (ActionFilter::Extra, "Extras"),
                    (ActionFilter::Skipped, "Skipped"),
                    (ActionFilter::Errors, "Errors"),
                ] {
                    if ui.selectable_label(state.action_filter == f, lbl).clicked() {
                        state.action_filter = f;
                    }
                }
                ui.add_sized(
                    [180.0, 20.0],
                    egui::TextEdit::singleline(&mut state.action_search).hint_text("Filter path..."),
                );
            } else {
                ui.checkbox(&mut state.auto_scroll_log, "Auto-scroll");
                ui.add_sized(
                    [180.0, 20.0],
                    egui::TextEdit::singleline(&mut state.log_search).hint_text("Filter log lines..."),
                );
                if ui.small_button("Copy Log").clicked() {
                    let all_text: String = state.live_log_lines.iter().cloned().collect::<Vec<_>>().join("\n");
                    if set_clipboard_text(&all_text) {
                        state.status_message = "Copied console log to clipboard.".to_string();
                    }
                }
            }
        });

        ui.add_space(3.0);

        // 3. Data Area (File Actions Table or Raw Log Terminal)
        egui::Frame::NONE
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(127, 157, 185)))
            .inner_margin(egui::Margin::same(4))
            .show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                match state.console_subtab {
                    ConsoleSubTab::FileActions => render_actions_table(ui, state),
                    ConsoleSubTab::RawLog => render_raw_log(ui, state),
                }
            });
    });
}

fn stat_pill(ui: &mut egui::Ui, label: &str, value: &str, val_color: Color32) {
    egui::Frame::NONE
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(185, 185, 185)))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{label}:"))
                        .size(11.0)
                        .color(Color32::from_rgb(90, 90, 90)),
                );
                ui.label(RichText::new(value).size(11.5).strong().color(val_color));
            });
        });
}

fn render_actions_table(ui: &mut egui::Ui, state: &AppState) {
    if state.action_filter == ActionFilter::Errors {
        if state.live_errors.is_empty() {
            ui.label("No Robocopy errors captured.");
        } else {
            ScrollArea::vertical().show(ui, |ui| {
                for err in &state.live_errors {
                    ui.label(
                        RichText::new(err)
                            .monospace()
                            .size(11.5)
                            .color(Color32::from_rgb(185, 28, 28)),
                    );
                }
            });
        }
        return;
    }

    let q = state.action_search.trim().to_ascii_lowercase();
    let filtered: Vec<usize> = state
        .live_file_actions
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            let cat_ok = match state.action_filter {
                ActionFilter::All => true,
                ActionFilter::CopiedNew => r.kind.is_copied_or_new(),
                ActionFilter::Extra => r.kind.is_extra(),
                ActionFilter::Skipped => r.kind.is_skipped(),
                ActionFilter::Errors => matches!(r.kind, FileActionKind::Failed | FileActionKind::Mismatch),
            };
            cat_ok
                && (q.is_empty()
                    || r.path.to_ascii_lowercase().contains(&q)
                    || r.kind.label().to_ascii_lowercase().contains(&q))
        })
        .map(|(i, _)| i)
        .collect();

    if filtered.is_empty() {
        ui.label("No file actions recorded matching the active filter.");
        return;
    }

    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .column(Column::initial(110.0).at_least(80.0))
        .column(Column::initial(95.0).at_least(70.0))
        .column(Column::initial(140.0).at_least(90.0))
        .column(Column::remainder())
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.strong("Class");
            });
            header.col(|ui| {
                ui.strong("Size");
            });
            header.col(|ui| {
                ui.strong("Timestamp");
            });
            header.col(|ui| {
                ui.strong("Path");
            });
        })
        .body(|body| {
            body.rows(19.0, filtered.len(), |mut row| {
                let rec = &state.live_file_actions[filtered[row.index()]];
                let kind_color = if rec.kind.is_copied_or_new() {
                    Color32::from_rgb(21, 128, 61)
                } else if rec.kind.is_extra() {
                    Color32::from_rgb(29, 78, 216)
                } else if matches!(rec.kind, FileActionKind::Failed | FileActionKind::Mismatch) {
                    Color32::from_rgb(185, 28, 28)
                } else {
                    Color32::from_rgb(90, 90, 90)
                };

                row.col(|ui| {
                    ui.label(RichText::new(rec.kind.label()).size(11.0).strong().color(kind_color));
                });
                row.col(|ui| {
                    let sz = rec.size_bytes.map(format_bytes).unwrap_or_else(|| "-".to_string());
                    ui.label(RichText::new(sz).size(11.0));
                });
                row.col(|ui| {
                    let ts = rec.timestamp.as_deref().unwrap_or("-");
                    ui.label(RichText::new(ts).size(11.0));
                });
                row.col(|ui| {
                    ui.label(RichText::new(&rec.path).monospace().size(11.0));
                });
            });
        });
}

fn render_raw_log(ui: &mut egui::Ui, state: &AppState) {
    let q = state.log_search.trim().to_ascii_lowercase();
    let lines: Vec<&String> = state
        .live_log_lines
        .iter()
        .filter(|l| q.is_empty() || l.to_ascii_lowercase().contains(&q))
        .collect();

    ScrollArea::both()
        .stick_to_bottom(state.auto_scroll_log)
        .show_rows(ui, 16.0, lines.len(), |ui, range| {
            for idx in range {
                let line = lines[idx];
                let color = if line.contains("ERROR") || line.starts_with("[ERROR]") {
                    Color32::from_rgb(185, 28, 28)
                } else {
                    Color32::BLACK
                };
                ui.label(RichText::new(line).monospace().size(11.0).color(color));
            }
        });
}
