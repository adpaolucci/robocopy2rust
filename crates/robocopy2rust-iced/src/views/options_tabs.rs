use crate::app::App;
use crate::message::{LogModeChoice, Message, SparseChoice};
use crate::views::{page_header, section_card};
use iced::widget::{button, checkbox, column, pick_list, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length};

pub fn view_copy_transfer(app: &App) -> Element<'_, Message> {
    let c = &app.job.copy;

    let header = page_header(
        "Copy & Transfer Options (/S, /E, /MIR, /COPY, /DCOPY, /MT, /MOVE, /Z, /B)",
        button(text("Reset Copy Defaults").size(12))
            .on_press(Message::ResetCopyDefaults)
            .style(button::secondary),
    );

    let recursion = column![
        row![
            checkbox(c.subdirectories_non_empty)
                .label("/S (Subdirs, skip empty)")
                .on_toggle(Message::ToggleSubdirsS),
            checkbox(c.subdirectories_including_empty)
                .label("/E (Subdirs + empty)")
                .on_toggle(Message::ToggleSubdirsE),
            checkbox(c.purge)
                .label("/PURGE (Delete dest extras)")
                .on_toggle(Message::TogglePurge),
            checkbox(c.mirror)
                .label("/MIR (Mirror = /E + /PURGE)")
                .on_toggle(Message::ToggleMirror),
        ]
        .spacing(12),
        row![
            checkbox(c.move_files)
                .label("/MOV (Move files)")
                .on_toggle(Message::ToggleMoveFiles),
            checkbox(c.move_files_and_dirs)
                .label("/MOVE (Move files & dirs)")
                .on_toggle(Message::ToggleMoveAll),
            text("/LEV:n:").size(13),
            text_input("levels", &app.max_levels_text)
                .on_input(Message::MaxLevelsChanged)
                .size(13)
                .width(Length::Fixed(70.0)),
            text("/MT:n (1-128):").size(13),
            text_input("8", &app.threads_text)
                .on_input(Message::ThreadsChanged)
                .size(13)
                .width(Length::Fixed(70.0)),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    ]
    .spacing(8);

    let engine = column![
        row![
            checkbox(c.restartable)
                .label("/Z (Restartable)")
                .on_toggle(Message::ToggleRestartableZ),
            checkbox(c.backup_mode)
                .label("/B (Backup mode)")
                .on_toggle(Message::ToggleBackupB),
            checkbox(c.restartable_or_backup)
                .label("/ZB (Restart else Backup)")
                .on_toggle(Message::ToggleRestartBackupZB),
            checkbox(c.unbuffered_io)
                .label("/J (Unbuffered I/O)")
                .on_toggle(Message::ToggleUnbufferedJ),
            checkbox(c.efs_raw).label("/EFSRAW").on_toggle(Message::ToggleEfsRaw),
        ]
        .spacing(12),
        row![
            checkbox(c.network_compress)
                .label("/COMPRESS (SMB)")
                .on_toggle(Message::ToggleCompress),
            checkbox(c.no_offload)
                .label("/NOOFFLOAD")
                .on_toggle(Message::ToggleNoOffload),
            checkbox(c.no_clone).label("/NOCLONE").on_toggle(Message::ToggleNoClone),
            text("/SPARSE:").size(13),
            pick_list(
                SparseChoice::ALL,
                Some(SparseChoice(c.sparse_mode)),
                Message::SparseModeChanged,
            )
            .text_size(13),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    ]
    .spacing(8);

    let metadata = column![
        row![
            text("File /COPY:").size(13),
            checkbox(c.copy_flags.data)
                .label("D (Data)")
                .on_toggle(|b| Message::CopyFlagChanged('D', b)),
            checkbox(c.copy_flags.attributes)
                .label("A (Attr)")
                .on_toggle(|b| Message::CopyFlagChanged('A', b)),
            checkbox(c.copy_flags.timestamps)
                .label("T (Times)")
                .on_toggle(|b| Message::CopyFlagChanged('T', b)),
            checkbox(c.copy_flags.security)
                .label("S (ACL)")
                .on_toggle(|b| Message::CopyFlagChanged('S', b)),
            checkbox(c.copy_flags.owner)
                .label("O (Owner)")
                .on_toggle(|b| Message::CopyFlagChanged('O', b)),
            checkbox(c.copy_flags.auditing)
                .label("U (Audit)")
                .on_toggle(|b| Message::CopyFlagChanged('U', b)),
            checkbox(c.copy_flags.skip_alt_data_streams)
                .label("X (No ADS)")
                .on_toggle(|b| Message::CopyFlagChanged('X', b)),
            checkbox(c.no_copy).label("/NOCOPY").on_toggle(Message::ToggleNoCopy),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        row![
            button(text("DAT Default").size(12))
                .on_press(Message::CopyPresetDat)
                .style(button::secondary),
            button(text("/SEC (DATS)").size(12))
                .on_press(Message::CopyPresetSec)
                .style(button::secondary),
            button(text("/COPYALL").size(12))
                .on_press(Message::CopyPresetAll)
                .style(button::secondary),
            checkbox(c.sec_fix).label("/SECFIX").on_toggle(Message::ToggleSecFix),
            checkbox(c.tim_fix).label("/TIMFIX").on_toggle(Message::ToggleTimFix),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        row![
            text("Dir /DCOPY:").size(13),
            checkbox(c.dir_copy_flags.data)
                .label("D (Data)")
                .on_toggle(|b| Message::DirCopyFlagChanged('D', b)),
            checkbox(c.dir_copy_flags.attributes)
                .label("A (Attr)")
                .on_toggle(|b| Message::DirCopyFlagChanged('A', b)),
            checkbox(c.dir_copy_flags.timestamps)
                .label("T (Times)")
                .on_toggle(|b| Message::DirCopyFlagChanged('T', b)),
            checkbox(c.dir_copy_flags.extended_attributes)
                .label("E (EAs)")
                .on_toggle(|b| Message::DirCopyFlagChanged('E', b)),
            checkbox(c.dir_copy_flags.skip_alt_data_streams)
                .label("X (No ADS)")
                .on_toggle(|b| Message::DirCopyFlagChanged('X', b)),
            checkbox(c.no_dir_copy)
                .label("/NODCOPY")
                .on_toggle(Message::ToggleNoDirCopy),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(8);

    let links_attrs = column![
        row![
            checkbox(c.create_tree_only)
                .label("/CREATE (0-byte skeleton)")
                .on_toggle(Message::ToggleCreateTree),
            checkbox(c.fat_names)
                .label("/FAT (8.3 names)")
                .on_toggle(Message::ToggleFatNames),
            checkbox(c.disable_long_paths)
                .label("/256 (Disable >256 paths)")
                .on_toggle(Message::ToggleDisableLongPaths),
            checkbox(c.copy_junctions)
                .label("/SJ (Copy junctions)")
                .on_toggle(Message::ToggleCopyJunctions),
            checkbox(c.copy_symlinks)
                .label("/SL (Copy symlinks)")
                .on_toggle(Message::ToggleCopySymlinks),
        ]
        .spacing(10),
        row![
            text("/A+: (Add Attrs):").size(13),
            text_input("RASHCNETO", &app.add_attrs_text)
                .on_input(Message::AddAttrsChanged)
                .size(13)
                .width(Length::Fixed(130.0)),
            text("/A-: (Remove Attrs):").size(13),
            text_input("RASHCNETO", &app.rem_attrs_text)
                .on_input(Message::RemoveAttrsChanged)
                .size(13)
                .width(Length::Fixed(130.0)),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    ]
    .spacing(8);

    let content = column![
        section_card("Directory Traversal & Sync Mode", recursion),
        section_card("Access Mode, Multi-Threading & Transfer Engine", engine),
        section_card("File & Directory Metadata Flags (/COPY & /DCOPY)", metadata),
        section_card("Attributes, Symbolic Links & Path Rules", links_attrs),
    ]
    .spacing(10);

    column![header, scrollable(content).height(Length::Fill)]
        .spacing(10)
        .height(Length::Fill)
        .into()
}

pub fn view_file_selection(app: &App) -> Element<'_, Message> {
    let s = &app.job.selection;
    let header = page_header(
        "File Selection, Exclusions, Size, Age & Junction Filters",
        button(text("Reset Selection Defaults").size(12))
            .on_press(Message::ResetSelectionDefaults)
            .style(button::secondary),
    );

    let exclusions = column![
        row![
            text("/XF (Exclude Files):").size(13).width(Length::Fixed(150.0)),
            text_input(r#"*.tmp *.log ~$* "Thumbs.db""#, &app.exclude_files_text)
                .on_input(Message::ExcludeFilesChanged)
                .size(13)
                .width(Length::Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        row![
            text("/XD (Exclude Dirs):").size(13).width(Length::Fixed(150.0)),
            text_input(r#".git node_modules target"#, &app.exclude_dirs_text)
                .on_input(Message::ExcludeDirsChanged)
                .size(13)
                .width(Length::Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        row![
            checkbox(s.exclude_junctions)
                .label("/XJ (Exclude all junctions & symlinks)")
                .on_toggle(Message::ToggleExcludeJunctions),
            checkbox(s.exclude_dir_junctions)
                .label("/XJD (Exclude dir junctions)")
                .on_toggle(Message::ToggleExcludeDirJunctions),
            checkbox(s.exclude_file_symlinks)
                .label("/XJF (Exclude file symlinks)")
                .on_toggle(Message::ToggleExcludeFileSymlinks),
        ]
        .spacing(12),
    ]
    .spacing(8);

    let classes = column![
        row![
            checkbox(s.exclude_changed)
                .label("/XC (Excl Changed)")
                .on_toggle(Message::ToggleExcludeChanged),
            checkbox(s.exclude_newer)
                .label("/XN (Excl Newer)")
                .on_toggle(Message::ToggleExcludeNewer),
            checkbox(s.exclude_older)
                .label("/XO (Excl Older)")
                .on_toggle(Message::ToggleExcludeOlder),
            checkbox(s.exclude_extra)
                .label("/XX (Excl Extra)")
                .on_toggle(Message::ToggleExcludeExtra),
            checkbox(s.exclude_lonely)
                .label("/XL (Excl Lonely)")
                .on_toggle(Message::ToggleExcludeLonely),
        ]
        .spacing(10),
        row![
            checkbox(s.include_same)
                .label("/IS (Incl Same)")
                .on_toggle(Message::ToggleIncludeSame),
            checkbox(s.include_tweaked)
                .label("/IT (Incl Tweaked)")
                .on_toggle(Message::ToggleIncludeTweaked),
            checkbox(s.include_modified)
                .label("/IM (Incl Modified)")
                .on_toggle(Message::ToggleIncludeModified),
            checkbox(s.fat_file_times)
                .label("/FFT (2s FAT times)")
                .on_toggle(Message::ToggleFatFileTimes),
            checkbox(s.compensate_dst)
                .label("/DST (1h DST)")
                .on_toggle(Message::ToggleCompensateDst),
        ]
        .spacing(10),
    ]
    .spacing(8);

    let attrs = column![
        row![
            checkbox(s.archive_only)
                .label("/A (Archive bit set)")
                .on_toggle(Message::ToggleArchiveOnly),
            checkbox(s.archive_and_reset)
                .label("/M (Archive bit set & reset)")
                .on_toggle(Message::ToggleArchiveReset),
        ]
        .spacing(12),
        row![
            text("/IA: (Include Attrs):").size(13),
            text_input("RASHCNETO", &app.inc_attrs_text)
                .on_input(Message::IncludeAttrsChanged)
                .size(13)
                .width(Length::Fixed(130.0)),
            text("/XA: (Exclude Attrs):").size(13),
            text_input("RASHCNETO", &app.exc_attrs_text)
                .on_input(Message::ExcludeAttrsChanged)
                .size(13)
                .width(Length::Fixed(130.0)),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    ]
    .spacing(8);

    let bounds = column![
        row![
            text("/MAX:n (Max bytes):").size(13),
            text_input("bytes", &app.max_size_text)
                .on_input(Message::MaxSizeChanged)
                .size(13)
                .width(Length::Fixed(130.0)),
            text("/MIN:n (Min bytes):").size(13),
            text_input("bytes", &app.min_size_text)
                .on_input(Message::MinSizeChanged)
                .size(13)
                .width(Length::Fixed(130.0)),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        row![
            text("/MAXAGE:").size(13),
            text_input("days or YYYYMMDD", s.max_age.as_deref().unwrap_or(""))
                .on_input(Message::MaxAgeChanged)
                .size(13)
                .width(Length::Fixed(140.0)),
            text("/MINAGE:").size(13),
            text_input("days or YYYYMMDD", s.min_age.as_deref().unwrap_or(""))
                .on_input(Message::MinAgeChanged)
                .size(13)
                .width(Length::Fixed(140.0)),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        row![
            text("/MAXLAD:").size(13),
            text_input("days or YYYYMMDD", s.max_last_access.as_deref().unwrap_or(""))
                .on_input(Message::MaxLadChanged)
                .size(13)
                .width(Length::Fixed(140.0)),
            text("/MINLAD:").size(13),
            text_input("days or YYYYMMDD", s.min_last_access.as_deref().unwrap_or(""))
                .on_input(Message::MinLadChanged)
                .size(13)
                .width(Length::Fixed(140.0)),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    ]
    .spacing(8);

    let content = column![
        section_card("Excluded Files, Directories & Reparse Points", exclusions),
        section_card("File Comparison Classes & Timestamp Granularity", classes),
        section_card("Archive Bit & NTFS Attribute Filters", attrs),
        section_card("File Size & Age / Last Access Date Bounds", bounds),
    ]
    .spacing(10);

    column![header, scrollable(content).height(Length::Fill)]
        .spacing(10)
        .height(Length::Fill)
        .into()
}

pub fn view_perf_retry(app: &App) -> Element<'_, Message> {
    let header = page_header(
        "Performance Throttling, Monitoring, Run Hours & Retry Policies",
        button(text("Reset Retry & Throttle Defaults").size(12))
            .on_press(Message::ResetRetryDefaults)
            .style(button::secondary),
    );

    let retry = column![
        row![
            text("/R:n (Retries):").size(13),
            text_input("3", &app.retries_text)
                .on_input(Message::RetriesChanged)
                .size(13)
                .width(Length::Fixed(90.0)),
            text("/W:n (Wait secs):").size(13),
            text_input("5", &app.wait_text)
                .on_input(Message::WaitSecsChanged)
                .size(13)
                .width(Length::Fixed(90.0)),
            checkbox(app.job.retry.wait_for_sharenames)
                .label("/TBD (Wait for sharenames)")
                .on_toggle(Message::ToggleWaitShares),
            checkbox(app.job.retry.save_defaults_to_registry)
                .label("/REG (Save defaults)")
                .on_toggle(Message::ToggleRegDefaults),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        row![
            checkbox(app.job.retry.low_free_space_mode)
                .label("/LFSM (Low Free Space Mode)")
                .on_toggle(Message::ToggleLfsm),
            text("/LFSM:n[KMG] Floor:").size(13),
            text_input(
                "e.g. 500M or 2G",
                app.job.retry.low_free_space_floor.as_deref().unwrap_or("")
            )
            .on_input(Message::LfsmFloorChanged)
            .size(13)
            .width(Length::Fixed(140.0)),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    ]
    .spacing(8);

    let throttle = column![
        row![
            text("/IoMaxSize:").size(13),
            text_input("64K / 1M", app.job.throttle.io_max_size.as_deref().unwrap_or(""))
                .on_input(Message::IoMaxSizeChanged)
                .size(13)
                .width(Length::Fixed(110.0)),
            text("/IoRate:").size(13),
            text_input("25M", app.job.throttle.io_rate.as_deref().unwrap_or(""))
                .on_input(Message::IoRateChanged)
                .size(13)
                .width(Length::Fixed(110.0)),
            text("/Threshold:").size(13),
            text_input("10M", app.job.throttle.threshold.as_deref().unwrap_or(""))
                .on_input(Message::ThresholdChanged)
                .size(13)
                .width(Length::Fixed(110.0)),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        row![
            text("/IPG:n (Inter-Packet Gap ms):").size(13),
            text_input("ms", &app.ipg_text)
                .on_input(Message::IpgChanged)
                .size(13)
                .width(Length::Fixed(100.0)),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    ]
    .spacing(8);

    let monitoring = column![
        row![
            text("/MON:n (Changes):").size(13),
            text_input("count", &app.mon_text)
                .on_input(Message::MonChangesChanged)
                .size(13)
                .width(Length::Fixed(90.0)),
            text("/MOT:m (Minutes):").size(13),
            text_input("mins", &app.mot_text)
                .on_input(Message::MotMinutesChanged)
                .size(13)
                .width(Length::Fixed(90.0)),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        row![
            text("/RH:hhmm-hhmm:").size(13),
            text_input("2200-0600", app.job.copy.run_hours.as_deref().unwrap_or(""))
                .on_input(Message::RunHoursChanged)
                .size(13)
                .width(Length::Fixed(120.0)),
            checkbox(app.job.copy.per_file_run_hours)
                .label("/PF (Check run hours per file)")
                .on_toggle(Message::TogglePerFileRunHours),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    ]
    .spacing(8);

    let content = column![
        section_card("Retry & Disk Space Protection (/R, /W, /REG, /TBD, /LFSM)", retry),
        section_card("I/O Throttling & Bandwidth Control", throttle),
        section_card("Continuous Monitoring & Run Hours", monitoring),
    ]
    .spacing(10);

    column![header, scrollable(content).height(Length::Fill)]
        .spacing(10)
        .height(Length::Fill)
        .into()
}

pub fn view_logging_jobs(app: &App) -> Element<'_, Message> {
    let l = &app.job.logging;
    let jf = &app.job.job_files;
    let header = page_header(
        "Logging Output, Robocopy Job Directives & Extra Switches",
        button(text("Reset Logging Defaults").size(12))
            .on_press(Message::ResetLoggingDefaults)
            .style(button::secondary),
    );

    let flags = column![
        row![
            checkbox(l.list_only)
                .label("/L (List only - Dry Run)")
                .on_toggle(Message::ToggleListOnly),
            checkbox(l.report_extra)
                .label("/X (Extras)")
                .on_toggle(Message::ToggleReportExtra),
            checkbox(l.verbose)
                .label("/V (Verbose)")
                .on_toggle(Message::ToggleVerbose),
            checkbox(l.include_timestamps)
                .label("/TS (Timestamps)")
                .on_toggle(Message::ToggleTimestamps),
            checkbox(l.full_paths)
                .label("/FP (Full paths)")
                .on_toggle(Message::ToggleFullPaths),
            checkbox(l.sizes_in_bytes)
                .label("/BYTES")
                .on_toggle(Message::ToggleBytes),
            checkbox(l.unicode_output)
                .label("/UNICODE")
                .on_toggle(Message::ToggleUnicodeOutput),
        ]
        .spacing(10),
        row![
            checkbox(l.no_sizes).label("/NS").on_toggle(Message::ToggleNoSizes),
            checkbox(l.no_classes).label("/NC").on_toggle(Message::ToggleNoClasses),
            checkbox(l.no_file_list)
                .label("/NFL")
                .on_toggle(Message::ToggleNoFileList),
            checkbox(l.no_dir_list)
                .label("/NDL")
                .on_toggle(Message::ToggleNoDirList),
            checkbox(l.no_progress)
                .label("/NP")
                .on_toggle(Message::ToggleNoProgress),
            checkbox(l.show_eta).label("/ETA").on_toggle(Message::ToggleEta),
            checkbox(l.no_job_header)
                .label("/NJH")
                .on_toggle(Message::ToggleNoJobHeader),
            checkbox(l.no_job_summary)
                .label("/NJS")
                .on_toggle(Message::ToggleNoJobSummary),
            checkbox(l.tee).label("/TEE").on_toggle(Message::ToggleTee),
        ]
        .spacing(10),
    ]
    .spacing(8);

    let log_file = row![
        text("Log Mode:").size(13),
        pick_list(
            LogModeChoice::ALL,
            Some(LogModeChoice(l.log_mode)),
            Message::LogModeChanged
        )
        .text_size(13),
        text("Log Path:").size(13),
        text_input(r"C:\logs\robocopy.log", l.log_path.as_deref().unwrap_or(""))
            .on_input(Message::LogPathChanged)
            .size(13)
            .width(Length::Fill),
        button(text("Browse...").size(12))
            .on_press(Message::BrowseLogFile)
            .style(button::secondary),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let job_files = column![
        row![
            text("/JOB:").size(13),
            text_input("jobname", jf.load_job.as_deref().unwrap_or(""))
                .on_input(Message::LoadJobChanged)
                .size(13)
                .width(Length::Fixed(200.0)),
            text("/SAVE:").size(13),
            text_input("jobname", jf.save_job.as_deref().unwrap_or(""))
                .on_input(Message::SaveJobChanged)
                .size(13)
                .width(Length::Fixed(200.0)),
            checkbox(jf.quit_after_parse)
                .label("/QUIT")
                .on_toggle(Message::ToggleQuit),
            checkbox(jf.no_source_dir).label("/NOSD").on_toggle(Message::ToggleNosd),
            checkbox(jf.no_dest_dir).label("/NODD").on_toggle(Message::ToggleNodd),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        row![
            text("Extra Raw Args:").size(13),
            text_input("Additional raw switches passed verbatim", &app.extra_args_text)
                .on_input(Message::ExtraArgsChanged)
                .size(13)
                .width(Length::Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(8);

    let content = column![
        section_card("Console & Log Output Flags", flags),
        section_card("External Log File (/LOG, /LOG+, /UNILOG, /UNILOG+)", log_file),
        section_card("Robocopy .rcj Job Directives & Extra Switches", job_files),
    ]
    .spacing(10);

    column![header, scrollable(content).height(Length::Fill)]
        .spacing(10)
        .height(Length::Fill)
        .into()
}
