use crate::app::App;
use crate::message::{ActiveModal, Message, WorkspaceTab};
use iced::widget::{button, column, row, rule, scrollable, text};
use iced::{Alignment, Element, Length};

pub fn view_sidebar(app: &App) -> Element<'_, Message> {
    let mut items = column![
        row![
            text("Navigation").size(14).width(Length::Fill),
            button(text("Save...").size(12))
                .on_press(Message::OpenModal(ActiveModal::SaveProfile))
                .style(button::secondary),
        ]
        .align_y(Alignment::Center),
        rule::horizontal(1),
    ]
    .spacing(4);

    for tab in WorkspaceTab::all() {
        let count_suffix = match tab {
            WorkspaceTab::BatchQueue if !app.batch_items.is_empty() => format!(" ({})", app.batch_items.len()),
            WorkspaceTab::LiveConsole if !app.live_file_actions.is_empty() => {
                format!(" ({})", app.live_file_actions.len())
            }
            WorkspaceTab::HistoryReports if !app.run_history.is_empty() => format!(" ({})", app.run_history.len()),
            _ => String::new(),
        };
        let active = app.active_tab == *tab;
        let btn = button(text(format!("{}{}", tab.title(), count_suffix)).size(13))
            .width(Length::Fill)
            .on_press(Message::SelectTab(*tab))
            .style(if active { button::primary } else { button::text });
        items = items.push(btn);
    }

    items = items.push(rule::horizontal(1));
    items = items.push(text("Built-in Presets").size(14));

    for (idx, preset) in app.presets.iter().enumerate() {
        let label = if preset.destructive {
            format!("[!] {}", preset.name)
        } else {
            preset.name.clone()
        };
        let btn = button(text(label).size(12))
            .width(Length::Fill)
            .on_press(Message::ApplyPreset(idx))
            .style(if preset.destructive {
                button::secondary
            } else {
                button::text
            });
        items = items.push(btn);
    }

    items = items.push(rule::horizontal(1));
    items = items.push(text(format!("Saved Profiles ({})", app.profiles.len())).size(14));

    if app.profiles.is_empty() {
        items = items.push(text("No saved profiles yet.").size(12));
    } else {
        for (idx, prof) in app.profiles.iter().enumerate() {
            items = items.push(
                row![
                    button(text(&prof.name).size(12))
                        .width(Length::Fill)
                        .on_press(Message::LoadProfile(idx))
                        .style(button::text),
                    button(text("x").size(12))
                        .on_press(Message::DeleteProfile(prof.id.clone()))
                        .style(button::secondary),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            );
        }
    }

    items = items.push(rule::horizontal(1));
    items = items.push(
        row![
            button(text("Import .rcj").size(12))
                .on_press(Message::ImportRcjFile)
                .style(button::secondary)
                .width(Length::Fill),
            button(text("About").size(12))
                .on_press(Message::OpenModal(ActiveModal::About))
                .style(button::secondary)
                .width(Length::Fill),
        ]
        .spacing(4),
    );

    scrollable(items).height(Length::Fill).into()
}
