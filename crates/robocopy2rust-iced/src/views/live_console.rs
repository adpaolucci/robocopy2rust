use crate::app::App;
use crate::message::{ActionFilter, ConsoleSubTab, Message};
use crate::views::{page_header, section_card};
use iced::widget::{button, column, container, progress_bar, row, rule, scrollable, text, text_input};
use iced::{Alignment, Color, Element, Font, Length};
use robocopy2rust_core::format::{format_bytes, format_duration};
use robocopy2rust_report::ReportFormat;
use robocopy2rust_runner::FileActionKind;

pub fn view_live_console(app: &App) -> Element<'_, Message> {
    let right_hdr: Element<'_, Message> = if let Some(res) = &app.last_result {
        row![
            button(text("Export HTML...").size(12))
                .on_press(Message::ExportReport(ReportFormat::Html))
                .style(button::secondary),
            button(text("Export All (HTML+JSON+MD)...").size(12))
                .on_press(Message::ExportReport(ReportFormat::All))
                .style(button::secondary),
            text(format!("Duration: {}", format_duration(res.duration_secs))).size(12),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
    } else {
        text("Idle").size(12).into()
    };

    let header = page_header("Live Execution Monitor, File Action Diff & Console Stream", right_hdr);

    let active_file = app
        .live_current_file
        .as_deref()
        .unwrap_or("Waiting for file activity...");

    let sum = &app.live_summary;
    let mut kpis = row![
        kpi_box("Dirs Copied", format!("{}/{}", sum.dirs.copied, sum.dirs.total)),
        kpi_box("Files Copied", format!("{}/{}", sum.files.copied, sum.files.total)),
        kpi_box("Skipped", sum.files.skipped.to_string()),
        kpi_box("Extras", sum.files.extras.to_string()),
        kpi_box("Failed", sum.files.failed.to_string()),
        kpi_box("Bytes Copied", format_bytes(sum.bytes.copied)),
    ]
    .spacing(8);

    if let Some(mbm) = sum.speed_mb_per_min {
        kpis = kpis.push(kpi_box("Speed", format!("{mbm:.2} MB/min")));
    }

    let mut progress_col = column![
        row![
            progress_bar(0.0..=100.0, app.live_progress_pct).length(Length::Fixed(280.0)),
            text(format!("{:.0}%", app.live_progress_pct)).size(13),
            text(active_file).size(12).font(Font::MONOSPACE).width(Length::Fill),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        kpis,
    ]
    .spacing(8);

    if let Some(res) = &app.last_result {
        let color = if res.exit_info.is_success {
            Color::from_rgb(0.08, 0.50, 0.24)
        } else {
            Color::from_rgb(0.75, 0.14, 0.14)
        };
        progress_col = progress_col.push(
            container(
                text(format!(
                    "Exit Code {}: {} — {}",
                    res.exit_code, res.exit_info.short_label, res.exit_info.summary
                ))
                .size(13)
                .color(color),
            )
            .padding(8)
            .width(Length::Fill)
            .style(container::bordered_box),
        );
    }

    let subtab_bar = row![
        button(text(format!("File Actions / Diff ({})", app.live_file_actions.len())).size(12))
            .on_press(Message::SwitchConsoleSubTab(ConsoleSubTab::FileActions))
            .style(if app.console_subtab == ConsoleSubTab::FileActions {
                button::primary
            } else {
                button::secondary
            }),
        button(text(format!("Raw Console Stream ({} lines)", app.live_log_lines.len())).size(12))
            .on_press(Message::SwitchConsoleSubTab(ConsoleSubTab::RawLog))
            .style(if app.console_subtab == ConsoleSubTab::RawLog {
                button::primary
            } else {
                button::secondary
            }),
    ]
    .spacing(6)
    .height(Length::Shrink)
    .align_y(Alignment::Center);

    let sub_controls: Element<'_, Message> = if app.console_subtab == ConsoleSubTab::FileActions {
        let mut r = row![].spacing(6).align_y(Alignment::Center);
        for (af, lbl) in [
            (ActionFilter::All, "All"),
            (ActionFilter::CopiedNew, "Copied/New"),
            (ActionFilter::Extra, "Extras"),
            (ActionFilter::Skipped, "Skipped"),
            (ActionFilter::Errors, "Errors"),
        ] {
            r = r.push(button(text(lbl).size(12)).on_press(Message::SetActionFilter(af)).style(
                if app.action_filter == af {
                    button::primary
                } else {
                    button::secondary
                },
            ));
        }
        r = r.push(
            text_input("Filter path...", &app.action_search)
                .on_input(Message::ActionSearchChanged)
                .size(12)
                .width(Length::Fixed(220.0)),
        );
        r.into()
    } else {
        row![
            text_input("Filter log lines...", &app.log_search)
                .on_input(Message::LogSearchChanged)
                .size(12)
                .width(Length::Fixed(220.0)),
            button(text("Copy Log").size(12))
                .on_press(Message::CopyLogToClipboard)
                .style(button::secondary),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
    };

    let body: Element<'_, Message> = match app.console_subtab {
        ConsoleSubTab::FileActions => view_actions_list(app),
        ConsoleSubTab::RawLog => view_raw_log(app),
    };

    column![
        header,
        section_card("Live Transfer Progress & Summary Counters", progress_col),
        row![subtab_bar, sub_controls].spacing(8).align_y(Alignment::Center),
        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(8)
            .style(container::bordered_box),
    ]
    .spacing(8)
    .height(Length::Fill)
    .into()
}

fn kpi_box<'a>(label: &'a str, value: String) -> Element<'a, Message> {
    container(column![text(label).size(11), text(value).size(14)].spacing(2))
        .padding([6, 10])
        .style(container::bordered_box)
        .into()
}

fn view_actions_list(app: &App) -> Element<'_, Message> {
    if app.action_filter == ActionFilter::Errors {
        if app.live_errors.is_empty() {
            return text("No Robocopy errors captured.").size(13).into();
        }
        let mut col = column![].spacing(4);
        for err in &app.live_errors {
            col = col.push(
                text(err)
                    .size(12)
                    .font(Font::MONOSPACE)
                    .color(Color::from_rgb(0.75, 0.14, 0.14)),
            );
        }
        return scrollable(col).height(Length::Fill).into();
    }

    let q = app.action_search.trim().to_ascii_lowercase();
    let mut rows = column![
        row![
            text("Class").size(12).width(Length::Fixed(105.0)),
            text("Size").size(12).width(Length::Fixed(90.0)),
            text("Timestamp").size(12).width(Length::Fixed(140.0)),
            text("Path").size(12).width(Length::Fill),
        ]
        .spacing(8),
        rule::horizontal(1),
    ]
    .spacing(4);

    let mut matched = 0usize;
    for rec in app.live_file_actions.iter().take(5000) {
        let cat_ok = match app.action_filter {
            ActionFilter::All => true,
            ActionFilter::CopiedNew => rec.kind.is_copied_or_new(),
            ActionFilter::Extra => rec.kind.is_extra(),
            ActionFilter::Skipped => rec.kind.is_skipped(),
            ActionFilter::Errors => matches!(rec.kind, FileActionKind::Failed | FileActionKind::Mismatch),
        };
        if !cat_ok {
            continue;
        }
        if !q.is_empty()
            && !rec.path.to_ascii_lowercase().contains(&q)
            && !rec.kind.label().to_ascii_lowercase().contains(&q)
        {
            continue;
        }
        matched += 1;
        let sz = rec.size_bytes.map(format_bytes).unwrap_or_else(|| "-".to_string());
        let ts = rec.timestamp.as_deref().unwrap_or("-");
        let kind_color = if rec.kind.is_copied_or_new() {
            Color::from_rgb(0.08, 0.50, 0.24)
        } else if rec.kind.is_extra() {
            Color::from_rgb(0.11, 0.31, 0.85)
        } else if matches!(rec.kind, FileActionKind::Failed | FileActionKind::Mismatch) {
            Color::from_rgb(0.75, 0.14, 0.14)
        } else {
            Color::from_rgb(0.45, 0.45, 0.45)
        };

        rows = rows.push(
            row![
                text(rec.kind.label())
                    .size(12)
                    .color(kind_color)
                    .width(Length::Fixed(105.0)),
                text(sz).size(12).width(Length::Fixed(90.0)),
                text(ts).size(12).width(Length::Fixed(140.0)),
                text(&rec.path).size(12).font(Font::MONOSPACE).width(Length::Fill),
            ]
            .spacing(8),
        );
    }

    if matched == 0 {
        return text("No file actions recorded matching the active filter.")
            .size(13)
            .into();
    }

    scrollable(rows).height(Length::Fill).into()
}

fn view_raw_log(app: &App) -> Element<'_, Message> {
    let q = app.log_search.trim().to_ascii_lowercase();
    let mut col = column![].spacing(2);
    let mut count = 0usize;
    let total = app.live_log_lines.len();
    let skip = total.saturating_sub(2500);

    for line in app.live_log_lines.iter().skip(skip) {
        if !q.is_empty() && !line.to_ascii_lowercase().contains(&q) {
            continue;
        }
        count += 1;
        let line_el = if line.contains("ERROR") || line.starts_with("[ERROR]") {
            text(line)
                .size(12)
                .font(Font::MONOSPACE)
                .color(Color::from_rgb(0.75, 0.14, 0.14))
        } else {
            text(line).size(12).font(Font::MONOSPACE)
        };
        col = col.push(line_el);
    }

    if count == 0 {
        return text("Console output is empty.").size(13).into();
    }

    scrollable(col).height(Length::Fill).into()
}
