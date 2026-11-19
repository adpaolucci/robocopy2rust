use crate::state::AppState;
use eframe::egui::{self, Color32, RichText, Stroke};

pub fn render_status_bar(ctx: &egui::Context, state: &AppState) {
    let win_face = Color32::from_rgb(236, 233, 216);
    let win_border = Color32::from_rgb(172, 168, 153);

    egui::TopBottomPanel::bottom("status_bar")
        .frame(
            egui::Frame::NONE
                .fill(win_face)
                .stroke(Stroke::new(1.0_f32, win_border))
                .inner_margin(egui::Margin::symmetric(3, 2)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(3.0, 0.0);

                // Segment 1: Active status message
                sunken_status_panel(ui, 360.0, |ui| {
                    ui.label(RichText::new(&state.status_message).size(11.0).color(Color32::BLACK));
                });

                // Segment 2: Active Tab & Diagnostics
                let diag_text = if state.error_count() > 0 {
                    format!(
                        "{} | {} Err / {} Warn",
                        state.active_tab.title(),
                        state.error_count(),
                        state.warning_count()
                    )
                } else if state.warning_count() > 0 {
                    format!("{} | {} Warn", state.active_tab.title(), state.warning_count())
                } else {
                    format!("{} | Valid", state.active_tab.title())
                };
                sunken_status_panel(ui, 190.0, |ui| {
                    ui.label(RichText::new(diag_text).size(11.0).color(Color32::BLACK));
                });

                // Segment 3: Runner state / Exit code
                let run_seg = if state.is_running {
                    format!("RUNNING ({:.1}%)", state.live_progress_pct)
                } else if let Some(res) = &state.last_result {
                    format!("Last: {}", res.exit_info.short_label)
                } else {
                    "Idle".to_string()
                };
                sunken_status_panel(ui, 170.0, |ui| {
                    ui.label(RichText::new(run_seg).size(11.0).color(Color32::BLACK));
                });

                // Right-aligned Robocopy environment segment
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let badge = if state.env.robocopy_available {
                        "100% Pure Rust - Robocopy Ready"
                    } else {
                        "100% Pure Rust - Builder Only (No robocopy.exe)"
                    };
                    sunken_status_panel(ui, 240.0, |ui| {
                        ui.label(RichText::new(badge).size(11.0).color(Color32::from_rgb(70, 70, 70)));
                    });
                });
            });
        });
}

fn sunken_status_panel<R>(ui: &mut egui::Ui, width: f32, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let frame = egui::Frame::NONE
        .fill(Color32::from_rgb(244, 243, 238))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(172, 168, 153)))
        .inner_margin(egui::Margin::symmetric(5, 1));

    ui.allocate_ui_with_layout(
        egui::vec2(width, 18.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| frame.show(ui, add_contents).inner,
    )
    .inner
}
