use crate::app::App;
use crate::message::{BatchItemStatus, Message};
use crate::views::page_header;
use iced::widget::{button, checkbox, column, container, row, rule, scrollable, text};
use iced::{Alignment, Color, Element, Font, Length};

pub fn view_batch_queue(app: &App) -> Element<'_, Message> {
    let can_edit = !app.is_running;
    let can_start = can_edit && !app.batch_items.is_empty();

    let controls = row![
        checkbox(app.batch_stop_on_error)
            .label("Stop on Error (exit >= 8)")
            .on_toggle(Message::ToggleStopOnError),
        button(text("Add Current").size(12))
            .on_press_maybe(can_edit.then_some(Message::AddCurrentToQueue))
            .style(button::secondary),
        button(text("Dry-Run Queue (/L)").size(12))
            .on_press_maybe(can_start.then_some(Message::StartBatchQueue(true)))
            .style(button::success),
        button(text("Run Queue").size(12))
            .on_press_maybe(can_start.then_some(Message::StartBatchQueue(false)))
            .style(button::primary),
        button(text("Stop").size(12))
            .on_press_maybe(app.batch_running.then_some(Message::StopExecution))
            .style(button::danger),
        button(text("Clear").size(12))
            .on_press_maybe(can_start.then_some(Message::ClearBatchQueue))
            .style(button::secondary),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let header = page_header("Multi-Job Batch Execution Queue", controls);

    let body: Element<'_, Message> = if app.batch_items.is_empty() {
        text("Batch queue is empty. Configure a job and click 'Queue +' to enqueue multiple Robocopy jobs.")
            .size(13)
            .into()
    } else {
        let mut rows = column![
            row![
                text("#").size(12).width(Length::Fixed(32.0)),
                text("Job Name").size(12).width(Length::Fixed(170.0)),
                text("Source").size(12).width(Length::FillPortion(2)),
                text("Destination").size(12).width(Length::FillPortion(2)),
                text("Status").size(12).width(Length::Fixed(110.0)),
                text("Actions").size(12).width(Length::Fixed(230.0)),
            ]
            .spacing(8),
            rule::horizontal(1),
        ]
        .spacing(6);

        let len = app.batch_items.len();
        for (idx, item) in app.batch_items.iter().enumerate() {
            let (status_txt, status_col) = match item.status {
                BatchItemStatus::Pending => ("Pending".to_string(), Color::from_rgb(0.45, 0.45, 0.45)),
                BatchItemStatus::Running => ("Running...".to_string(), Color::from_rgb(0.11, 0.31, 0.85)),
                BatchItemStatus::Succeeded(c) => (format!("OK ({c})"), Color::from_rgb(0.08, 0.50, 0.24)),
                BatchItemStatus::Failed(c) => (format!("FAIL ({c})"), Color::from_rgb(0.75, 0.14, 0.14)),
            };

            let actions = row![
                button(text("Up").size(11))
                    .on_press_maybe((idx > 0).then_some(Message::MoveQueueUp(idx)))
                    .style(button::secondary),
                button(text("Dn").size(11))
                    .on_press_maybe((idx + 1 < len).then_some(Message::MoveQueueDown(idx)))
                    .style(button::secondary),
                button(text("Edit").size(11))
                    .on_press(Message::LoadQueueItem(idx))
                    .style(button::secondary),
                button(text("Dup").size(11))
                    .on_press(Message::DuplicateQueueItem(idx))
                    .style(button::secondary),
                button(text("Del").size(11))
                    .on_press_maybe(can_edit.then_some(Message::RemoveQueueItem(idx)))
                    .style(button::danger),
            ]
            .spacing(4);

            rows = rows.push(
                row![
                    text(format!("{}", idx + 1)).size(12).width(Length::Fixed(32.0)),
                    text(&item.job.name).size(12).width(Length::Fixed(170.0)),
                    text(&item.job.source)
                        .size(12)
                        .font(Font::MONOSPACE)
                        .width(Length::FillPortion(2)),
                    text(&item.job.destination)
                        .size(12)
                        .font(Font::MONOSPACE)
                        .width(Length::FillPortion(2)),
                    text(status_txt).size(12).color(status_col).width(Length::Fixed(110.0)),
                    actions.width(Length::Fixed(230.0)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }

        scrollable(rows).height(Length::Fill).into()
    };

    column![
        header,
        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .style(container::bordered_box),
    ]
    .spacing(10)
    .height(Length::Fill)
    .into()
}
