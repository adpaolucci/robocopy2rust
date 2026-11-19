use crate::state::{ActiveModal, AppState, set_clipboard_text};
use eframe::egui::{self, Color32, RichText, Stroke};

pub fn render_command_bar(ctx: &egui::Context, state: &mut AppState) {
    egui::TopBottomPanel::bottom("live_command_bar")
        .frame(
            egui::Frame::NONE
                .fill(Color32::from_rgb(236, 233, 216))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(172, 168, 153)))
                .inner_margin(egui::Margin::symmetric(6, 4)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("CMD:").size(11.0).strong().color(Color32::BLACK));

                let mut cmd_view = state.cached_command.clone();
                let avail = (ui.available_width() - 255.0).max(200.0);
                ui.add_sized(
                    [avail, 20.0],
                    egui::TextEdit::singleline(&mut cmd_view)
                        .font(egui::TextStyle::Monospace)
                        .interactive(true),
                );

                if ui
                    .small_button("Copy")
                    .on_hover_text("Copy command line (Ctrl+Shift+C)")
                    .clicked()
                    && set_clipboard_text(&state.cached_command)
                {
                    state.status_message = "Copied Robocopy command line to clipboard.".to_string();
                }
                if ui
                    .small_button("Import...")
                    .on_hover_text("Paste a raw Robocopy command to populate GUI")
                    .clicked()
                {
                    state.open_import_modal();
                }
                if ui
                    .small_button("Schedule...")
                    .on_hover_text("Generate Windows Task Scheduler command")
                    .clicked()
                {
                    state.open_task_scheduler_modal();
                }
                if ui
                    .small_button("Exit Codes")
                    .on_hover_text("Open 0-16 Exit Code Bitmask Decoder")
                    .clicked()
                {
                    state.active_modal = ActiveModal::ExitCodeReference;
                }
            });
        });
}
