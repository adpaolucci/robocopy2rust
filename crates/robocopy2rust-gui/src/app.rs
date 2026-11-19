use crate::state::{AppState, WorkspaceTab};
use crate::views::{
    batch_queue::render_batch_queue,
    command_bar::render_command_bar,
    console_tree::render_console_tree,
    designer::render_designer,
    history_view::render_history_view,
    live_console::render_live_console,
    modals::render_modals,
    options_tabs::{
        render_copy_transfer_tab, render_file_selection_tab, render_logging_jobs_tab, render_perf_retry_tab,
    },
    ribbon::render_ribbon,
    status_bar::render_status_bar,
};
use eframe::egui::{self, Color32, Stroke};

pub struct Robocopy2RustApp {
    pub state: AppState,
}

impl Robocopy2RustApp {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

impl eframe::App for Robocopy2RustApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.state.poll_runner(ctx);
        self.state.handle_shortcuts(ctx);

        let win_face = Color32::from_rgb(236, 233, 216);
        let win_border = Color32::from_rgb(172, 168, 153);

        // 1. Classic Win32 Menu Bar + Target & Action Toolbar
        render_ribbon(ctx, &mut self.state);

        // 2. Classic Win32 Multi-Segment Sunken Status Bar (bottommost)
        render_status_bar(ctx, &self.state);

        // 3. Toggleable Live Generated Command-Line Strip
        if self.state.show_command_bar {
            render_command_bar(ctx, &mut self.state);
        }

        // 4. Left Pane: Classic MMC Console Tree (Views, Presets & Saved Profiles)
        if self.state.show_console_tree {
            egui::SidePanel::left("left_console_tree")
                .resizable(true)
                .default_width(245.0)
                .width_range(190.0..=360.0)
                .frame(
                    egui::Frame::NONE
                        .fill(win_face)
                        .stroke(Stroke::new(1.0_f32, win_border))
                        .inner_margin(egui::Margin::same(3)),
                )
                .show(ctx, |ui| {
                    render_console_tree(ui, &mut self.state);
                });
        }

        // 5. Center Workspace View
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(win_face).inner_margin(egui::Margin::same(3)))
            .show(ctx, |ui| match self.state.active_tab {
                WorkspaceTab::Designer => render_designer(ui, &mut self.state, ctx),
                WorkspaceTab::CopyTransfer => render_copy_transfer_tab(ui, &mut self.state),
                WorkspaceTab::FileSelection => render_file_selection_tab(ui, &mut self.state),
                WorkspaceTab::PerformanceRetry => render_perf_retry_tab(ui, &mut self.state),
                WorkspaceTab::LoggingJobs => render_logging_jobs_tab(ui, &mut self.state),
                WorkspaceTab::LiveConsole => render_live_console(ui, &mut self.state, ctx),
                WorkspaceTab::BatchQueue => render_batch_queue(ui, &mut self.state, ctx),
                WorkspaceTab::HistoryReports => render_history_view(ui, &mut self.state),
            });

        // 6. Active Win32 Modal Dialogs
        render_modals(ctx, &mut self.state);
    }
}
