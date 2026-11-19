use crate::app::App;
use crate::message::{ActiveModal, CadenceChoice, Message};
use iced::widget::{button, checkbox, column, container, mouse_area, pick_list, row, rule, text, text_input};
use iced::{Alignment, Color, Element, Font, Length};
use robocopy2rust_core::{ScheduleCadence, build_schtasks_command, decode_exit_code};

pub fn view_modal(app: &App) -> Element<'_, Message> {
    let inner: Element<'_, Message> = match app.active_modal {
        ActiveModal::None => column![].into(),
        ActiveModal::ConfirmDestructive => view_confirm_destructive(app),
        ActiveModal::ImportCommand => view_import_command(app),
        ActiveModal::SaveProfile => view_save_profile(app),
        ActiveModal::TaskScheduler => view_task_scheduler(app),
        ActiveModal::ExitCodeReference => view_exit_code_modal(app),
        ActiveModal::About => view_about_modal(app),
    };

    // Stop clicks inside the modal card from closing via backdrop mouse_area
    mouse_area(
        container(inner)
            .padding(18)
            .width(Length::Fixed(560.0))
            .style(container::rounded_box),
    )
    .on_press(Message::OpenModal(app.active_modal))
    .into()
}

fn view_confirm_destructive(app: &App) -> Element<'_, Message> {
    let Some(pending) = &app.pending_destructive_job else {
        return column![].into();
    };
    let flags = pending.destructive_switch_names().join(", ");

    column![
        text("Confirm Destructive Robocopy Execution")
            .size(17)
            .color(Color::from_rgb(0.75, 0.14, 0.14)),
        rule::horizontal(1),
        text(format!("Destructive flag(s) active: {flags}")).size(13),
        text("Files in Destination (/MIR /PURGE) or Source (/MOV /MOVE) may be permanently deleted.").size(13),
        text(format!("Source:      {}", pending.source))
            .size(12)
            .font(Font::MONOSPACE),
        text(format!("Destination: {}", pending.destination))
            .size(12)
            .font(Font::MONOSPACE),
        row![
            button(text("Run Safe Dry-Run (/L) First").size(13))
                .on_press(Message::DryRunPendingInstead)
                .style(button::success),
            button(text("Cancel").size(13))
                .on_press(Message::CloseModal)
                .style(button::secondary),
            button(text("Execute Destructive Job").size(13))
                .on_press(Message::ConfirmDestructiveExecute)
                .style(button::danger),
        ]
        .spacing(8),
    ]
    .spacing(10)
    .into()
}

fn view_import_command(app: &App) -> Element<'_, Message> {
    column![
        text("Import / Parse Robocopy Command Line").size(17),
        rule::horizontal(1),
        text("Paste any Microsoft Robocopy command string to populate the GUI switches:").size(13),
        text_input(r#"robocopy "C:\Src" "D:\Dst" /MIR /MT:16"#, &app.import_cmd_text)
            .on_input(Message::ImportCmdTextChanged)
            .on_submit(Message::ApplyImportCmd)
            .font(Font::MONOSPACE)
            .size(12)
            .padding(8),
        row![
            button(text("Parse & Load into Designer").size(13))
                .on_press(Message::ApplyImportCmd)
                .style(button::primary),
            button(text("Cancel").size(13))
                .on_press(Message::CloseModal)
                .style(button::secondary),
        ]
        .spacing(8),
    ]
    .spacing(10)
    .into()
}

fn view_save_profile(app: &App) -> Element<'_, Message> {
    let valid = !app.save_profile_name.trim().is_empty();
    column![
        text("Save Robocopy Job Profile").size(17),
        rule::horizontal(1),
        row![
            text("Name:").size(13).width(Length::Fixed(90.0)),
            text_input("Profile Name", &app.save_profile_name)
                .on_input(Message::SaveProfileNameChanged)
                .size(13)
                .width(Length::Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        row![
            text("Description:").size(13).width(Length::Fixed(90.0)),
            text_input("Optional notes", &app.save_profile_desc)
                .on_input(Message::SaveProfileDescChanged)
                .size(13)
                .width(Length::Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        row![
            button(text("Save Profile").size(13))
                .on_press_maybe(valid.then_some(Message::ConfirmSaveProfile))
                .style(button::primary),
            button(text("Cancel").size(13))
                .on_press(Message::CloseModal)
                .style(button::secondary),
        ]
        .spacing(8),
    ]
    .spacing(10)
    .into()
}

fn view_task_scheduler(app: &App) -> Element<'_, Message> {
    let generated = build_schtasks_command(&app.task_sched_name, &app.task_sched_target, &app.task_sched_spec);

    let mut col = column![
        text("Windows Task Scheduler (schtasks) Generator").size(17),
        rule::horizontal(1),
        row![
            text("Task Name:").size(13).width(Length::Fixed(95.0)),
            text_input("Robocopy2Rust_DailySync", &app.task_sched_name)
                .on_input(Message::TaskNameChanged)
                .size(13)
                .width(Length::Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        row![
            text("Cadence:").size(13).width(Length::Fixed(95.0)),
            pick_list(
                CadenceChoice::ALL,
                Some(CadenceChoice(app.task_sched_spec.cadence)),
                Message::TaskCadenceChanged,
            )
            .text_size(13),
            text("Time (HH:MM):").size(13),
            text_input("02:00", &app.task_sched_spec.start_time)
                .on_input(Message::TaskStartTimeChanged)
                .size(13)
                .width(Length::Fixed(75.0)),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        checkbox(app.task_sched_spec.run_highest)
            .label("Run with highest privileges (/RL HIGHEST)")
            .on_toggle(Message::TaskRunHighestToggled),
    ]
    .spacing(10);

    if app.task_sched_spec.cadence == ScheduleCadence::Weekly {
        col = col.push(
            row![
                text("Days (/D):").size(13).width(Length::Fixed(95.0)),
                text_input("MON,WED,FRI", app.task_sched_spec.days.as_deref().unwrap_or(""))
                    .on_input(Message::TaskDaysChanged)
                    .size(13)
                    .width(Length::Fill),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }

    col = col
        .push(
            row![
                text("Target (/TR):").size(13).width(Length::Fixed(95.0)),
                text_input("Command or script path", &app.task_sched_target)
                    .on_input(Message::TaskTargetChanged)
                    .font(Font::MONOSPACE)
                    .size(12)
                    .width(Length::Fill),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .push(text("Generated schtasks Command:").size(13))
        .push(text_input("", &generated).font(Font::MONOSPACE).size(12).padding(6))
        .push(
            row![
                button(text("Copy schtasks Command").size(13))
                    .on_press(Message::CopySchtasksCmd)
                    .style(button::primary),
                button(text("Close").size(13))
                    .on_press(Message::CloseModal)
                    .style(button::secondary),
            ]
            .spacing(8),
        );

    col.into()
}

fn view_exit_code_modal(app: &App) -> Element<'_, Message> {
    let info = decode_exit_code(app.exit_code_val);
    let col_status = if info.is_success {
        Color::from_rgb(0.08, 0.50, 0.24)
    } else {
        Color::from_rgb(0.75, 0.14, 0.14)
    };

    let mut quick = row![
        text("Exit Code:").size(13),
        text_input("1", &app.exit_code_input)
            .on_input(Message::ExitCodeInputChanged)
            .size(13)
            .width(Length::Fixed(60.0)),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    for q in [0, 1, 2, 3, 4, 8, 16] {
        quick = quick.push(
            button(text(q.to_string()).size(12))
                .on_press(Message::SelectQuickExitCode(q))
                .style(if app.exit_code_val == q {
                    button::primary
                } else {
                    button::secondary
                }),
        );
    }

    let mut bits = column![].spacing(4);
    for b in info.bit_descriptions {
        bits = bits.push(text(format!("• {b}")).size(12));
    }

    column![
        text("Robocopy Bitmask Exit Code Decoder (0-16)").size(17),
        rule::horizontal(1),
        quick,
        text(info.short_label).size(15).color(col_status),
        text(info.summary).size(13),
        rule::horizontal(1),
        bits,
        text("Rule: Exit codes 0..=7 indicate success; exit codes >= 8 indicate copy or fatal errors.").size(11),
        button(text("Close").size(13))
            .on_press(Message::CloseModal)
            .style(button::secondary),
    ]
    .spacing(8)
    .into()
}

fn view_about_modal(app: &App) -> Element<'_, Message> {
    let robo_path = app
        .env
        .robocopy_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "Not found on host OS".to_string());

    column![
        text(concat!("Robocopy2Rust Management Console v", env!("CARGO_PKG_VERSION"))).size(17),
        rule::horizontal(1),
        text("Cross-Platform Iced 0.14 Edition (wgpu + tiny-skia CPU fallback)").size(13),
        text("Part of the GUAP Ecosystem (guap.ca) • Licensed under Apache-2.0").size(12),
        text(format!("Host OS: {}", app.env.os)).size(12),
        text(format!("Robocopy Path: {robo_path}")).size(12),
        text(format!("Config Store: {}", app.env.config_dir.display())).size(12),
        button(text("Close").size(13))
            .on_press(Message::CloseModal)
            .style(button::primary),
    ]
    .spacing(8)
    .into()
}
