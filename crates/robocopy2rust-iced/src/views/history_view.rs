use crate::app::App;
use crate::message::Message;
use crate::views::{page_header, section_card};
use iced::widget::{button, column, container, row, rule, scrollable, text};
use iced::{Alignment, Color, Element, Font, Length};
use robocopy2rust_core::format::{format_bytes, format_duration};
use robocopy2rust_report::ReportFormat;

pub fn view_history_view(app: &App) -> Element<'_, Message> {
    let mut controls = row![].spacing(6);
    if app.last_result.is_some() {
        controls = controls
            .push(
                button(text("Export Last HTML...").size(12))
                    .on_press(Message::ExportReport(ReportFormat::Html))
                    .style(button::secondary),
            )
            .push(
                button(text("Export Last JSON...").size(12))
                    .on_press(Message::ExportReport(ReportFormat::Json))
                    .style(button::secondary),
            )
            .push(
                button(text("Export Last MD...").size(12))
                    .on_press(Message::ExportReport(ReportFormat::Markdown))
                    .style(button::secondary),
            );
    }
    if !app.run_history.is_empty() {
        controls = controls.push(
            button(text("Clear History").size(12))
                .on_press(Message::ClearHistory)
                .style(button::danger),
        );
    }

    let header = page_header("Persistent Run History & Offline Audit Reports", controls);

    let mut col = column![header].spacing(10);

    if let Some(idx) = app.selected_history_idx
        && let Some(entry) = app.run_history.get(idx)
    {
        let detail = column![
            row![
                text(&entry.job_name).size(14),
                text(format!("| Timestamp: {}", entry.timestamp)).size(12),
                text(format!("| Exit: {}", entry.exit_label)).size(12),
                button(text("Reload Command into Designer").size(12))
                    .on_press(Message::ReloadHistoryCommand(idx))
                    .style(button::primary),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            text(&entry.command_line).size(12).font(Font::MONOSPACE),
            text(&entry.summary_text).size(12),
        ]
        .spacing(6);
        col = col.push(section_card("Selected Run Record", detail));
    }

    let list_body: Element<'_, Message> = if app.run_history.is_empty() {
        text("No historical runs recorded yet. Execute or dry-run a job to populate audit history.")
            .size(13)
            .into()
    } else {
        let mut rows = column![
            row![
                text("Timestamp").size(12).width(Length::Fixed(165.0)),
                text("Job").size(12).width(Length::Fixed(140.0)),
                text("Mode").size(12).width(Length::Fixed(65.0)),
                text("Exit").size(12).width(Length::Fixed(125.0)),
                text("Files / Fails").size(12).width(Length::Fixed(85.0)),
                text("Bytes (Time)").size(12).width(Length::Fixed(125.0)),
                text("Paths").size(12).width(Length::Fill),
            ]
            .spacing(8),
            rule::horizontal(1),
        ]
        .spacing(5);

        for (idx, e) in app.run_history.iter().enumerate() {
            let selected = app.selected_history_idx == Some(idx);
            let exit_col = if e.is_success {
                Color::from_rgb(0.08, 0.50, 0.24)
            } else {
                Color::from_rgb(0.75, 0.14, 0.14)
            };

            rows = rows.push(
                row![
                    button(text(&e.timestamp).size(11))
                        .on_press(Message::SelectHistory(idx))
                        .style(if selected { button::primary } else { button::text })
                        .width(Length::Fixed(165.0)),
                    text(&e.job_name).size(12).width(Length::Fixed(140.0)),
                    text(if e.dry_run { "Dry-Run" } else { "Live" })
                        .size(12)
                        .width(Length::Fixed(65.0)),
                    text(&e.exit_label).size(12).color(exit_col).width(Length::Fixed(125.0)),
                    text(format!("{} / {}", e.files_copied, e.files_failed))
                        .size(12)
                        .width(Length::Fixed(85.0)),
                    text(format!(
                        "{} ({})",
                        format_bytes(e.bytes_copied),
                        format_duration(e.duration_secs)
                    ))
                    .size(12)
                    .width(Length::Fixed(125.0)),
                    text(format!("{} -> {}", e.source, e.destination))
                        .size(11)
                        .font(Font::MONOSPACE)
                        .width(Length::Fill),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }

        scrollable(rows).height(Length::Fill).into()
    };

    col = col.push(
        container(list_body)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .style(container::bordered_box),
    );

    col.height(Length::Fill).into()
}
