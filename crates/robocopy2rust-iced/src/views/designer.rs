use crate::app::App;
use crate::message::{ActiveModal, Message, WorkspaceTab};
use crate::views::{page_header, section_card};
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Color, Element, Length};
use robocopy2rust_core::ValidationSeverity;

pub fn view_designer(app: &App) -> Element<'_, Message> {
    let header = page_header(
        "Job Designer & Diagnostics Overview",
        row![
            button(text("Save Profile...").size(12))
                .on_press(Message::OpenModal(ActiveModal::SaveProfile))
                .style(button::secondary),
            button(text("Parse Command...").size(12))
                .on_press(Message::OpenModal(ActiveModal::ImportCommand))
                .style(button::secondary),
            button(text("Reset Default").size(12))
                .on_press(Message::ResetDefaultJob)
                .style(button::secondary),
        ]
        .spacing(6),
    );

    let mode_text = if app.job.logging.list_only {
        "SAFE DRY-RUN (/L)"
    } else if app.job.has_destructive_flags() {
        "DESTRUCTIVE MODE"
    } else {
        "STANDARD INCREMENTAL COPY"
    };
    let mode_color = if app.job.logging.list_only {
        Color::from_rgb(0.08, 0.50, 0.24)
    } else if app.job.has_destructive_flags() {
        Color::from_rgb(0.75, 0.14, 0.14)
    } else {
        Color::from_rgb(0.11, 0.31, 0.85)
    };

    let threads_txt = app
        .job
        .copy
        .multithreaded
        .map(|t| format!("{t} threads (/MT:{t})"))
        .unwrap_or_else(|| "Single-threaded".to_string());

    let mut overview_col = column![
        row![
            text("Job Name:").size(13),
            text_input("Job Name", &app.job.name)
                .on_input(Message::JobNameChanged)
                .size(13)
                .width(Length::Fixed(240.0)),
            text("Notes:").size(13),
            text_input("Optional description", &app.job.description)
                .on_input(Message::JobDescChanged)
                .size(13)
                .width(Length::Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        row![
            text("Mode:").size(13),
            text(mode_text).size(13).color(mode_color),
            text("|").size(13),
            text(format!("Threads: {threads_txt}")).size(13),
            text("|").size(13),
            text(format!(
                "Retries: /R:{} /W:{}s",
                app.job
                    .retry
                    .retries
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "default".to_string()),
                app.job
                    .retry
                    .wait_seconds
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "default".to_string()),
            ))
            .size(13),
        ]
        .spacing(8),
    ]
    .spacing(8);

    if app.job.has_destructive_flags() {
        let flags = app.job.destructive_switch_names().join(", ");
        overview_col = overview_col.push(
            container(
                row![
                    text(format!(
                        "WARNING: {flags} active — destination/source files may be deleted. Run Dry-Run (/L) first."
                    ))
                    .size(13)
                    .color(Color::from_rgb(0.75, 0.14, 0.14))
                    .width(Length::Fill),
                    button(text("Run Safe Dry-Run (/L)").size(12))
                        .on_press(Message::TriggerDryRun)
                        .style(button::success),
                ]
                .align_y(Alignment::Center)
                .spacing(8),
            )
            .padding(8)
            .style(container::bordered_box),
        );
    }

    let mut diag_col = column![].spacing(5);
    if app.findings.is_empty() {
        diag_col = diag_col.push(
            text("[OK] No configuration conflicts or path hazards detected.")
                .size(13)
                .color(Color::from_rgb(0.08, 0.50, 0.24)),
        );
    } else {
        for f in &app.findings {
            let (tag, col) = match f.severity {
                ValidationSeverity::Error => ("[ERROR]", Color::from_rgb(0.75, 0.14, 0.14)),
                ValidationSeverity::Warning => ("[WARN] ", Color::from_rgb(0.72, 0.35, 0.04)),
                ValidationSeverity::Info => ("[INFO] ", Color::from_rgb(0.11, 0.31, 0.85)),
            };
            let msg = if let Some(sug) = &f.suggestion {
                format!("{tag} {}: {} ({sug})", f.code, f.message)
            } else {
                format!("{tag} {}: {}", f.code, f.message)
            };
            diag_col = diag_col.push(text(msg).size(13).color(col));
        }
    }

    let mut preset_rows = column![].spacing(8);
    for pair in app.presets.chunks(2) {
        let mut r = row![].spacing(8);
        for (offset, p) in pair.iter().enumerate() {
            let global_idx = app.presets.iter().position(|x| x.id == p.id).unwrap_or(offset);
            let card = container(
                column![
                    row![
                        text(&p.name).size(14).width(Length::Fill),
                        text(&p.category).size(12),
                        button(text("Apply").size(12))
                            .on_press(Message::ApplyPreset(global_idx))
                            .style(if p.destructive { button::danger } else { button::primary }),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                    text(&p.summary).size(12),
                ]
                .spacing(5),
            )
            .padding(10)
            .width(Length::Fill)
            .style(container::bordered_box);
            r = r.push(card);
        }
        preset_rows = preset_rows.push(r);
    }

    let nav_shortcuts = row![
        button(text("Copy & Transfer").size(12))
            .on_press(Message::SelectTab(WorkspaceTab::CopyTransfer))
            .style(button::secondary),
        button(text("File Selection").size(12))
            .on_press(Message::SelectTab(WorkspaceTab::FileSelection))
            .style(button::secondary),
        button(text("Throttle & Retry").size(12))
            .on_press(Message::SelectTab(WorkspaceTab::PerformanceRetry))
            .style(button::secondary),
        button(text("Logging & Jobs").size(12))
            .on_press(Message::SelectTab(WorkspaceTab::LoggingJobs))
            .style(button::secondary),
        button(text("Live Console").size(12))
            .on_press(Message::SelectTab(WorkspaceTab::LiveConsole))
            .style(button::secondary),
        button(text("Batch Queue").size(12))
            .on_press(Message::SelectTab(WorkspaceTab::BatchQueue))
            .style(button::secondary),
    ]
    .spacing(6);

    let diag_title = format!(
        "Real-Time Validation Diagnostics ({} errors, {} warnings)",
        app.error_count(),
        app.warning_count()
    );

    let content = column![
        section_card("Active Job Profile & Paths", overview_col),
        section_card(diag_title, diag_col),
        section_card("Quick-Start Templates (Preserves Current Source & Dest)", preset_rows),
        section_card("Configure Detailed Switches", nav_shortcuts),
    ]
    .spacing(10);

    column![header, scrollable(content).height(Length::Fill)]
        .spacing(10)
        .height(Length::Fill)
        .into()
}
