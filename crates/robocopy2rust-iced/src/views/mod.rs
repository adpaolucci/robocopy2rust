pub mod batch_queue;
pub mod designer;
pub mod history_view;
pub mod live_console;
pub mod modals;
pub mod options_tabs;
pub mod sidebar;

use crate::message::Message;
use iced::widget::{column, container, row, rule, text};
use iced::{Alignment, Element, Length};

pub fn section_card<'a>(title: impl Into<String>, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    let title_str: String = title.into();
    let body = column![text(title_str).size(15), rule::horizontal(1), content.into()].spacing(8);

    container(body)
        .width(Length::Fill)
        .padding(12)
        .style(container::bordered_box)
        .into()
}

pub fn page_header<'a>(title: &'a str, right_controls: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    row![text(title).size(17).width(Length::Fill), right_controls.into()]
        .align_y(Alignment::Center)
        .spacing(8)
        .into()
}
