use crate::LaunchArgs;
use crate::win32_dialogs::{copy_to_clipboard, pick_folder, pick_open_file, pick_save_file};
use crate::win32_state::{ActionFilter, BatchStatus, Win32State, join_tokens, split_tokens};
use chrono::Utc;
use robocopy2rust_core::format::{format_bytes, format_duration};
use robocopy2rust_core::{
    CopyFlags, DirCopyFlags, FileAttrFlags, LogFileMode, RobocopyJob, RunHistoryEntry, ScheduleCadence, ScheduleSpec,
    SparseMode, ValidationSeverity, build_batch_script, build_powershell_script, build_schtasks_command,
    decode_exit_code, parse_command_line, parse_rcj, serialize_rcj,
};
use robocopy2rust_report::{ReportFormat, write_report};
use robocopy2rust_runner::{RunResult, RunnerEvent, spawn_job_runner};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use winsafe::{self as w, co, gui, msg, prelude::*};

// Menu command IDs
const IDM_IMPORT_RCJ: u16 = 4001;
const IDM_EXPORT_RCJ: u16 = 4002;
const IDM_EXPORT_BAT: u16 = 4003;
const IDM_EXPORT_PS1: u16 = 4004;
const IDM_REPORT_HTML: u16 = 4005;
const IDM_REPORT_ALL: u16 = 4006;
const IDM_EXIT: u16 = 4007;

const IDM_RESET_JOB: u16 = 4010;
const IDM_SWAP_PATHS: u16 = 4011;
const IDM_COPY_CMD: u16 = 4012;
const IDM_SAVE_PROFILE: u16 = 4013;
const IDM_QUEUE_ADD: u16 = 4014;

const IDM_DRY_RUN: u16 = 4020;
const IDM_RUN_JOB: u16 = 4021;
const IDM_STOP_JOB: u16 = 4022;

const IDM_ABOUT: u16 = 4030;

const TAB_CONSOLE_IDX: u32 = 5;

fn lbl(parent: &(impl GuiParent + 'static), text: &str, x: i32, y: i32, w_dp: i32) -> gui::Label {
    gui::Label::new(
        parent,
        gui::LabelOpts {
            text,
            position: gui::dpi(x, y),
            size: gui::dpi(w_dp, 18),
            ..Default::default()
        },
    )
}

fn edit(
    parent: &(impl GuiParent + 'static),
    x: i32,
    y: i32,
    w_dp: i32,
    h_dp: i32,
    resize: (gui::Horz, gui::Vert),
) -> gui::Edit {
    gui::Edit::new(
        parent,
        gui::EditOpts {
            position: gui::dpi(x, y),
            width: gui::dpi_x(w_dp),
            height: gui::dpi_y(h_dp),
            resize_behavior: resize,
            ..Default::default()
        },
    )
}

fn chk(parent: &(impl GuiParent + 'static), text: &str, x: i32, y: i32) -> gui::CheckBox {
    gui::CheckBox::new(
        parent,
        gui::CheckBoxOpts {
            text,
            position: gui::dpi(x, y),
            ..Default::default()
        },
    )
}

fn btn(
    parent: &(impl GuiParent + 'static),
    text: &str,
    x: i32,
    y: i32,
    w_dp: i32,
    h_dp: i32,
    resize: (gui::Horz, gui::Vert),
) -> gui::Button {
    gui::Button::new(
        parent,
        gui::ButtonOpts {
            text,
            position: gui::dpi(x, y),
            width: gui::dpi_x(w_dp),
            height: gui::dpi_y(h_dp),
            resize_behavior: resize,
            ..Default::default()
        },
    )
}

fn tab_page(parent: &(impl GuiParent + 'static)) -> gui::TabPage {
    gui::TabPage::new(
        parent,
        gui::TabPageOpts {
            class_bg_brush: gui::Brush::Color(co::COLOR::BTNFACE),
            ..Default::default()
        },
    )
}

fn move_ctrl(ctrl: &impl GuiWindow, x: i32, y: i32, w: i32, h: i32) {
    if *ctrl.hwnd() != w::HWND::NULL {
        let _ = ctrl.hwnd().SetWindowPos(
            w::HwndPlace::None,
            w::POINT::with(x, y),
            w::SIZE::with(w.max(10), h.max(10)),
            co::SWP::NOZORDER,
        );
    }
}

fn repos_ctrl(ctrl: &impl GuiWindow, x: i32, y: i32) {
    if *ctrl.hwnd() != w::HWND::NULL {
        let _ = ctrl.hwnd().SetWindowPos(
            w::HwndPlace::None,
            w::POINT::with(x, y),
            w::SIZE::default(),
            co::SWP::NOZORDER | co::SWP::NOSIZE,
        );
    }
}

#[derive(Clone)]
struct DesignerPage {
    page: gui::TabPage,
    job_name: gui::Edit,
    job_desc: gui::Edit,
    import_cmd: gui::Edit,
    btn_parse_cmd: gui::Button,
    sch_name: gui::Edit,
    sch_cadence: gui::ComboBox,
    sch_time: gui::Edit,
    sch_highest: gui::CheckBox,
    sch_preview: gui::Edit,
    btn_copy_sch: gui::Button,
    diag_list: gui::ListView,
}

impl DesignerPage {
    fn new(parent: &(impl GuiParent + 'static)) -> Self {
        let page = tab_page(parent);

        lbl(&page, "Job Name:", 10, 12, 70);
        let job_name = edit(&page, 85, 9, 260, 22, (gui::Horz::None, gui::Vert::None));
        lbl(&page, "Notes:", 360, 12, 48);
        let job_desc = edit(&page, 412, 9, 500, 22, (gui::Horz::None, gui::Vert::None));

        lbl(&page, "Import CLI:", 10, 42, 70);
        let import_cmd = edit(&page, 85, 39, 710, 22, (gui::Horz::None, gui::Vert::None));
        let btn_parse_cmd = btn(
            &page,
            "Parse && Load",
            805,
            38,
            108,
            24,
            (gui::Horz::None, gui::Vert::None),
        );

        lbl(&page, "Task Name:", 10, 74, 70);
        let sch_name = edit(&page, 85, 71, 200, 22, (gui::Horz::None, gui::Vert::None));
        lbl(&page, "Cadence:", 295, 74, 58);
        let sch_cadence = gui::ComboBox::new(
            &page,
            gui::ComboBoxOpts {
                position: gui::dpi(356, 71),
                width: gui::dpi_x(105),
                items: &["DAILY", "WEEKLY", "HOURLY", "ONSTART", "ONLOGON"],
                selected_item: Some(0),
                ..Default::default()
            },
        );
        lbl(&page, "Time:", 472, 74, 38);
        let sch_time = edit(&page, 512, 71, 65, 22, (gui::Horz::None, gui::Vert::None));
        let sch_highest = chk(&page, "/RL HIGHEST", 588, 73);
        let btn_copy_sch = btn(
            &page,
            "Copy schtasks",
            805,
            70,
            108,
            24,
            (gui::Horz::None, gui::Vert::None),
        );

        lbl(&page, "schtasks:", 10, 104, 70);
        let sch_preview = gui::Edit::new(
            &page,
            gui::EditOpts {
                position: gui::dpi(85, 101),
                width: gui::dpi_x(828),
                height: gui::dpi_y(22),
                control_style: co::ES::AUTOHSCROLL | co::ES::READONLY,
                resize_behavior: (gui::Horz::None, gui::Vert::None),
                ..Default::default()
            },
        );

        lbl(&page, "Real-Time Validation Diagnostics:", 10, 134, 240);
        let diag_list = gui::ListView::new(
            &page,
            gui::ListViewOpts {
                position: gui::dpi(10, 156),
                size: gui::dpi(903, 330),
                columns: &[
                    ("Severity", gui::dpi_x(85)),
                    ("Code", gui::dpi_x(175)),
                    ("Message", gui::dpi_x(370)),
                    ("Suggestion", gui::dpi_x(265)),
                ],
                resize_behavior: (gui::Horz::None, gui::Vert::None),
                ..Default::default()
            },
        );

        {
            let job_desc = job_desc.clone();
            let import_cmd = import_cmd.clone();
            let btn_parse_cmd = btn_parse_cmd.clone();
            let btn_copy_sch = btn_copy_sch.clone();
            let sch_preview = sch_preview.clone();
            let diag_list = diag_list.clone();
            page.on().wm_size(move |p| {
                if p.request == co::SIZE_R::MINIMIZED || p.client_area.cx <= 0 || p.client_area.cy <= 0 {
                    return Ok(());
                }
                let w = p.client_area.cx;
                let h = p.client_area.cy;
                let pad = gui::dpi_x(10);
                let gap = gui::dpi_x(8);
                let bw = gui::dpi_x(108);
                let bx = (w - pad - bw).max(gui::dpi_x(400));

                move_ctrl(
                    &job_desc,
                    gui::dpi_x(412),
                    gui::dpi_y(9),
                    (w - gui::dpi_x(412) - pad).max(gui::dpi_x(120)),
                    gui::dpi_y(22),
                );
                move_ctrl(
                    &import_cmd,
                    gui::dpi_x(85),
                    gui::dpi_y(39),
                    (bx - gap - gui::dpi_x(85)).max(gui::dpi_x(120)),
                    gui::dpi_y(22),
                );
                move_ctrl(&btn_parse_cmd, bx, gui::dpi_y(38), bw, gui::dpi_y(24));
                move_ctrl(&btn_copy_sch, bx, gui::dpi_y(70), bw, gui::dpi_y(24));
                move_ctrl(
                    &sch_preview,
                    gui::dpi_x(85),
                    gui::dpi_y(101),
                    (w - gui::dpi_x(85) - pad).max(gui::dpi_x(120)),
                    gui::dpi_y(22),
                );
                move_ctrl(
                    &diag_list,
                    pad,
                    gui::dpi_y(156),
                    (w - 2 * pad).max(gui::dpi_x(200)),
                    (h - gui::dpi_y(156) - gui::dpi_y(10)).max(gui::dpi_y(80)),
                );
                Ok(())
            });
        }

        Self {
            page,
            job_name,
            job_desc,
            import_cmd,
            btn_parse_cmd,
            sch_name,
            sch_cadence,
            sch_time,
            sch_highest,
            sch_preview,
            btn_copy_sch,
            diag_list,
        }
    }
}

#[derive(Clone)]
struct CopyPage {
    page: gui::TabPage,
    s_non_empty: gui::CheckBox,
    e_incl_empty: gui::CheckBox,
    mir: gui::CheckBox,
    purge: gui::CheckBox,
    mov_files: gui::CheckBox,
    move_all: gui::CheckBox,
    lev_edit: gui::Edit,
    mt_edit: gui::Edit,
    z_restart: gui::CheckBox,
    b_backup: gui::CheckBox,
    zb_both: gui::CheckBox,
    j_unbuf: gui::CheckBox,
    efsraw: gui::CheckBox,
    compress: gui::CheckBox,
    no_offload: gui::CheckBox,
    no_clone: gui::CheckBox,
    sparse_combo: gui::ComboBox,
    copy_d: gui::CheckBox,
    copy_a: gui::CheckBox,
    copy_t: gui::CheckBox,
    copy_s: gui::CheckBox,
    copy_o: gui::CheckBox,
    copy_u: gui::CheckBox,
    copy_x: gui::CheckBox,
    no_copy: gui::CheckBox,
    btn_copy_dat: gui::Button,
    btn_copy_sec: gui::Button,
    btn_copy_all: gui::Button,
    dcopy_d: gui::CheckBox,
    dcopy_a: gui::CheckBox,
    dcopy_t: gui::CheckBox,
    dcopy_e: gui::CheckBox,
    dcopy_x: gui::CheckBox,
    no_dcopy: gui::CheckBox,
    secfix: gui::CheckBox,
    timfix: gui::CheckBox,
    create_tree: gui::CheckBox,
    fat_names: gui::CheckBox,
    no_256: gui::CheckBox,
    copy_sj: gui::CheckBox,
    copy_sl: gui::CheckBox,
    add_attrs: gui::Edit,
    rem_attrs: gui::Edit,
}

impl CopyPage {
    fn new(parent: &(impl GuiParent + 'static)) -> Self {
        let page = tab_page(parent);

        lbl(&page, "Recursion && Sync:", 10, 12, 130);
        let s_non_empty = chk(&page, "/S (Subdirs non-empty)", 145, 10);
        let e_incl_empty = chk(&page, "/E (Subdirs + empty)", 320, 10);
        let purge = chk(&page, "/PURGE (Delete dest extras)", 485, 10);
        let mir = chk(&page, "/MIR (Mirror = /E + /PURGE)", 685, 10);

        let mov_files = chk(&page, "/MOV (Move files)", 145, 38);
        let move_all = chk(&page, "/MOVE (Move files + dirs)", 320, 38);
        lbl(&page, "/LEV:n:", 510, 40, 50);
        let lev_edit = edit(&page, 562, 37, 65, 22, (gui::Horz::None, gui::Vert::None));
        lbl(&page, "/MT:n (1-128):", 645, 40, 88);
        let mt_edit = edit(&page, 738, 37, 65, 22, (gui::Horz::None, gui::Vert::None));

        lbl(&page, "Transfer && Engine:", 10, 74, 130);
        let z_restart = chk(&page, "/Z (Restartable)", 145, 72);
        let b_backup = chk(&page, "/B (Backup)", 280, 72);
        let zb_both = chk(&page, "/ZB (Restart/Backup)", 395, 72);
        let j_unbuf = chk(&page, "/J (Unbuffered I/O)", 565, 72);
        let efsraw = chk(&page, "/EFSRAW", 720, 72);

        let compress = chk(&page, "/COMPRESS (SMB)", 145, 100);
        let no_offload = chk(&page, "/NOOFFLOAD", 280, 100);
        let no_clone = chk(&page, "/NOCLONE", 395, 100);
        lbl(&page, "/SPARSE:", 510, 102, 58);
        let sparse_combo = gui::ComboBox::new(
            &page,
            gui::ComboBoxOpts {
                position: gui::dpi(572, 99),
                width: gui::dpi_x(130),
                items: &["Default", "/SPARSE:Y", "/SPARSE:N"],
                selected_item: Some(0),
                ..Default::default()
            },
        );

        lbl(&page, "File /COPY Flags:", 10, 140, 130);
        let copy_d = chk(&page, "D (Data)", 145, 138);
        let copy_a = chk(&page, "A (Attr)", 225, 138);
        let copy_t = chk(&page, "T (Times)", 300, 138);
        let copy_s = chk(&page, "S (ACL)", 385, 138);
        let copy_o = chk(&page, "O (Owner)", 465, 138);
        let copy_u = chk(&page, "U (Audit)", 555, 138);
        let copy_x = chk(&page, "X (No ADS)", 640, 138);
        let no_copy = chk(&page, "/NOCOPY", 740, 138);

        let btn_copy_dat = btn(
            &page,
            "DAT Default",
            145,
            166,
            95,
            24,
            (gui::Horz::None, gui::Vert::None),
        );
        let btn_copy_sec = btn(
            &page,
            "/SEC (DATS)",
            248,
            166,
            95,
            24,
            (gui::Horz::None, gui::Vert::None),
        );
        let btn_copy_all = btn(&page, "/COPYALL", 351, 166, 95, 24, (gui::Horz::None, gui::Vert::None));
        let secfix = chk(&page, "/SECFIX", 480, 169);
        let timfix = chk(&page, "/TIMFIX", 575, 169);

        lbl(&page, "Dir /DCOPY Flags:", 10, 206, 130);
        let dcopy_d = chk(&page, "D (Data)", 145, 204);
        let dcopy_a = chk(&page, "A (Attr)", 225, 204);
        let dcopy_t = chk(&page, "T (Times)", 300, 204);
        let dcopy_e = chk(&page, "E (EAs)", 385, 204);
        let dcopy_x = chk(&page, "X (No ADS)", 465, 204);
        let no_dcopy = chk(&page, "/NODCOPY", 565, 204);

        lbl(&page, "Tree && Links:", 10, 244, 130);
        let create_tree = chk(&page, "/CREATE (0-byte tree)", 145, 242);
        let fat_names = chk(&page, "/FAT (8.3 names)", 315, 242);
        let no_256 = chk(&page, "/256 (No long paths)", 465, 242);
        let copy_sj = chk(&page, "/SJ (Copy junctions)", 635, 242);
        let copy_sl = chk(&page, "/SL (Copy symlinks)", 790, 242);

        lbl(&page, "/A+: (Add Attrs):", 10, 282, 130);
        let add_attrs = edit(&page, 145, 279, 140, 22, (gui::Horz::None, gui::Vert::None));
        lbl(&page, "/A-: (Remove Attrs):", 310, 282, 135);
        let rem_attrs = edit(&page, 450, 279, 140, 22, (gui::Horz::None, gui::Vert::None));
        lbl(&page, "Valid attr chars: R A S H C N E T O", 605, 282, 240);

        Self {
            page,
            s_non_empty,
            e_incl_empty,
            mir,
            purge,
            mov_files,
            move_all,
            lev_edit,
            mt_edit,
            z_restart,
            b_backup,
            zb_both,
            j_unbuf,
            efsraw,
            compress,
            no_offload,
            no_clone,
            sparse_combo,
            copy_d,
            copy_a,
            copy_t,
            copy_s,
            copy_o,
            copy_u,
            copy_x,
            no_copy,
            btn_copy_dat,
            btn_copy_sec,
            btn_copy_all,
            dcopy_d,
            dcopy_a,
            dcopy_t,
            dcopy_e,
            dcopy_x,
            no_dcopy,
            secfix,
            timfix,
            create_tree,
            fat_names,
            no_256,
            copy_sj,
            copy_sl,
            add_attrs,
            rem_attrs,
        }
    }
}

#[derive(Clone)]
struct SelectionPage {
    page: gui::TabPage,
    xf_edit: gui::Edit,
    xd_edit: gui::Edit,
    xj: gui::CheckBox,
    xjd: gui::CheckBox,
    xjf: gui::CheckBox,
    xc: gui::CheckBox,
    xn: gui::CheckBox,
    xo: gui::CheckBox,
    xx: gui::CheckBox,
    xl: gui::CheckBox,
    is_same: gui::CheckBox,
    it_tweaked: gui::CheckBox,
    im_modified: gui::CheckBox,
    fft: gui::CheckBox,
    dst: gui::CheckBox,
    a_arch: gui::CheckBox,
    m_reset: gui::CheckBox,
    ia_edit: gui::Edit,
    xa_edit: gui::Edit,
    max_sz: gui::Edit,
    min_sz: gui::Edit,
    max_age: gui::Edit,
    min_age: gui::Edit,
    max_lad: gui::Edit,
    min_lad: gui::Edit,
}

impl SelectionPage {
    fn new(parent: &(impl GuiParent + 'static)) -> Self {
        let page = tab_page(parent);

        lbl(&page, "/XF Exclude Files:", 10, 12, 130);
        let xf_edit = edit(&page, 145, 9, 760, 22, (gui::Horz::None, gui::Vert::None));
        lbl(&page, "/XD Exclude Dirs:", 10, 42, 130);
        let xd_edit = edit(&page, 145, 39, 760, 22, (gui::Horz::None, gui::Vert::None));

        lbl(&page, "Reparse Points:", 10, 74, 130);
        let xj = chk(&page, "/XJ (Exclude all junctions && symlinks)", 145, 72);
        let xjd = chk(&page, "/XJD (Exclude dir junctions)", 415, 72);
        let xjf = chk(&page, "/XJF (Exclude file symlinks)", 635, 72);

        lbl(&page, "Compare Classes:", 10, 108, 130);
        let xc = chk(&page, "/XC (Excl Changed)", 145, 106);
        let xn = chk(&page, "/XN (Excl Newer)", 295, 106);
        let xo = chk(&page, "/XO (Excl Older)", 440, 106);
        let xx = chk(&page, "/XX (Excl Extra)", 585, 106);
        let xl = chk(&page, "/XL (Excl Lonely)", 720, 106);

        let is_same = chk(&page, "/IS (Incl Same)", 145, 134);
        let it_tweaked = chk(&page, "/IT (Incl Tweaked)", 295, 134);
        let im_modified = chk(&page, "/IM (Incl Modified)", 440, 134);
        let fft = chk(&page, "/FFT (2s FAT time)", 585, 134);
        let dst = chk(&page, "/DST (1h DST)", 720, 134);

        lbl(&page, "Attributes:", 10, 172, 130);
        let a_arch = chk(&page, "/A (Archive bit)", 145, 170);
        let m_reset = chk(&page, "/M (Archive + reset)", 295, 170);
        lbl(&page, "/IA:", 455, 172, 32);
        let ia_edit = edit(&page, 490, 169, 120, 22, (gui::Horz::None, gui::Vert::None));
        lbl(&page, "/XA:", 625, 172, 32);
        let xa_edit = edit(&page, 660, 169, 120, 22, (gui::Horz::None, gui::Vert::None));

        lbl(&page, "/MAX:n (bytes):", 10, 212, 130);
        let max_sz = edit(&page, 145, 209, 130, 22, (gui::Horz::None, gui::Vert::None));
        lbl(&page, "/MIN:n (bytes):", 300, 212, 110);
        let min_sz = edit(&page, 415, 209, 130, 22, (gui::Horz::None, gui::Vert::None));

        lbl(&page, "/MAXAGE (days/date):", 10, 244, 130);
        let max_age = edit(&page, 145, 241, 130, 22, (gui::Horz::None, gui::Vert::None));
        lbl(&page, "/MINAGE:", 300, 244, 110);
        let min_age = edit(&page, 415, 241, 130, 22, (gui::Horz::None, gui::Vert::None));

        lbl(&page, "/MAXLAD:", 10, 276, 130);
        let max_lad = edit(&page, 145, 273, 130, 22, (gui::Horz::None, gui::Vert::None));
        lbl(&page, "/MINLAD:", 300, 276, 110);
        let min_lad = edit(&page, 415, 273, 130, 22, (gui::Horz::None, gui::Vert::None));

        {
            let xf_edit = xf_edit.clone();
            let xd_edit = xd_edit.clone();
            page.on().wm_size(move |p| {
                if p.request == co::SIZE_R::MINIMIZED || p.client_area.cx <= 0 || p.client_area.cy <= 0 {
                    return Ok(());
                }
                let w = p.client_area.cx;
                let pad = gui::dpi_x(10);
                let ew = (w - gui::dpi_x(145) - pad).max(gui::dpi_x(120));
                move_ctrl(&xf_edit, gui::dpi_x(145), gui::dpi_y(9), ew, gui::dpi_y(22));
                move_ctrl(&xd_edit, gui::dpi_x(145), gui::dpi_y(39), ew, gui::dpi_y(22));
                Ok(())
            });
        }

        Self {
            page,
            xf_edit,
            xd_edit,
            xj,
            xjd,
            xjf,
            xc,
            xn,
            xo,
            xx,
            xl,
            is_same,
            it_tweaked,
            im_modified,
            fft,
            dst,
            a_arch,
            m_reset,
            ia_edit,
            xa_edit,
            max_sz,
            min_sz,
            max_age,
            min_age,
            max_lad,
            min_lad,
        }
    }
}

#[derive(Clone)]
struct PerfRetryPage {
    page: gui::TabPage,
    r_edit: gui::Edit,
    w_edit: gui::Edit,
    tbd: gui::CheckBox,
    reg: gui::CheckBox,
    lfsm: gui::CheckBox,
    lfsm_floor: gui::Edit,
    io_max_size: gui::Edit,
    io_rate: gui::Edit,
    threshold: gui::Edit,
    ipg_ms: gui::Edit,
    mon_n: gui::Edit,
    mot_m: gui::Edit,
    rh_window: gui::Edit,
    pf_per_file: gui::CheckBox,
}

impl PerfRetryPage {
    fn new(parent: &(impl GuiParent + 'static)) -> Self {
        let page = tab_page(parent);

        lbl(&page, "/R:n (Retries):", 10, 12, 135);
        let r_edit = edit(&page, 150, 9, 100, 22, (gui::Horz::None, gui::Vert::None));
        lbl(&page, "/W:n (Wait secs):", 275, 12, 115);
        let w_edit = edit(&page, 395, 9, 100, 22, (gui::Horz::None, gui::Vert::None));
        let tbd = chk(&page, "/TBD (Wait for sharenames)", 520, 11);
        let reg = chk(&page, "/REG (Save /R && /W)", 715, 11);

        let lfsm = chk(&page, "/LFSM (Low Free Space Mode)", 150, 44);
        lbl(&page, "/LFSM:n[KMG] Floor:", 370, 46, 130);
        let lfsm_floor = edit(&page, 505, 43, 120, 22, (gui::Horz::None, gui::Vert::None));

        lbl(&page, "/IoMaxSize:n[KMG]:", 10, 88, 135);
        let io_max_size = edit(&page, 150, 85, 110, 22, (gui::Horz::None, gui::Vert::None));
        lbl(&page, "/IoRate:n[KMG]:", 275, 88, 115);
        let io_rate = edit(&page, 395, 85, 110, 22, (gui::Horz::None, gui::Vert::None));
        lbl(&page, "/Threshold:n[KMG]:", 520, 88, 130);
        let threshold = edit(&page, 655, 85, 110, 22, (gui::Horz::None, gui::Vert::None));

        lbl(&page, "/IPG:n (Gap ms):", 10, 122, 135);
        let ipg_ms = edit(&page, 150, 119, 110, 22, (gui::Horz::None, gui::Vert::None));

        lbl(&page, "/MON:n (Changes):", 10, 162, 135);
        let mon_n = edit(&page, 150, 159, 110, 22, (gui::Horz::None, gui::Vert::None));
        lbl(&page, "/MOT:m (Minutes):", 275, 162, 115);
        let mot_m = edit(&page, 395, 159, 110, 22, (gui::Horz::None, gui::Vert::None));

        lbl(&page, "/RH:hhmm-hhmm:", 10, 196, 135);
        let rh_window = edit(&page, 150, 193, 130, 22, (gui::Horz::None, gui::Vert::None));
        let pf_per_file = chk(&page, "/PF (Check run hours per file)", 295, 195);

        Self {
            page,
            r_edit,
            w_edit,
            tbd,
            reg,
            lfsm,
            lfsm_floor,
            io_max_size,
            io_rate,
            threshold,
            ipg_ms,
            mon_n,
            mot_m,
            rh_window,
            pf_per_file,
        }
    }
}

#[derive(Clone)]
struct LoggingPage {
    page: gui::TabPage,
    l_list_only: gui::CheckBox,
    x_extra: gui::CheckBox,
    v_verbose: gui::CheckBox,
    ts_stamps: gui::CheckBox,
    fp_full: gui::CheckBox,
    bytes_flag: gui::CheckBox,
    unicode_out: gui::CheckBox,
    ns_nosize: gui::CheckBox,
    nc_noclass: gui::CheckBox,
    nfl_nofile: gui::CheckBox,
    ndl_nodir: gui::CheckBox,
    np_noprog: gui::CheckBox,
    eta_flag: gui::CheckBox,
    njh_flag: gui::CheckBox,
    njs_flag: gui::CheckBox,
    tee_flag: gui::CheckBox,
    log_mode: gui::ComboBox,
    log_path: gui::Edit,
    btn_log_browse: gui::Button,
    job_load: gui::Edit,
    job_save: gui::Edit,
    quit_flag: gui::CheckBox,
    nosd_flag: gui::CheckBox,
    nodd_flag: gui::CheckBox,
    extra_args: gui::Edit,
}

impl LoggingPage {
    fn new(parent: &(impl GuiParent + 'static)) -> Self {
        let page = tab_page(parent);

        let l_list_only = chk(&page, "/L (List only - Dry Run)", 10, 10);
        let x_extra = chk(&page, "/X (Report extras)", 195, 10);
        let v_verbose = chk(&page, "/V (Verbose)", 340, 10);
        let ts_stamps = chk(&page, "/TS (Timestamps)", 460, 10);
        let fp_full = chk(&page, "/FP (Full paths)", 595, 10);
        let bytes_flag = chk(&page, "/BYTES", 720, 10);
        let unicode_out = chk(&page, "/UNICODE", 805, 10);

        let ns_nosize = chk(&page, "/NS (No sizes)", 10, 38);
        let nc_noclass = chk(&page, "/NC (No classes)", 140, 38);
        let nfl_nofile = chk(&page, "/NFL (No files)", 275, 38);
        let ndl_nodir = chk(&page, "/NDL (No dirs)", 400, 38);
        let np_noprog = chk(&page, "/NP (No %)", 520, 38);
        let eta_flag = chk(&page, "/ETA", 625, 38);
        let njh_flag = chk(&page, "/NJH", 695, 38);
        let njs_flag = chk(&page, "/NJS", 760, 38);
        let tee_flag = chk(&page, "/TEE", 825, 38);

        lbl(&page, "Log Mode:", 10, 76, 75);
        let log_mode = gui::ComboBox::new(
            &page,
            gui::ComboBoxOpts {
                position: gui::dpi(90, 73),
                width: gui::dpi_x(135),
                items: &["None", "/LOG:", "/LOG+:", "/UNILOG:", "/UNILOG+:"],
                selected_item: Some(0),
                ..Default::default()
            },
        );
        lbl(&page, "Log File:", 238, 76, 62);
        let log_path = edit(&page, 305, 73, 505, 22, (gui::Horz::None, gui::Vert::None));
        let btn_log_browse = btn(&page, "Browse...", 820, 72, 85, 24, (gui::Horz::None, gui::Vert::None));

        lbl(&page, "/JOB:name:", 10, 114, 75);
        let job_load = edit(&page, 90, 111, 240, 22, (gui::Horz::None, gui::Vert::None));
        lbl(&page, "/SAVE:name:", 350, 114, 85);
        let job_save = edit(&page, 440, 111, 240, 22, (gui::Horz::None, gui::Vert::None));

        let quit_flag = chk(&page, "/QUIT", 90, 142);
        let nosd_flag = chk(&page, "/NOSD", 165, 142);
        let nodd_flag = chk(&page, "/NODD", 245, 142);

        lbl(&page, "Extra Raw Args:", 10, 180, 110);
        let extra_args = edit(&page, 125, 177, 780, 22, (gui::Horz::None, gui::Vert::None));

        {
            let log_path = log_path.clone();
            let btn_log_browse = btn_log_browse.clone();
            let extra_args = extra_args.clone();
            page.on().wm_size(move |p| {
                if p.request == co::SIZE_R::MINIMIZED || p.client_area.cx <= 0 || p.client_area.cy <= 0 {
                    return Ok(());
                }
                let w = p.client_area.cx;
                let pad = gui::dpi_x(10);
                let gap = gui::dpi_x(8);
                let bw = gui::dpi_x(85);
                let bx = (w - pad - bw).max(gui::dpi_x(450));
                move_ctrl(
                    &log_path,
                    gui::dpi_x(305),
                    gui::dpi_y(73),
                    (bx - gap - gui::dpi_x(305)).max(gui::dpi_x(120)),
                    gui::dpi_y(22),
                );
                move_ctrl(&btn_log_browse, bx, gui::dpi_y(72), bw, gui::dpi_y(24));
                move_ctrl(
                    &extra_args,
                    gui::dpi_x(125),
                    gui::dpi_y(177),
                    (w - gui::dpi_x(125) - pad).max(gui::dpi_x(120)),
                    gui::dpi_y(22),
                );
                Ok(())
            });
        }

        Self {
            page,
            l_list_only,
            x_extra,
            v_verbose,
            ts_stamps,
            fp_full,
            bytes_flag,
            unicode_out,
            ns_nosize,
            nc_noclass,
            nfl_nofile,
            ndl_nodir,
            np_noprog,
            eta_flag,
            njh_flag,
            njs_flag,
            tee_flag,
            log_mode,
            log_path,
            btn_log_browse,
            job_load,
            job_save,
            quit_flag,
            nosd_flag,
            nodd_flag,
            extra_args,
        }
    }
}

#[derive(Clone)]
struct ConsolePage {
    page: gui::TabPage,
    progress: gui::ProgressBar,
    summary_lbl: gui::Label,
    filter_combo: gui::ComboBox,
    btn_copy_log: gui::Button,
    btn_export_html: gui::Button,
    btn_export_all: gui::Button,
    actions_list: gui::ListView,
    raw_log_edit: gui::Edit,
}

impl ConsolePage {
    fn new(parent: &(impl GuiParent + 'static)) -> Self {
        let page = tab_page(parent);

        let progress = gui::ProgressBar::new(
            &page,
            gui::ProgressBarOpts {
                position: gui::dpi(10, 10),
                size: gui::dpi(210, 22),
                ..Default::default()
            },
        );
        let summary_lbl = lbl(
            &page,
            "Idle — run Dry-Run (/L) or Execute Job to see live output.",
            230,
            12,
            410,
        );
        let filter_combo = gui::ComboBox::new(
            &page,
            gui::ComboBoxOpts {
                position: gui::dpi(645, 9),
                width: gui::dpi_x(105),
                items: &["All", "Copied/New", "Extras", "Skipped", "Errors"],
                selected_item: Some(0),
                resize_behavior: (gui::Horz::None, gui::Vert::None),
                ..Default::default()
            },
        );
        let btn_copy_log = btn(&page, "Copy Log", 756, 8, 72, 24, (gui::Horz::None, gui::Vert::None));
        let btn_export_html = btn(&page, "HTML...", 832, 8, 62, 24, (gui::Horz::None, gui::Vert::None));
        let btn_export_all = btn(&page, "All...", 898, 8, 52, 24, (gui::Horz::None, gui::Vert::None));

        let actions_list = gui::ListView::new(
            &page,
            gui::ListViewOpts {
                position: gui::dpi(10, 40),
                size: gui::dpi(940, 220),
                columns: &[
                    ("Class", gui::dpi_x(100)),
                    ("Size", gui::dpi_x(95)),
                    ("Timestamp", gui::dpi_x(145)),
                    ("Path", gui::dpi_x(585)),
                ],
                resize_behavior: (gui::Horz::None, gui::Vert::None),
                ..Default::default()
            },
        );

        let raw_log_edit = gui::Edit::new(
            &page,
            gui::EditOpts {
                position: gui::dpi(10, 268),
                width: gui::dpi_x(940),
                height: gui::dpi_y(215),
                control_style: co::ES::MULTILINE | co::ES::READONLY | co::ES::AUTOVSCROLL | co::ES::AUTOHSCROLL,
                window_style: co::WS::CHILD | co::WS::TABSTOP | co::WS::VISIBLE | co::WS::VSCROLL | co::WS::HSCROLL,
                resize_behavior: (gui::Horz::None, gui::Vert::None),
                ..Default::default()
            },
        );

        {
            let summary_lbl = summary_lbl.clone();
            let filter_combo = filter_combo.clone();
            let btn_copy_log = btn_copy_log.clone();
            let btn_export_html = btn_export_html.clone();
            let btn_export_all = btn_export_all.clone();
            let actions_list = actions_list.clone();
            let raw_log_edit = raw_log_edit.clone();
            page.on().wm_size(move |p| {
                if p.request == co::SIZE_R::MINIMIZED || p.client_area.cx <= 0 || p.client_area.cy <= 0 {
                    return Ok(());
                }
                let w = p.client_area.cx;
                let h = p.client_area.cy;
                let pad = gui::dpi_x(10);
                let gap = gui::dpi_x(8);
                let sp = gui::dpi_x(4);
                let w_all = gui::dpi_x(54);
                let x_all = (w - pad - w_all).max(gui::dpi_x(720));
                let w_html = gui::dpi_x(64);
                let x_html = x_all - sp - w_html;
                let w_copy = gui::dpi_x(74);
                let x_copy = x_html - sp - w_copy;
                let w_flt = gui::dpi_x(105);
                let x_flt = x_copy - gui::dpi_x(6) - w_flt;

                move_ctrl(&btn_export_all, x_all, gui::dpi_y(8), w_all, gui::dpi_y(24));
                move_ctrl(&btn_export_html, x_html, gui::dpi_y(8), w_html, gui::dpi_y(24));
                move_ctrl(&btn_copy_log, x_copy, gui::dpi_y(8), w_copy, gui::dpi_y(24));
                repos_ctrl(&filter_combo, x_flt, gui::dpi_y(9));
                move_ctrl(
                    &summary_lbl,
                    gui::dpi_x(230),
                    gui::dpi_y(12),
                    (x_flt - gap - gui::dpi_x(230)).max(gui::dpi_x(100)),
                    gui::dpi_y(18),
                );

                let top_y = gui::dpi_y(40);
                let avail_h = (h - top_y - gui::dpi_y(18)).max(gui::dpi_y(160));
                let act_h = (avail_h / 2).max(gui::dpi_y(120));
                let full_w = (w - 2 * pad).max(gui::dpi_x(200));
                move_ctrl(&actions_list, pad, top_y, full_w, act_h);
                let raw_y = top_y + act_h + gui::dpi_y(8);
                move_ctrl(
                    &raw_log_edit,
                    pad,
                    raw_y,
                    full_w,
                    (h - raw_y - gui::dpi_y(10)).max(gui::dpi_y(80)),
                );
                Ok(())
            });
        }

        Self {
            page,
            progress,
            summary_lbl,
            filter_combo,
            btn_copy_log,
            btn_export_html,
            btn_export_all,
            actions_list,
            raw_log_edit,
        }
    }
}

#[derive(Clone)]
struct BatchHistoryPage {
    page: gui::TabPage,
    btn_add: gui::Button,
    btn_dry_queue: gui::Button,
    btn_run_queue: gui::Button,
    btn_remove: gui::Button,
    btn_clear_queue: gui::Button,
    chk_stop_err: gui::CheckBox,
    batch_list: gui::ListView,
    btn_reload_hist: gui::Button,
    btn_clear_hist: gui::Button,
    hist_list: gui::ListView,
    code_edit: gui::Edit,
    btn_decode: gui::Button,
    decode_lbl: gui::Label,
}

impl BatchHistoryPage {
    fn new(parent: &(impl GuiParent + 'static)) -> Self {
        let page = tab_page(parent);

        let btn_add = btn(&page, "Add Current", 10, 8, 95, 24, (gui::Horz::None, gui::Vert::None));
        let btn_dry_queue = btn(
            &page,
            "Dry-Run Queue",
            110,
            8,
            105,
            24,
            (gui::Horz::None, gui::Vert::None),
        );
        let btn_run_queue = btn(&page, "Run Queue", 220, 8, 90, 24, (gui::Horz::None, gui::Vert::None));
        let btn_remove = btn(&page, "Remove Sel", 315, 8, 90, 24, (gui::Horz::None, gui::Vert::None));
        let btn_clear_queue = btn(&page, "Clear Queue", 410, 8, 90, 24, (gui::Horz::None, gui::Vert::None));
        let chk_stop_err = chk(&page, "Stop on Error (exit >= 8)", 515, 11);

        let batch_list = gui::ListView::new(
            &page,
            gui::ListViewOpts {
                position: gui::dpi(10, 38),
                size: gui::dpi(940, 170),
                columns: &[
                    ("#", gui::dpi_x(40)),
                    ("Job Name", gui::dpi_x(170)),
                    ("Source", gui::dpi_x(240)),
                    ("Destination", gui::dpi_x(240)),
                    ("Status", gui::dpi_x(105)),
                    ("Notes", gui::dpi_x(135)),
                ],
                resize_behavior: (gui::Horz::None, gui::Vert::None),
                ..Default::default()
            },
        );

        lbl(&page, "Run History:", 10, 218, 90);
        let btn_reload_hist = btn(
            &page,
            "Reload Command",
            105,
            214,
            115,
            24,
            (gui::Horz::None, gui::Vert::None),
        );
        let btn_clear_hist = btn(
            &page,
            "Clear History",
            226,
            214,
            95,
            24,
            (gui::Horz::None, gui::Vert::None),
        );

        lbl(&page, "Exit Code (0-16):", 355, 218, 105);
        let code_edit = edit(&page, 462, 215, 45, 22, (gui::Horz::None, gui::Vert::None));
        let btn_decode = btn(&page, "Decode", 512, 214, 65, 24, (gui::Horz::None, gui::Vert::None));
        let decode_lbl = lbl(&page, "1 -> Files Copied (success)", 585, 218, 360);

        let hist_list = gui::ListView::new(
            &page,
            gui::ListViewOpts {
                position: gui::dpi(10, 244),
                size: gui::dpi(940, 240),
                columns: &[
                    ("Timestamp", gui::dpi_x(155)),
                    ("Job", gui::dpi_x(150)),
                    ("Mode", gui::dpi_x(70)),
                    ("Exit", gui::dpi_x(120)),
                    ("Files / Fails", gui::dpi_x(95)),
                    ("Paths", gui::dpi_x(340)),
                ],
                resize_behavior: (gui::Horz::None, gui::Vert::None),
                ..Default::default()
            },
        );

        {
            let batch_list = batch_list.clone();
            let decode_lbl = decode_lbl.clone();
            let hist_list = hist_list.clone();
            page.on().wm_size(move |p| {
                if p.request == co::SIZE_R::MINIMIZED || p.client_area.cx <= 0 || p.client_area.cy <= 0 {
                    return Ok(());
                }
                let w = p.client_area.cx;
                let h = p.client_area.cy;
                let pad = gui::dpi_x(10);
                let full_w = (w - 2 * pad).max(gui::dpi_x(200));
                move_ctrl(&batch_list, pad, gui::dpi_y(38), full_w, gui::dpi_y(170));
                move_ctrl(
                    &decode_lbl,
                    gui::dpi_x(585),
                    gui::dpi_y(218),
                    (w - gui::dpi_x(585) - pad).max(gui::dpi_x(120)),
                    gui::dpi_y(18),
                );
                move_ctrl(
                    &hist_list,
                    pad,
                    gui::dpi_y(244),
                    full_w,
                    (h - gui::dpi_y(244) - gui::dpi_y(10)).max(gui::dpi_y(80)),
                );
                Ok(())
            });
        }

        Self {
            page,
            btn_add,
            btn_dry_queue,
            btn_run_queue,
            btn_remove,
            btn_clear_queue,
            chk_stop_err,
            batch_list,
            btn_reload_hist,
            btn_clear_hist,
            hist_list,
            code_edit,
            btn_decode,
            decode_lbl,
        }
    }
}

#[derive(Clone)]
struct Win32App {
    wnd: gui::WindowMain,
    src_edit: gui::Edit,
    btn_src_browse: gui::Button,
    btn_swap: gui::Button,
    dst_edit: gui::Edit,
    btn_dst_browse: gui::Button,
    files_edit: gui::Edit,

    preset_combo: gui::ComboBox,
    btn_apply_preset: gui::Button,
    profile_combo: gui::ComboBox,
    btn_load_prof: gui::Button,
    btn_save_prof: gui::Button,
    btn_del_prof: gui::Button,
    btn_dry_run: gui::Button,
    btn_run_job: gui::Button,
    btn_queue_add: gui::Button,
    btn_stop_job: gui::Button,

    tab: gui::Tab,
    tab_pages: Vec<gui::TabPage>,
    designer: DesignerPage,
    copy_pg: CopyPage,
    sel_pg: SelectionPage,
    perf_pg: PerfRetryPage,
    log_pg: LoggingPage,
    console_pg: ConsolePage,
    batch_pg: BatchHistoryPage,

    cmd_edit: gui::Edit,
    btn_copy_cmd: gui::Button,
    btn_exp_rcj: gui::Button,
    btn_exp_bat: gui::Button,
    btn_exp_ps1: gui::Button,
    status_bar: gui::StatusBar,

    state: Arc<Mutex<Win32State>>,
    suppress_sync: Arc<AtomicBool>,
}

fn build_main_menu() -> w::SysResult<w::HMENU> {
    let menu = w::HMENU::CreateMenu()?;
    let file_m = w::HMENU::CreatePopupMenu()?;
    file_m.append_item(&[
        w::MenuItem::Entry {
            cmd_id: IDM_IMPORT_RCJ,
            text: "Import .rcj Job File...",
        },
        w::MenuItem::Entry {
            cmd_id: IDM_EXPORT_RCJ,
            text: "Export .rcj Job File...",
        },
        w::MenuItem::Entry {
            cmd_id: IDM_EXPORT_BAT,
            text: "Export .bat Script...",
        },
        w::MenuItem::Entry {
            cmd_id: IDM_EXPORT_PS1,
            text: "Export .ps1 PowerShell Script...",
        },
        w::MenuItem::Separator,
        w::MenuItem::Entry {
            cmd_id: IDM_REPORT_HTML,
            text: "Export Last Run HTML Report...",
        },
        w::MenuItem::Entry {
            cmd_id: IDM_REPORT_ALL,
            text: "Export Last Run Bundle (All)...",
        },
        w::MenuItem::Separator,
        w::MenuItem::Entry {
            cmd_id: IDM_EXIT,
            text: "Exit",
        },
    ])?;

    let job_m = w::HMENU::CreatePopupMenu()?;
    job_m.append_item(&[
        w::MenuItem::Entry {
            cmd_id: IDM_RESET_JOB,
            text: "Reset Default Job",
        },
        w::MenuItem::Entry {
            cmd_id: IDM_SWAP_PATHS,
            text: "Swap Source <-> Destination",
        },
        w::MenuItem::Entry {
            cmd_id: IDM_COPY_CMD,
            text: "Copy Command Line",
        },
        w::MenuItem::Entry {
            cmd_id: IDM_SAVE_PROFILE,
            text: "Save Profile",
        },
        w::MenuItem::Entry {
            cmd_id: IDM_QUEUE_ADD,
            text: "Add Current Job to Batch Queue",
        },
    ])?;

    let exec_m = w::HMENU::CreatePopupMenu()?;
    exec_m.append_item(&[
        w::MenuItem::Entry {
            cmd_id: IDM_DRY_RUN,
            text: "Dry-Run Preview (/L)",
        },
        w::MenuItem::Entry {
            cmd_id: IDM_RUN_JOB,
            text: "Execute Job",
        },
        w::MenuItem::Entry {
            cmd_id: IDM_STOP_JOB,
            text: "Stop Active Job",
        },
    ])?;

    let help_m = w::HMENU::CreatePopupMenu()?;
    help_m.append_item(&[w::MenuItem::Entry {
        cmd_id: IDM_ABOUT,
        text: "About Robocopy2Rust Win32...",
    }])?;

    menu.append_item(&[
        w::MenuItem::Submenu {
            submenu: &file_m,
            text: "&File",
        },
        w::MenuItem::Submenu {
            submenu: &job_m,
            text: "&Job",
        },
        w::MenuItem::Submenu {
            submenu: &exec_m,
            text: "&Execution",
        },
        w::MenuItem::Submenu {
            submenu: &help_m,
            text: "&Help",
        },
    ])?;
    Ok(menu)
}

pub fn run(launch: LaunchArgs) -> anyhow::Result<()> {
    let hmenu = build_main_menu().map_err(|e| anyhow::anyhow!("Menu creation failed: {e}"))?;
    let state = Arc::new(Mutex::new(Win32State::new(&launch)));

    let wnd = gui::WindowMain::new(gui::WindowMainOpts {
        title: "Robocopy2Rust Management Console (Native Win32)",
        size: gui::dpi(1020, 660),
        style: co::WS::CAPTION
            | co::WS::SYSMENU
            | co::WS::CLIPCHILDREN
            | co::WS::BORDER
            | co::WS::VISIBLE
            | co::WS::SIZEBOX
            | co::WS::MINIMIZEBOX
            | co::WS::MAXIMIZEBOX,
        class_icon: gui::Icon::Id(101),
        class_bg_brush: gui::Brush::Color(co::COLOR::BTNFACE),
        menu: hmenu,
        ..Default::default()
    });

    // Row 1: Target paths
    lbl(&wnd, "Source:", 10, 12, 48);
    let src_edit = edit(&wnd, 60, 9, 310, 22, (gui::Horz::None, gui::Vert::None));
    let btn_src_browse = btn(&wnd, "...", 374, 8, 32, 24, (gui::Horz::None, gui::Vert::None));
    let btn_swap = btn(&wnd, "<->", 410, 8, 34, 24, (gui::Horz::None, gui::Vert::None));
    lbl(&wnd, "Dest:", 452, 12, 36);
    let dst_edit = edit(&wnd, 490, 9, 310, 22, (gui::Horz::None, gui::Vert::None));
    let btn_dst_browse = btn(&wnd, "...", 804, 8, 32, 24, (gui::Horz::None, gui::Vert::None));
    lbl(&wnd, "Files:", 844, 12, 36);
    let files_edit = edit(&wnd, 882, 9, 128, 22, (gui::Horz::Resize, gui::Vert::None));

    // Row 2: Presets, Profiles, Execution
    lbl(&wnd, "Preset:", 10, 40, 48);
    let preset_combo = gui::ComboBox::new(
        &wnd,
        gui::ComboBoxOpts {
            position: gui::dpi(60, 37),
            width: gui::dpi_x(190),
            ..Default::default()
        },
    );
    let btn_apply_preset = btn(&wnd, "Apply", 254, 36, 52, 24, (gui::Horz::None, gui::Vert::None));
    lbl(&wnd, "Profile:", 314, 40, 46);
    let profile_combo = gui::ComboBox::new(
        &wnd,
        gui::ComboBoxOpts {
            position: gui::dpi(362, 37),
            width: gui::dpi_x(150),
            ..Default::default()
        },
    );
    let btn_load_prof = btn(&wnd, "Load", 516, 36, 46, 24, (gui::Horz::None, gui::Vert::None));
    let btn_save_prof = btn(&wnd, "Save", 566, 36, 46, 24, (gui::Horz::None, gui::Vert::None));
    let btn_del_prof = btn(&wnd, "Del", 616, 36, 38, 24, (gui::Horz::None, gui::Vert::None));

    let btn_dry_run = btn(
        &wnd,
        "Dry-Run (/L)",
        710,
        36,
        90,
        24,
        (gui::Horz::Repos, gui::Vert::None),
    );
    let btn_run_job = btn(&wnd, "Run Job", 804, 36, 76, 24, (gui::Horz::Repos, gui::Vert::None));
    let btn_queue_add = btn(&wnd, "Queue +", 884, 36, 66, 24, (gui::Horz::Repos, gui::Vert::None));
    let btn_stop_job = btn(&wnd, "Stop", 954, 36, 56, 24, (gui::Horz::Repos, gui::Vert::None));

    // Center Tab Control
    let designer = DesignerPage::new(&wnd);
    let copy_pg = CopyPage::new(&wnd);
    let sel_pg = SelectionPage::new(&wnd);
    let perf_pg = PerfRetryPage::new(&wnd);
    let log_pg = LoggingPage::new(&wnd);
    let console_pg = ConsolePage::new(&wnd);
    let batch_pg = BatchHistoryPage::new(&wnd);

    let tab_pages = vec![
        designer.page.clone(),
        copy_pg.page.clone(),
        sel_pg.page.clone(),
        perf_pg.page.clone(),
        log_pg.page.clone(),
        console_pg.page.clone(),
        batch_pg.page.clone(),
    ];

    let tab = gui::Tab::new(
        &wnd,
        gui::TabOpts {
            position: gui::dpi(10, 66),
            size: gui::dpi(1000, 534),
            resize_behavior: (gui::Horz::Resize, gui::Vert::Resize),
            pages: &[
                ("Designer && Diagnostics", designer.page.clone()),
                ("Copy && Transfer", copy_pg.page.clone()),
                ("File Selection", sel_pg.page.clone()),
                ("Throttle && Retry", perf_pg.page.clone()),
                ("Logging && Jobs", log_pg.page.clone()),
                ("Live Console && Diff", console_pg.page.clone()),
                ("Batch && History", batch_pg.page.clone()),
            ],
            ..Default::default()
        },
    );

    // Bottom Command Bar
    let _ = gui::Label::new(
        &wnd,
        gui::LabelOpts {
            text: "CMD:",
            position: gui::dpi(10, 610),
            size: gui::dpi(38, 18),
            resize_behavior: (gui::Horz::None, gui::Vert::Repos),
            ..Default::default()
        },
    );
    let cmd_edit = gui::Edit::new(
        &wnd,
        gui::EditOpts {
            position: gui::dpi(50, 607),
            width: gui::dpi_x(742),
            height: gui::dpi_y(22),
            control_style: co::ES::AUTOHSCROLL | co::ES::READONLY,
            resize_behavior: (gui::Horz::Resize, gui::Vert::Repos),
            ..Default::default()
        },
    );
    let btn_copy_cmd = btn(&wnd, "Copy", 798, 606, 56, 24, (gui::Horz::Repos, gui::Vert::Repos));
    let btn_exp_rcj = btn(&wnd, ".rcj", 858, 606, 48, 24, (gui::Horz::Repos, gui::Vert::Repos));
    let btn_exp_bat = btn(&wnd, ".bat", 910, 606, 48, 24, (gui::Horz::Repos, gui::Vert::Repos));
    let btn_exp_ps1 = btn(&wnd, ".ps1", 962, 606, 48, 24, (gui::Horz::Repos, gui::Vert::Repos));

    // Bottom StatusBar
    let status_bar = gui::StatusBar::new(
        &wnd,
        &[
            gui::SbPart::Proportional(2),
            gui::SbPart::Fixed(gui::dpi_x(185)),
            gui::SbPart::Fixed(gui::dpi_x(180)),
            gui::SbPart::Fixed(gui::dpi_x(210)),
        ],
    );

    let app = Win32App {
        wnd,
        src_edit,
        btn_src_browse,
        btn_swap,
        dst_edit,
        btn_dst_browse,
        files_edit,
        preset_combo,
        btn_apply_preset,
        profile_combo,
        btn_load_prof,
        btn_save_prof,
        btn_del_prof,
        btn_dry_run,
        btn_run_job,
        btn_queue_add,
        btn_stop_job,
        tab,
        tab_pages,
        designer,
        copy_pg,
        sel_pg,
        perf_pg,
        log_pg,
        console_pg,
        batch_pg,
        cmd_edit,
        btn_copy_cmd,
        btn_exp_rcj,
        btn_exp_bat,
        btn_exp_ps1,
        status_bar,
        state,
        suppress_sync: Arc::new(AtomicBool::new(false)),
    };

    app.register_events();
    app.wnd
        .run_main(None)
        .map_err(|e| anyhow::anyhow!("Win32 message loop error: {e}"))?;
    Ok(())
}

impl Win32App {
    fn select_tab_index(&self, idx: u32) {
        unsafe {
            self.tab.hwnd().SendMessage(msg::TcmSetCurSel { index: idx });
        }
        self.layout_active_tab_page(idx);
    }

    fn layout_active_tab_page(&self, active_idx: u32) {
        for (i, pg) in self.tab_pages.iter().enumerate() {
            if i as u32 != active_idx {
                let _ = pg.hwnd().ShowWindow(co::SW::HIDE);
            }
        }
        if let Some(pg) = self.tab_pages.get(active_idx as usize)
            && let Ok(tab_rc) = self.tab.hwnd().GetWindowRect()
            && let Ok(mut rc) = self.wnd.hwnd().ScreenToClientRc(tab_rc)
        {
            unsafe {
                self.tab.hwnd().SendMessage(msg::TcmAdjustRect {
                    display_rect: false,
                    rect: &mut rc,
                });
            }
            let _ = pg.hwnd().SetWindowPos(
                w::HwndPlace::None,
                w::POINT::with(rc.left, rc.top),
                w::SIZE::with((rc.right - rc.left).max(10), (rc.bottom - rc.top).max(10)),
                co::SWP::NOZORDER | co::SWP::SHOWWINDOW,
            );
        }
    }

    fn sync_job_to_controls(&self) {
        self.suppress_sync.store(true, Ordering::SeqCst);
        let st = self.state.lock().unwrap();
        let j = &st.job;

        let _ = self.src_edit.set_text(&j.source);
        let _ = self.dst_edit.set_text(&j.destination);
        let _ = self.files_edit.set_text(&join_tokens(&j.file_specs));

        let _ = self.designer.job_name.set_text(&j.name);
        let _ = self.designer.job_desc.set_text(&j.description);

        // Copy tab
        self.copy_pg.s_non_empty.set_check(j.copy.subdirectories_non_empty);
        self.copy_pg
            .e_incl_empty
            .set_check(j.copy.subdirectories_including_empty);
        self.copy_pg.mir.set_check(j.copy.mirror);
        self.copy_pg.purge.set_check(j.copy.purge);
        self.copy_pg.mov_files.set_check(j.copy.move_files);
        self.copy_pg.move_all.set_check(j.copy.move_files_and_dirs);
        let _ = self
            .copy_pg
            .lev_edit
            .set_text(&j.copy.max_levels.map(|v| v.to_string()).unwrap_or_default());
        let _ = self
            .copy_pg
            .mt_edit
            .set_text(&j.copy.multithreaded.map(|v| v.to_string()).unwrap_or_default());
        self.copy_pg.z_restart.set_check(j.copy.restartable);
        self.copy_pg.b_backup.set_check(j.copy.backup_mode);
        self.copy_pg.zb_both.set_check(j.copy.restartable_or_backup);
        self.copy_pg.j_unbuf.set_check(j.copy.unbuffered_io);
        self.copy_pg.efsraw.set_check(j.copy.efs_raw);
        self.copy_pg.compress.set_check(j.copy.network_compress);
        self.copy_pg.no_offload.set_check(j.copy.no_offload);
        self.copy_pg.no_clone.set_check(j.copy.no_clone);
        let sp_idx = match j.copy.sparse_mode {
            SparseMode::Default => 0,
            SparseMode::Enable => 1,
            SparseMode::Disable => 2,
        };
        self.copy_pg.sparse_combo.items().select(Some(sp_idx));
        self.copy_pg.copy_d.set_check(j.copy.copy_flags.data);
        self.copy_pg.copy_a.set_check(j.copy.copy_flags.attributes);
        self.copy_pg.copy_t.set_check(j.copy.copy_flags.timestamps);
        self.copy_pg.copy_s.set_check(j.copy.copy_flags.security);
        self.copy_pg.copy_o.set_check(j.copy.copy_flags.owner);
        self.copy_pg.copy_u.set_check(j.copy.copy_flags.auditing);
        self.copy_pg.copy_x.set_check(j.copy.copy_flags.skip_alt_data_streams);
        self.copy_pg.no_copy.set_check(j.copy.no_copy);
        self.copy_pg.dcopy_d.set_check(j.copy.dir_copy_flags.data);
        self.copy_pg.dcopy_a.set_check(j.copy.dir_copy_flags.attributes);
        self.copy_pg.dcopy_t.set_check(j.copy.dir_copy_flags.timestamps);
        self.copy_pg
            .dcopy_e
            .set_check(j.copy.dir_copy_flags.extended_attributes);
        self.copy_pg
            .dcopy_x
            .set_check(j.copy.dir_copy_flags.skip_alt_data_streams);
        self.copy_pg.no_dcopy.set_check(j.copy.no_dir_copy);
        self.copy_pg.secfix.set_check(j.copy.sec_fix);
        self.copy_pg.timfix.set_check(j.copy.tim_fix);
        self.copy_pg.create_tree.set_check(j.copy.create_tree_only);
        self.copy_pg.fat_names.set_check(j.copy.fat_names);
        self.copy_pg.no_256.set_check(j.copy.disable_long_paths);
        self.copy_pg.copy_sj.set_check(j.copy.copy_junctions);
        self.copy_pg.copy_sl.set_check(j.copy.copy_symlinks);
        let _ = self.copy_pg.add_attrs.set_text(&j.copy.add_attributes.to_flag_string());
        let _ = self
            .copy_pg
            .rem_attrs
            .set_text(&j.copy.remove_attributes.to_flag_string());

        // Selection tab
        let _ = self.sel_pg.xf_edit.set_text(&join_tokens(&j.selection.exclude_files));
        let _ = self.sel_pg.xd_edit.set_text(&join_tokens(&j.selection.exclude_dirs));
        self.sel_pg.xj.set_check(j.selection.exclude_junctions);
        self.sel_pg.xjd.set_check(j.selection.exclude_dir_junctions);
        self.sel_pg.xjf.set_check(j.selection.exclude_file_symlinks);
        self.sel_pg.xc.set_check(j.selection.exclude_changed);
        self.sel_pg.xn.set_check(j.selection.exclude_newer);
        self.sel_pg.xo.set_check(j.selection.exclude_older);
        self.sel_pg.xx.set_check(j.selection.exclude_extra);
        self.sel_pg.xl.set_check(j.selection.exclude_lonely);
        self.sel_pg.is_same.set_check(j.selection.include_same);
        self.sel_pg.it_tweaked.set_check(j.selection.include_tweaked);
        self.sel_pg.im_modified.set_check(j.selection.include_modified);
        self.sel_pg.fft.set_check(j.selection.fat_file_times);
        self.sel_pg.dst.set_check(j.selection.compensate_dst);
        self.sel_pg.a_arch.set_check(j.selection.archive_only);
        self.sel_pg.m_reset.set_check(j.selection.archive_and_reset);
        let _ = self
            .sel_pg
            .ia_edit
            .set_text(&j.selection.include_attrs.to_flag_string());
        let _ = self
            .sel_pg
            .xa_edit
            .set_text(&j.selection.exclude_attrs.to_flag_string());
        let _ = self
            .sel_pg
            .max_sz
            .set_text(&j.selection.max_size_bytes.map(|v| v.to_string()).unwrap_or_default());
        let _ = self
            .sel_pg
            .min_sz
            .set_text(&j.selection.min_size_bytes.map(|v| v.to_string()).unwrap_or_default());
        let _ = self
            .sel_pg
            .max_age
            .set_text(j.selection.max_age.as_deref().unwrap_or(""));
        let _ = self
            .sel_pg
            .min_age
            .set_text(j.selection.min_age.as_deref().unwrap_or(""));
        let _ = self
            .sel_pg
            .max_lad
            .set_text(j.selection.max_last_access.as_deref().unwrap_or(""));
        let _ = self
            .sel_pg
            .min_lad
            .set_text(j.selection.min_last_access.as_deref().unwrap_or(""));

        // Perf & Retry tab
        let _ = self
            .perf_pg
            .r_edit
            .set_text(&j.retry.retries.map(|v| v.to_string()).unwrap_or_default());
        let _ = self
            .perf_pg
            .w_edit
            .set_text(&j.retry.wait_seconds.map(|v| v.to_string()).unwrap_or_default());
        self.perf_pg.tbd.set_check(j.retry.wait_for_sharenames);
        self.perf_pg.reg.set_check(j.retry.save_defaults_to_registry);
        self.perf_pg.lfsm.set_check(j.retry.low_free_space_mode);
        let _ = self
            .perf_pg
            .lfsm_floor
            .set_text(j.retry.low_free_space_floor.as_deref().unwrap_or(""));
        let _ = self
            .perf_pg
            .io_max_size
            .set_text(j.throttle.io_max_size.as_deref().unwrap_or(""));
        let _ = self
            .perf_pg
            .io_rate
            .set_text(j.throttle.io_rate.as_deref().unwrap_or(""));
        let _ = self
            .perf_pg
            .threshold
            .set_text(j.throttle.threshold.as_deref().unwrap_or(""));
        let _ = self
            .perf_pg
            .ipg_ms
            .set_text(&j.copy.inter_packet_gap_ms.map(|v| v.to_string()).unwrap_or_default());
        let _ = self
            .perf_pg
            .mon_n
            .set_text(&j.copy.monitor_changes.map(|v| v.to_string()).unwrap_or_default());
        let _ = self
            .perf_pg
            .mot_m
            .set_text(&j.copy.monitor_minutes.map(|v| v.to_string()).unwrap_or_default());
        let _ = self
            .perf_pg
            .rh_window
            .set_text(j.copy.run_hours.as_deref().unwrap_or(""));
        self.perf_pg.pf_per_file.set_check(j.copy.per_file_run_hours);

        // Logging & Jobs tab
        self.log_pg.l_list_only.set_check(j.logging.list_only);
        self.log_pg.x_extra.set_check(j.logging.report_extra);
        self.log_pg.v_verbose.set_check(j.logging.verbose);
        self.log_pg.ts_stamps.set_check(j.logging.include_timestamps);
        self.log_pg.fp_full.set_check(j.logging.full_paths);
        self.log_pg.bytes_flag.set_check(j.logging.sizes_in_bytes);
        self.log_pg.unicode_out.set_check(j.logging.unicode_output);
        self.log_pg.ns_nosize.set_check(j.logging.no_sizes);
        self.log_pg.nc_noclass.set_check(j.logging.no_classes);
        self.log_pg.nfl_nofile.set_check(j.logging.no_file_list);
        self.log_pg.ndl_nodir.set_check(j.logging.no_dir_list);
        self.log_pg.np_noprog.set_check(j.logging.no_progress);
        self.log_pg.eta_flag.set_check(j.logging.show_eta);
        self.log_pg.njh_flag.set_check(j.logging.no_job_header);
        self.log_pg.njs_flag.set_check(j.logging.no_job_summary);
        self.log_pg.tee_flag.set_check(j.logging.tee);
        let lm_idx = match j.logging.log_mode {
            LogFileMode::None => 0,
            LogFileMode::Overwrite => 1,
            LogFileMode::Append => 2,
            LogFileMode::UnicodeOverwrite => 3,
            LogFileMode::UnicodeAppend => 4,
        };
        self.log_pg.log_mode.items().select(Some(lm_idx));
        let _ = self
            .log_pg
            .log_path
            .set_text(j.logging.log_path.as_deref().unwrap_or(""));
        let _ = self
            .log_pg
            .job_load
            .set_text(j.job_files.load_job.as_deref().unwrap_or(""));
        let _ = self
            .log_pg
            .job_save
            .set_text(j.job_files.save_job.as_deref().unwrap_or(""));
        self.log_pg.quit_flag.set_check(j.job_files.quit_after_parse);
        self.log_pg.nosd_flag.set_check(j.job_files.no_source_dir);
        self.log_pg.nodd_flag.set_check(j.job_files.no_dest_dir);
        let _ = self.log_pg.extra_args.set_text(&join_tokens(&j.extra_args));

        drop(st);
        self.suppress_sync.store(false, Ordering::SeqCst);
        self.refresh_derived_views();
    }

    fn sync_controls_to_job(&self) {
        if self.suppress_sync.load(Ordering::SeqCst) {
            return;
        }

        let parse_opt_u32 = |e: &gui::Edit| -> Option<u32> { e.text().ok().and_then(|s| s.trim().parse().ok()) };
        let parse_opt_u16 = |e: &gui::Edit| -> Option<u16> { e.text().ok().and_then(|s| s.trim().parse().ok()) };
        let parse_opt_u64 = |e: &gui::Edit| -> Option<u64> { e.text().ok().and_then(|s| s.trim().parse().ok()) };
        let parse_opt_str =
            |e: &gui::Edit| -> Option<String> { e.text().ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) };

        {
            let mut st = self.state.lock().unwrap();
            let j = &mut st.job;
            j.source = self.src_edit.text().unwrap_or_default();
            j.destination = self.dst_edit.text().unwrap_or_default();
            j.file_specs = split_tokens(&self.files_edit.text().unwrap_or_default());
            j.name = self.designer.job_name.text().unwrap_or_default();
            j.description = self.designer.job_desc.text().unwrap_or_default();

            j.copy.subdirectories_non_empty = self.copy_pg.s_non_empty.is_checked();
            j.copy.subdirectories_including_empty = self.copy_pg.e_incl_empty.is_checked();
            j.copy.mirror = self.copy_pg.mir.is_checked();
            j.copy.purge = self.copy_pg.purge.is_checked();
            j.copy.move_files = self.copy_pg.mov_files.is_checked();
            j.copy.move_files_and_dirs = self.copy_pg.move_all.is_checked();
            j.copy.max_levels = parse_opt_u32(&self.copy_pg.lev_edit);
            j.copy.multithreaded = parse_opt_u16(&self.copy_pg.mt_edit);
            j.copy.restartable = self.copy_pg.z_restart.is_checked();
            j.copy.backup_mode = self.copy_pg.b_backup.is_checked();
            j.copy.restartable_or_backup = self.copy_pg.zb_both.is_checked();
            j.copy.unbuffered_io = self.copy_pg.j_unbuf.is_checked();
            j.copy.efs_raw = self.copy_pg.efsraw.is_checked();
            j.copy.network_compress = self.copy_pg.compress.is_checked();
            j.copy.no_offload = self.copy_pg.no_offload.is_checked();
            j.copy.no_clone = self.copy_pg.no_clone.is_checked();
            j.copy.sparse_mode = match self.copy_pg.sparse_combo.items().selected_index() {
                Some(1) => SparseMode::Enable,
                Some(2) => SparseMode::Disable,
                _ => SparseMode::Default,
            };
            j.copy.copy_flags = CopyFlags {
                data: self.copy_pg.copy_d.is_checked(),
                attributes: self.copy_pg.copy_a.is_checked(),
                timestamps: self.copy_pg.copy_t.is_checked(),
                security: self.copy_pg.copy_s.is_checked(),
                owner: self.copy_pg.copy_o.is_checked(),
                auditing: self.copy_pg.copy_u.is_checked(),
                skip_alt_data_streams: self.copy_pg.copy_x.is_checked(),
            };
            j.copy.no_copy = self.copy_pg.no_copy.is_checked();
            j.copy.dir_copy_flags = DirCopyFlags {
                data: self.copy_pg.dcopy_d.is_checked(),
                attributes: self.copy_pg.dcopy_a.is_checked(),
                timestamps: self.copy_pg.dcopy_t.is_checked(),
                extended_attributes: self.copy_pg.dcopy_e.is_checked(),
                skip_alt_data_streams: self.copy_pg.dcopy_x.is_checked(),
            };
            j.copy.no_dir_copy = self.copy_pg.no_dcopy.is_checked();
            j.copy.sec_fix = self.copy_pg.secfix.is_checked();
            j.copy.tim_fix = self.copy_pg.timfix.is_checked();
            j.copy.create_tree_only = self.copy_pg.create_tree.is_checked();
            j.copy.fat_names = self.copy_pg.fat_names.is_checked();
            j.copy.disable_long_paths = self.copy_pg.no_256.is_checked();
            j.copy.copy_junctions = self.copy_pg.copy_sj.is_checked();
            j.copy.copy_symlinks = self.copy_pg.copy_sl.is_checked();
            if let Ok(f) = FileAttrFlags::from_flag_string(&self.copy_pg.add_attrs.text().unwrap_or_default()) {
                j.copy.add_attributes = f;
            }
            if let Ok(f) = FileAttrFlags::from_flag_string(&self.copy_pg.rem_attrs.text().unwrap_or_default()) {
                j.copy.remove_attributes = f;
            }

            j.selection.exclude_files = split_tokens(&self.sel_pg.xf_edit.text().unwrap_or_default());
            j.selection.exclude_dirs = split_tokens(&self.sel_pg.xd_edit.text().unwrap_or_default());
            j.selection.exclude_junctions = self.sel_pg.xj.is_checked();
            j.selection.exclude_dir_junctions = self.sel_pg.xjd.is_checked();
            j.selection.exclude_file_symlinks = self.sel_pg.xjf.is_checked();
            j.selection.exclude_changed = self.sel_pg.xc.is_checked();
            j.selection.exclude_newer = self.sel_pg.xn.is_checked();
            j.selection.exclude_older = self.sel_pg.xo.is_checked();
            j.selection.exclude_extra = self.sel_pg.xx.is_checked();
            j.selection.exclude_lonely = self.sel_pg.xl.is_checked();
            j.selection.include_same = self.sel_pg.is_same.is_checked();
            j.selection.include_tweaked = self.sel_pg.it_tweaked.is_checked();
            j.selection.include_modified = self.sel_pg.im_modified.is_checked();
            j.selection.fat_file_times = self.sel_pg.fft.is_checked();
            j.selection.compensate_dst = self.sel_pg.dst.is_checked();
            j.selection.archive_only = self.sel_pg.a_arch.is_checked();
            j.selection.archive_and_reset = self.sel_pg.m_reset.is_checked();
            if let Ok(f) = FileAttrFlags::from_flag_string(&self.sel_pg.ia_edit.text().unwrap_or_default()) {
                j.selection.include_attrs = f;
            }
            if let Ok(f) = FileAttrFlags::from_flag_string(&self.sel_pg.xa_edit.text().unwrap_or_default()) {
                j.selection.exclude_attrs = f;
            }
            j.selection.max_size_bytes = parse_opt_u64(&self.sel_pg.max_sz);
            j.selection.min_size_bytes = parse_opt_u64(&self.sel_pg.min_sz);
            j.selection.max_age = parse_opt_str(&self.sel_pg.max_age);
            j.selection.min_age = parse_opt_str(&self.sel_pg.min_age);
            j.selection.max_last_access = parse_opt_str(&self.sel_pg.max_lad);
            j.selection.min_last_access = parse_opt_str(&self.sel_pg.min_lad);

            j.retry.retries = parse_opt_u32(&self.perf_pg.r_edit);
            j.retry.wait_seconds = parse_opt_u32(&self.perf_pg.w_edit);
            j.retry.wait_for_sharenames = self.perf_pg.tbd.is_checked();
            j.retry.save_defaults_to_registry = self.perf_pg.reg.is_checked();
            j.retry.low_free_space_mode = self.perf_pg.lfsm.is_checked();
            j.retry.low_free_space_floor = parse_opt_str(&self.perf_pg.lfsm_floor);
            j.throttle.io_max_size = parse_opt_str(&self.perf_pg.io_max_size);
            j.throttle.io_rate = parse_opt_str(&self.perf_pg.io_rate);
            j.throttle.threshold = parse_opt_str(&self.perf_pg.threshold);
            j.copy.inter_packet_gap_ms = parse_opt_u32(&self.perf_pg.ipg_ms);
            j.copy.monitor_changes = parse_opt_u32(&self.perf_pg.mon_n);
            j.copy.monitor_minutes = parse_opt_u32(&self.perf_pg.mot_m);
            j.copy.run_hours = parse_opt_str(&self.perf_pg.rh_window);
            j.copy.per_file_run_hours = self.perf_pg.pf_per_file.is_checked();

            j.logging.list_only = self.log_pg.l_list_only.is_checked();
            j.logging.report_extra = self.log_pg.x_extra.is_checked();
            j.logging.verbose = self.log_pg.v_verbose.is_checked();
            j.logging.include_timestamps = self.log_pg.ts_stamps.is_checked();
            j.logging.full_paths = self.log_pg.fp_full.is_checked();
            j.logging.sizes_in_bytes = self.log_pg.bytes_flag.is_checked();
            j.logging.unicode_output = self.log_pg.unicode_out.is_checked();
            j.logging.no_sizes = self.log_pg.ns_nosize.is_checked();
            j.logging.no_classes = self.log_pg.nc_noclass.is_checked();
            j.logging.no_file_list = self.log_pg.nfl_nofile.is_checked();
            j.logging.no_dir_list = self.log_pg.ndl_nodir.is_checked();
            j.logging.no_progress = self.log_pg.np_noprog.is_checked();
            j.logging.show_eta = self.log_pg.eta_flag.is_checked();
            j.logging.no_job_header = self.log_pg.njh_flag.is_checked();
            j.logging.no_job_summary = self.log_pg.njs_flag.is_checked();
            j.logging.tee = self.log_pg.tee_flag.is_checked();
            j.logging.log_mode = match self.log_pg.log_mode.items().selected_index() {
                Some(1) => LogFileMode::Overwrite,
                Some(2) => LogFileMode::Append,
                Some(3) => LogFileMode::UnicodeOverwrite,
                Some(4) => LogFileMode::UnicodeAppend,
                _ => LogFileMode::None,
            };
            j.logging.log_path = parse_opt_str(&self.log_pg.log_path);
            j.job_files.load_job = parse_opt_str(&self.log_pg.job_load);
            j.job_files.save_job = parse_opt_str(&self.log_pg.job_save);
            j.job_files.quit_after_parse = self.log_pg.quit_flag.is_checked();
            j.job_files.no_source_dir = self.log_pg.nosd_flag.is_checked();
            j.job_files.no_dest_dir = self.log_pg.nodd_flag.is_checked();
            j.extra_args = split_tokens(&self.log_pg.extra_args.text().unwrap_or_default());

            st.refresh_derived();
        }
        self.refresh_derived_views();
    }

    fn refresh_derived_views(&self) {
        let st = self.state.lock().unwrap();
        let _ = self.cmd_edit.set_text(&st.cached_command);
        let _ = self.designer.import_cmd.set_text(&st.cached_command);
        self.update_schtasks_preview(&st.cached_command);

        self.designer.diag_list.set_redraw(false);
        let _ = self.designer.diag_list.items().delete_all();
        if st.findings.is_empty() {
            let _ = self.designer.diag_list.items().add(
                &["OK", "VALID", "No configuration conflicts or hazards detected.", "-"],
                None,
                (),
            );
        } else {
            for f in &st.findings {
                let sev = match f.severity {
                    ValidationSeverity::Error => "ERROR",
                    ValidationSeverity::Warning => "WARN",
                    ValidationSeverity::Info => "INFO",
                };
                let _ = self.designer.diag_list.items().add(
                    &[sev, &f.code, &f.message, f.suggestion.as_deref().unwrap_or("-")],
                    None,
                    (),
                );
            }
        }
        self.designer.diag_list.set_redraw(true);

        self.refresh_status_bar(&st);
    }

    fn update_schtasks_preview(&self, cmd_line: &str) {
        let task_name = self.designer.sch_name.text().unwrap_or_default();
        let cadence = match self.designer.sch_cadence.items().selected_index() {
            Some(1) => ScheduleCadence::Weekly,
            Some(2) => ScheduleCadence::Hourly,
            Some(3) => ScheduleCadence::OnStart,
            Some(4) => ScheduleCadence::OnLogon,
            _ => ScheduleCadence::Daily,
        };
        let spec = ScheduleSpec {
            cadence,
            start_time: self.designer.sch_time.text().unwrap_or_else(|_| "02:00".to_string()),
            days: Some("MON,WED,FRI".to_string()),
            run_highest: self.designer.sch_highest.is_checked(),
        };
        let cmd = build_schtasks_command(&task_name, cmd_line, &spec);
        let _ = self.designer.sch_preview.set_text(&cmd);
    }

    fn refresh_status_bar(&self, st: &Win32State) {
        let _ = self.status_bar.parts().get(0).set_text(&st.status_message);
        let diag = if st.error_count() > 0 {
            format!("{} Err / {} Warn", st.error_count(), st.warning_count())
        } else if st.warning_count() > 0 {
            format!("{} Warning(s)", st.warning_count())
        } else {
            "Config Valid".to_string()
        };
        let _ = self.status_bar.parts().get(1).set_text(&diag);

        let run_seg = if st.is_running {
            format!("RUNNING ({:.0}%)", st.live_progress_pct)
        } else if let Some(res) = &st.last_result {
            format!("Last: {}", res.exit_info.short_label)
        } else {
            "Idle".to_string()
        };
        let _ = self.status_bar.parts().get(2).set_text(&run_seg);

        let env_seg = if st.env.robocopy_available {
            "Pure Rust - Native Win32 Ready"
        } else {
            "Pure Rust - Win32 (No robocopy)"
        };
        let _ = self.status_bar.parts().get(3).set_text(env_seg);
    }

    fn refresh_profile_combo(&self) {
        let st = self.state.lock().unwrap();
        self.profile_combo.items().delete_all();
        let names: Vec<String> = st.profiles.iter().map(|p| p.name.clone()).collect();
        if !names.is_empty() {
            let _ = self.profile_combo.items().add(&names);
            self.profile_combo.items().select(Some(0));
        }
    }

    fn refresh_batch_and_history_lists(&self) {
        let st = self.state.lock().unwrap();

        self.batch_pg.batch_list.set_redraw(false);
        let _ = self.batch_pg.batch_list.items().delete_all();
        for (i, item) in st.batch_items.iter().enumerate() {
            let idx_str = (i + 1).to_string();
            let status_str = item.status.label();
            let _ = self.batch_pg.batch_list.items().add(
                &[
                    &idx_str,
                    &item.job.name,
                    &item.job.source,
                    &item.job.destination,
                    &status_str,
                    &item.note,
                ],
                None,
                (),
            );
        }
        self.batch_pg.batch_list.set_redraw(true);

        self.batch_pg.hist_list.set_redraw(false);
        let _ = self.batch_pg.hist_list.items().delete_all();
        for h in &st.run_history {
            let mode = if h.dry_run { "Dry-Run" } else { "Live" };
            let files = format!("{}/{}", h.files_copied, h.files_failed);
            let paths = format!("{} -> {}", h.source, h.destination);
            let _ = self.batch_pg.hist_list.items().add(
                &[&h.timestamp, &h.job_name, mode, &h.exit_label, &files, &paths],
                None,
                (),
            );
        }
        self.batch_pg.hist_list.set_redraw(true);
    }

    fn refresh_action_list(&self) {
        let st = self.state.lock().unwrap();
        let filter = ActionFilter::from_index(self.console_pg.filter_combo.items().selected_index());
        self.console_pg.actions_list.set_redraw(false);
        let _ = self.console_pg.actions_list.items().delete_all();
        for rec in &st.live_file_actions {
            if !filter.matches(rec) {
                continue;
            }
            let sz = rec.size_bytes.map(format_bytes).unwrap_or_else(|| "-".to_string());
            let ts = rec.timestamp.as_deref().unwrap_or("-");
            let _ = self
                .console_pg
                .actions_list
                .items()
                .add(&[rec.kind.label(), &sz, ts, &rec.path], None, ());
        }
        self.console_pg.actions_list.set_redraw(true);
    }

    fn trigger_dry_run(&self) {
        let dry = {
            let st = self.state.lock().unwrap();
            if st.is_running {
                return;
            }
            st.job.with_list_only()
        };
        self.start_execution(dry);
    }

    fn trigger_execute(&self) {
        let job = {
            let mut st = self.state.lock().unwrap();
            if st.is_running {
                return;
            }
            st.refresh_derived();
            if st.error_count() > 0 {
                st.status_message = "Cannot run: fix validation errors first.".to_string();
                self.refresh_status_bar(&st);
                let _ = self.wnd.hwnd().MessageBox(
                    "Please resolve the red configuration errors shown in Designer & Diagnostics before running.",
                    "Validation Error",
                    co::MB::ICONERROR,
                );
                return;
            }
            st.job.clone()
        };

        if job.has_destructive_flags() {
            let flags = job.destructive_switch_names().join(", ");
            let msg_txt = format!(
                "Destructive option(s) active: {flags}\n\nSource: {}\nDestination: {}\n\nClick YES to Execute destructively.\nClick NO to run a safe Dry-Run (/L) preview instead.\nClick CANCEL to abort.",
                job.source, job.destination
            );
            match self.wnd.hwnd().MessageBox(
                &msg_txt,
                "Confirm Destructive Robocopy Execution",
                co::MB::YESNOCANCEL | co::MB::ICONWARNING,
            ) {
                Ok(co::DLGID::YES) => self.start_execution(job),
                Ok(co::DLGID::NO) => self.start_execution(job.with_list_only()),
                _ => {}
            }
            return;
        }
        self.start_execution(job);
    }

    fn start_execution(&self, job: RobocopyJob) {
        let errs: Vec<_> = robocopy2rust_core::validate_job(&job)
            .into_iter()
            .filter(|f| f.severity == ValidationSeverity::Error)
            .collect();
        if !errs.is_empty() {
            let _ = self
                .wnd
                .hwnd()
                .MessageBox(&errs[0].message, "Validation Error", co::MB::ICONERROR);
            return;
        }

        self.select_tab_index(TAB_CONSOLE_IDX);
        self.console_pg.progress.set_range(0, 100);
        self.console_pg.progress.set_position(0);
        self.console_pg.actions_list.set_redraw(false);
        let _ = self.console_pg.actions_list.items().delete_all();
        self.console_pg.actions_list.set_redraw(true);
        let _ = self.console_pg.raw_log_edit.set_text("");

        let (rx, cancel) = spawn_job_runner(job.clone(), None);
        {
            let mut st = self.state.lock().unwrap();
            st.is_running = true;
            st.cancel_token = Some(cancel);
            st.live_progress_pct = 0.0;
            st.live_current_file = None;
            st.live_summary = Default::default();
            st.live_log_lines.clear();
            st.live_file_actions.clear();
            st.live_errors.clear();
            st.last_result = None;
            st.status_message = if job.logging.list_only {
                format!("Running Dry-Run (/L): {} -> {}", job.source, job.destination)
            } else {
                format!("Running Robocopy: {} -> {}", job.source, job.destination)
            };
            self.refresh_status_bar(&st);
        }
        let _ = self
            .console_pg
            .summary_lbl
            .hwnd()
            .SetWindowText("Executing Robocopy... streaming events...");

        let app = self.clone();
        std::thread::spawn(move || {
            let mut last_flush = Instant::now();
            let mut pending_lines: Vec<String> = Vec::new();
            let mut pending_actions = Vec::new();
            let mut latest_pct: Option<f32> = None;
            let mut latest_summary = None;
            let mut finished: Option<RunResult> = None;

            loop {
                match rx.recv_timeout(Duration::from_millis(40)) {
                    Ok(ev) => match ev {
                        RunnerEvent::Started { command_line } => pending_lines.push(format!("> {command_line}")),
                        RunnerEvent::LogLine(line) => pending_lines.push(line),
                        RunnerEvent::FileAction(rec) => {
                            pending_lines.push(rec.raw_line.clone());
                            pending_actions.push(rec);
                        }
                        RunnerEvent::Progress { percent, .. } => latest_pct = Some(percent),
                        RunnerEvent::Error(err) => pending_lines.push(format!("[ERROR] {err}")),
                        RunnerEvent::Summary(sum) => latest_summary = Some(sum),
                        RunnerEvent::Finished(res) => {
                            finished = Some(*res);
                            break;
                        }
                    },
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }

                if last_flush.elapsed() >= Duration::from_millis(45)
                    && (!pending_lines.is_empty()
                        || !pending_actions.is_empty()
                        || latest_pct.is_some()
                        || latest_summary.is_some())
                {
                    let lines_batch = std::mem::take(&mut pending_lines);
                    let actions_batch = std::mem::take(&mut pending_actions);
                    let pct_opt = latest_pct.take();
                    let sum_opt = latest_summary.take();
                    let app2 = app.clone();
                    app.wnd.run_ui_thread(move || {
                        app2.apply_runner_batch(lines_batch, actions_batch, pct_opt, sum_opt);
                        Ok(())
                    });
                    last_flush = Instant::now();
                }
            }

            if !pending_lines.is_empty()
                || !pending_actions.is_empty()
                || latest_pct.is_some()
                || latest_summary.is_some()
                || finished.is_some()
            {
                let app2 = app.clone();
                app.wnd.run_ui_thread(move || {
                    app2.apply_runner_batch(pending_lines, pending_actions, latest_pct, latest_summary);
                    if let Some(res) = finished {
                        app2.finish_runner(res);
                    }
                    Ok(())
                });
            }
        });
    }

    fn apply_runner_batch(
        &self,
        lines: Vec<String>,
        actions: Vec<robocopy2rust_runner::FileActionRecord>,
        pct: Option<f32>,
        summary: Option<robocopy2rust_runner::RunSummary>,
    ) {
        let mut st = self.state.lock().unwrap();
        for l in lines {
            st.push_log_line(l);
        }
        let filter = ActionFilter::from_index(self.console_pg.filter_combo.items().selected_index());
        if !actions.is_empty() {
            self.console_pg.actions_list.set_redraw(false);
            for rec in actions {
                if filter.matches(&rec) {
                    let sz = rec.size_bytes.map(format_bytes).unwrap_or_else(|| "-".to_string());
                    let ts = rec.timestamp.as_deref().unwrap_or("-");
                    let _ = self
                        .console_pg
                        .actions_list
                        .items()
                        .add(&[rec.kind.label(), &sz, ts, &rec.path], None, ());
                }
                st.live_file_actions.push(rec);
            }
            self.console_pg.actions_list.set_redraw(true);
        }

        if let Some(p) = pct {
            st.live_progress_pct = p;
            self.console_pg.progress.set_position(p.clamp(0.0, 100.0) as u32);
        }
        if let Some(s) = summary {
            st.live_summary = s;
        }

        // Keep raw Win32 Edit updated with tail of log (UTF-16 CRLF for Edit control)
        let tail_start = st.live_log_lines.len().saturating_sub(600);
        let tail: String = st
            .live_log_lines
            .iter()
            .skip(tail_start)
            .cloned()
            .collect::<Vec<_>>()
            .join("\r\n");
        let _ = self.console_pg.raw_log_edit.set_text(&tail);
        let len = tail.len() as i32;
        self.console_pg.raw_log_edit.set_selection(len, len);

        let sum_txt = format!(
            "Files: {}/{} | Copied: {} | Extras: {} | Failed: {}",
            st.live_summary.files.copied,
            st.live_summary.files.total,
            format_bytes(st.live_summary.bytes.copied),
            st.live_summary.files.extras,
            st.live_summary.files.failed
        );
        let _ = self.console_pg.summary_lbl.hwnd().SetWindowText(&sum_txt);
        self.refresh_status_bar(&st);
    }

    fn finish_runner(&self, res: RunResult) {
        let next_batch = {
            let mut st = self.state.lock().unwrap();
            st.is_running = false;
            st.cancel_token = None;
            st.live_progress_pct = if res.exit_info.is_success {
                100.0
            } else {
                st.live_progress_pct
            };
            self.console_pg.progress.set_position(st.live_progress_pct as u32);
            st.live_summary = res.summary.clone();
            st.status_message = format!(
                "{} finished: {} ({} files copied, {} failed)",
                if res.dry_run { "Dry-Run (/L)" } else { "Robocopy" },
                res.exit_info.short_label,
                res.summary.files.copied,
                res.summary.files.failed
            );

            let sum_txt = format!(
                "Code {} ({}) | Files {}/{} | {} | {}",
                res.exit_code,
                res.exit_info.short_label,
                res.summary.files.copied,
                res.summary.files.total,
                format_bytes(res.summary.bytes.copied),
                format_duration(res.duration_secs)
            );
            let _ = self.console_pg.summary_lbl.hwnd().SetWindowText(&sum_txt);

            let entry = RunHistoryEntry {
                id: format!("run-{}", Utc::now().timestamp_millis()),
                timestamp: res.ended_at.clone(),
                job_name: res.job_name.clone(),
                source: res.source.clone(),
                destination: res.destination.clone(),
                dry_run: res.dry_run,
                command_line: res.command_line.clone(),
                exit_code: res.exit_code,
                exit_label: res.exit_info.short_label.clone(),
                is_success: res.exit_info.is_success,
                duration_secs: res.duration_secs,
                files_copied: res.summary.files.copied,
                files_failed: res.summary.files.failed,
                bytes_copied: res.summary.bytes.copied,
                summary_text: res.exit_info.summary.clone(),
            };
            if let Ok(updated) = st.store.append_history(entry) {
                st.run_history = updated;
            }

            let was_ok = res.exit_info.is_success;
            let code = res.exit_code;
            let copied = res.summary.files.copied;
            st.last_result = Some(res);

            let mut next_job = None;
            if st.batch_running
                && let Some(idx) = st.active_batch_idx
            {
                if idx < st.batch_items.len() {
                    st.batch_items[idx].status = if was_ok {
                        BatchStatus::Succeeded(code)
                    } else {
                        BatchStatus::Failed(code)
                    };
                    st.batch_items[idx].note = format!("Code {code} | {copied} files");
                }
                if !was_ok && st.batch_stop_on_error {
                    st.batch_running = false;
                    st.active_batch_idx = None;
                    st.status_message = format!("Batch stopped at item #{} (exit code {code}).", idx + 1);
                } else if idx + 1 < st.batch_items.len() {
                    let next_idx = idx + 1;
                    st.active_batch_idx = Some(next_idx);
                    st.batch_items[next_idx].status = BatchStatus::Running;
                    st.batch_items[next_idx].note = "Running...".to_string();
                    let j = if st.batch_dry_run {
                        st.batch_items[next_idx].job.with_list_only()
                    } else {
                        st.batch_items[next_idx].job.clone()
                    };
                    next_job = Some(j);
                } else {
                    st.batch_running = false;
                    st.active_batch_idx = None;
                    st.status_message = "Batch queue completed.".to_string();
                }
            }
            self.refresh_status_bar(&st);
            next_job
        };

        self.refresh_batch_and_history_lists();
        if let Some(j) = next_batch {
            self.start_execution(j);
        }
    }

    fn export_report(&self, fmt: ReportFormat) {
        let res_opt = self.state.lock().unwrap().last_result.clone();
        let Some(res) = res_opt else {
            let _ = self.wnd.hwnd().MessageBox(
                "Run or dry-run a job first before exporting an audit report.",
                "No Run Result",
                co::MB::ICONINFORMATION,
            );
            return;
        };

        let target = if fmt == ReportFormat::All {
            pick_folder(
                self.wnd.hwnd(),
                "Select Folder for HTML + JSON + Markdown Report Bundle",
            )
            .ok()
            .flatten()
            .map(std::path::PathBuf::from)
        } else {
            pick_save_file(
                self.wnd.hwnd(),
                "Save Robocopy2Rust Audit Report",
                "robocopy2rust_report.html",
                "html",
                &[("HTML Report", "*.html")],
            )
            .ok()
            .flatten()
        };

        if let Some(path) = target {
            match write_report(&res, fmt, &path) {
                Ok(files) => {
                    let mut st = self.state.lock().unwrap();
                    st.status_message = format!(
                        "Saved report: {}",
                        files
                            .iter()
                            .map(|p| p.display().to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    self.refresh_status_bar(&st);
                }
                Err(e) => {
                    let _ = self
                        .wnd
                        .hwnd()
                        .MessageBox(&e.to_string(), "Report Error", co::MB::ICONERROR);
                }
            }
        }
    }

    fn save_current_profile(&self) {
        let mut st = self.state.lock().unwrap();
        let mut name = st.job.name.trim().to_string();
        if name.is_empty() {
            name = format!("Profile_{}", Utc::now().format("%Y%m%d_%H%M%S"));
            st.job.name = name.clone();
        }
        let desc = st.job.description.clone();
        let job_clone = st.job.clone();
        match st.store.upsert_profile(&name, &desc, job_clone) {
            Ok(updated) => {
                st.profiles = updated;
                st.status_message = format!("Saved profile '{name}'.");
                self.refresh_status_bar(&st);
            }
            Err(e) => {
                let _ = self
                    .wnd
                    .hwnd()
                    .MessageBox(&e.to_string(), "Profile Error", co::MB::ICONERROR);
            }
        }
        drop(st);
        self.refresh_profile_combo();
    }

    fn import_rcj_dialog(&self) {
        if let Ok(Some(path)) = pick_open_file(
            self.wnd.hwnd(),
            "Import Microsoft Robocopy Job (.rcj)",
            &[("Robocopy Job (*.rcj)", "*.rcj"), ("All Files", "*.*")],
        ) {
            match std::fs::read(&path)
                .map_err(anyhow::Error::from)
                .and_then(|b| parse_rcj(&b))
            {
                Ok((job, warnings)) => {
                    {
                        let mut st = self.state.lock().unwrap();
                        st.job = job;
                        st.refresh_derived();
                        st.status_message = if warnings.is_empty() {
                            format!("Imported .rcj: {}", path.display())
                        } else {
                            format!("Imported .rcj with {} warning(s).", warnings.len())
                        };
                    }
                    self.sync_job_to_controls();
                }
                Err(e) => {
                    let _ = self
                        .wnd
                        .hwnd()
                        .MessageBox(&e.to_string(), "Import Error", co::MB::ICONERROR);
                }
            }
        }
    }

    fn export_text_artifact(&self, title: &str, default_name: &str, ext: &str, filter: (&str, &str), content: String) {
        if let Ok(Some(path)) = pick_save_file(self.wnd.hwnd(), title, default_name, ext, &[filter]) {
            match std::fs::write(&path, content) {
                Ok(_) => {
                    let mut st = self.state.lock().unwrap();
                    st.status_message = format!("Exported {}", path.display());
                    self.refresh_status_bar(&st);
                }
                Err(e) => {
                    let _ = self
                        .wnd
                        .hwnd()
                        .MessageBox(&e.to_string(), "Export Error", co::MB::ICONERROR);
                }
            }
        }
    }

    fn register_events(&self) {
        // Populate initial combos, edits & lists on WM_CREATE
        let self2 = self.clone();
        self.wnd.on().wm_create(move |_| {
            {
                let st = self2.state.lock().unwrap();
                let preset_names: Vec<String> = st.presets.iter().map(|p| p.name.clone()).collect();
                let _ = self2.preset_combo.items().add(&preset_names);
                self2.preset_combo.items().select(Some(0));
            }
            self2.suppress_sync.store(true, Ordering::SeqCst);
            let _ = self2.designer.sch_name.set_text("Robocopy2Rust_DailySync");
            let _ = self2.designer.sch_time.set_text("02:00");
            self2.designer.sch_highest.set_check(true);
            let _ = self2.batch_pg.code_edit.set_text("1");
            self2.batch_pg.chk_stop_err.set_check(true);
            self2.suppress_sync.store(false, Ordering::SeqCst);

            self2.refresh_profile_combo();
            self2.sync_job_to_controls();
            self2.refresh_batch_and_history_lists();
            self2.layout_active_tab_page(0);
            Ok(0)
        });

        // Keep active TabPage cleanly sized on window resize and tab click
        let self2 = self.clone();
        self.wnd.on().wm_size(move |_| {
            let idx = self2.tab.items().selected().map(|i| i.index()).unwrap_or(0);
            self2.layout_active_tab_page(idx);
            Ok(())
        });

        let self2 = self.clone();
        self.tab.on().tcn_sel_change(move || {
            let idx = self2.tab.items().selected().map(|i| i.index()).unwrap_or(0);
            self2.layout_active_tab_page(idx);
            Ok(())
        });

        // Wire live edits & checkboxes -> sync_controls_to_job
        let wire_edit = |ed: &gui::Edit| {
            let app = self.clone();
            ed.on().en_change(move || {
                app.sync_controls_to_job();
                Ok(())
            });
        };
        let wire_chk = |cb: &gui::CheckBox| {
            let app = self.clone();
            cb.on().bn_clicked(move || {
                app.sync_controls_to_job();
                Ok(())
            });
        };
        let wire_cmb = |cm: &gui::ComboBox| {
            let app = self.clone();
            cm.on().cbn_sel_change(move || {
                app.sync_controls_to_job();
                Ok(())
            });
        };

        wire_edit(&self.src_edit);
        wire_edit(&self.dst_edit);
        wire_edit(&self.files_edit);
        wire_edit(&self.designer.job_name);
        wire_edit(&self.designer.job_desc);

        // Scheduler preview live edits
        for ed in [&self.designer.sch_name, &self.designer.sch_time] {
            let app = self.clone();
            ed.on().en_change(move || {
                let cmd = app.state.lock().unwrap().cached_command.clone();
                app.update_schtasks_preview(&cmd);
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.designer.sch_cadence.on().cbn_sel_change(move || {
                let cmd = app.state.lock().unwrap().cached_command.clone();
                app.update_schtasks_preview(&cmd);
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.designer.sch_highest.on().bn_clicked(move || {
                let cmd = app.state.lock().unwrap().cached_command.clone();
                app.update_schtasks_preview(&cmd);
                Ok(())
            });
        }

        for ed in [
            &self.copy_pg.lev_edit,
            &self.copy_pg.mt_edit,
            &self.copy_pg.add_attrs,
            &self.copy_pg.rem_attrs,
            &self.sel_pg.xf_edit,
            &self.sel_pg.xd_edit,
            &self.sel_pg.ia_edit,
            &self.sel_pg.xa_edit,
            &self.sel_pg.max_sz,
            &self.sel_pg.min_sz,
            &self.sel_pg.max_age,
            &self.sel_pg.min_age,
            &self.sel_pg.max_lad,
            &self.sel_pg.min_lad,
            &self.perf_pg.r_edit,
            &self.perf_pg.w_edit,
            &self.perf_pg.lfsm_floor,
            &self.perf_pg.io_max_size,
            &self.perf_pg.io_rate,
            &self.perf_pg.threshold,
            &self.perf_pg.ipg_ms,
            &self.perf_pg.mon_n,
            &self.perf_pg.mot_m,
            &self.perf_pg.rh_window,
            &self.log_pg.log_path,
            &self.log_pg.job_load,
            &self.log_pg.job_save,
            &self.log_pg.extra_args,
        ] {
            wire_edit(ed);
        }

        for cb in [
            &self.copy_pg.purge,
            &self.copy_pg.z_restart,
            &self.copy_pg.b_backup,
            &self.copy_pg.zb_both,
            &self.copy_pg.j_unbuf,
            &self.copy_pg.efsraw,
            &self.copy_pg.compress,
            &self.copy_pg.no_offload,
            &self.copy_pg.no_clone,
            &self.copy_pg.copy_d,
            &self.copy_pg.copy_a,
            &self.copy_pg.copy_t,
            &self.copy_pg.copy_s,
            &self.copy_pg.copy_o,
            &self.copy_pg.copy_u,
            &self.copy_pg.copy_x,
            &self.copy_pg.no_copy,
            &self.copy_pg.dcopy_d,
            &self.copy_pg.dcopy_a,
            &self.copy_pg.dcopy_t,
            &self.copy_pg.dcopy_e,
            &self.copy_pg.dcopy_x,
            &self.copy_pg.no_dcopy,
            &self.copy_pg.secfix,
            &self.copy_pg.timfix,
            &self.copy_pg.create_tree,
            &self.copy_pg.fat_names,
            &self.copy_pg.no_256,
            &self.copy_pg.copy_sj,
            &self.copy_pg.copy_sl,
            &self.sel_pg.xj,
            &self.sel_pg.xjd,
            &self.sel_pg.xjf,
            &self.sel_pg.xc,
            &self.sel_pg.xn,
            &self.sel_pg.xo,
            &self.sel_pg.xx,
            &self.sel_pg.xl,
            &self.sel_pg.is_same,
            &self.sel_pg.it_tweaked,
            &self.sel_pg.im_modified,
            &self.sel_pg.fft,
            &self.sel_pg.dst,
            &self.perf_pg.tbd,
            &self.perf_pg.reg,
            &self.perf_pg.lfsm,
            &self.perf_pg.pf_per_file,
            &self.log_pg.l_list_only,
            &self.log_pg.x_extra,
            &self.log_pg.v_verbose,
            &self.log_pg.ts_stamps,
            &self.log_pg.fp_full,
            &self.log_pg.bytes_flag,
            &self.log_pg.unicode_out,
            &self.log_pg.ns_nosize,
            &self.log_pg.nc_noclass,
            &self.log_pg.nfl_nofile,
            &self.log_pg.ndl_nodir,
            &self.log_pg.np_noprog,
            &self.log_pg.eta_flag,
            &self.log_pg.njh_flag,
            &self.log_pg.njs_flag,
            &self.log_pg.tee_flag,
            &self.log_pg.quit_flag,
            &self.log_pg.nosd_flag,
            &self.log_pg.nodd_flag,
        ] {
            wire_chk(cb);
        }
        {
            let app = self.clone();
            self.copy_pg.s_non_empty.on().bn_clicked(move || {
                if app.copy_pg.s_non_empty.is_checked() {
                    app.copy_pg.e_incl_empty.set_check(false);
                    app.copy_pg.mir.set_check(false);
                }
                app.sync_controls_to_job();
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.copy_pg.e_incl_empty.on().bn_clicked(move || {
                if app.copy_pg.e_incl_empty.is_checked() {
                    app.copy_pg.s_non_empty.set_check(false);
                } else {
                    app.copy_pg.mir.set_check(false);
                }
                app.sync_controls_to_job();
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.copy_pg.mir.on().bn_clicked(move || {
                if app.copy_pg.mir.is_checked() {
                    app.copy_pg.e_incl_empty.set_check(true);
                    app.copy_pg.purge.set_check(true);
                    app.copy_pg.s_non_empty.set_check(false);
                }
                app.sync_controls_to_job();
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.copy_pg.mov_files.on().bn_clicked(move || {
                if app.copy_pg.mov_files.is_checked() {
                    app.copy_pg.move_all.set_check(false);
                }
                app.sync_controls_to_job();
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.copy_pg.move_all.on().bn_clicked(move || {
                if app.copy_pg.move_all.is_checked() {
                    app.copy_pg.mov_files.set_check(false);
                }
                app.sync_controls_to_job();
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.sel_pg.a_arch.on().bn_clicked(move || {
                if app.sel_pg.a_arch.is_checked() {
                    app.sel_pg.m_reset.set_check(false);
                }
                app.sync_controls_to_job();
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.sel_pg.m_reset.on().bn_clicked(move || {
                if app.sel_pg.m_reset.is_checked() {
                    app.sel_pg.a_arch.set_check(false);
                }
                app.sync_controls_to_job();
                Ok(())
            });
        }
        wire_cmb(&self.copy_pg.sparse_combo);
        wire_cmb(&self.log_pg.log_mode);

        // Quick /COPY buttons
        {
            let app = self.clone();
            self.copy_pg.btn_copy_dat.on().bn_clicked(move || {
                {
                    let mut st = app.state.lock().unwrap();
                    st.job.copy.copy_flags = CopyFlags::default();
                    st.job.copy.no_copy = false;
                    st.refresh_derived();
                }
                app.sync_job_to_controls();
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.copy_pg.btn_copy_sec.on().bn_clicked(move || {
                {
                    let mut st = app.state.lock().unwrap();
                    st.job.copy.copy_flags = CopyFlags::sec();
                    st.job.copy.no_copy = false;
                    st.refresh_derived();
                }
                app.sync_job_to_controls();
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.copy_pg.btn_copy_all.on().bn_clicked(move || {
                {
                    let mut st = app.state.lock().unwrap();
                    st.job.copy.copy_flags = CopyFlags::copy_all();
                    st.job.copy.no_copy = false;
                    st.refresh_derived();
                }
                app.sync_job_to_controls();
                Ok(())
            });
        }

        // Toolbar buttons
        {
            let app = self.clone();
            self.btn_src_browse.on().bn_clicked(move || {
                if let Ok(Some(path)) = pick_folder(app.wnd.hwnd(), "Select Robocopy Source Directory") {
                    let _ = app.src_edit.set_text(&path);
                    app.sync_controls_to_job();
                }
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.btn_dst_browse.on().bn_clicked(move || {
                if let Ok(Some(path)) = pick_folder(app.wnd.hwnd(), "Select Robocopy Destination Directory") {
                    let _ = app.dst_edit.set_text(&path);
                    app.sync_controls_to_job();
                }
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.btn_swap.on().bn_clicked(move || {
                {
                    let mut st = app.state.lock().unwrap();
                    let j = &mut st.job;
                    std::mem::swap(&mut j.source, &mut j.destination);
                    st.refresh_derived();
                }
                app.sync_job_to_controls();
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.btn_apply_preset.on().bn_clicked(move || {
                if let Some(idx) = app.preset_combo.items().selected_index() {
                    let mut st = app.state.lock().unwrap();
                    if let Some(p) = st.presets.get(idx as usize).cloned() {
                        st.apply_preset_preserving_paths(p.job);
                        st.status_message = format!("Applied preset '{}'.", p.name);
                    }
                    drop(st);
                    app.sync_job_to_controls();
                }
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.btn_load_prof.on().bn_clicked(move || {
                if let Some(idx) = app.profile_combo.items().selected_index() {
                    let mut st = app.state.lock().unwrap();
                    if let Some(p) = st.profiles.get(idx as usize).cloned() {
                        st.job = p.job;
                        st.refresh_derived();
                        st.status_message = format!("Loaded profile '{}'.", p.name);
                    }
                    drop(st);
                    app.sync_job_to_controls();
                }
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.btn_save_prof.on().bn_clicked(move || {
                app.save_current_profile();
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.btn_del_prof.on().bn_clicked(move || {
                if let Some(idx) = app.profile_combo.items().selected_index() {
                    let mut st = app.state.lock().unwrap();
                    if let Some(p) = st.profiles.get(idx as usize).cloned()
                        && let Ok((_, updated)) = st.store.delete_profile(&p.id)
                    {
                        st.profiles = updated;
                        st.status_message = format!("Deleted profile '{}'.", p.name);
                        app.refresh_status_bar(&st);
                    }
                    drop(st);
                    app.refresh_profile_combo();
                }
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.btn_dry_run.on().bn_clicked(move || {
                app.trigger_dry_run();
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.btn_run_job.on().bn_clicked(move || {
                app.trigger_execute();
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.btn_queue_add.on().bn_clicked(move || {
                {
                    let mut st = app.state.lock().unwrap();
                    st.add_current_to_batch();
                    app.refresh_status_bar(&st);
                }
                app.refresh_batch_and_history_lists();
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.btn_stop_job.on().bn_clicked(move || {
                let mut st = app.state.lock().unwrap();
                if let Some(tok) = &st.cancel_token {
                    tok.cancel();
                    st.batch_running = false;
                    st.active_batch_idx = None;
                    st.status_message = "Cancellation requested...".to_string();
                    app.refresh_status_bar(&st);
                }
                Ok(())
            });
        }

        // Designer parse & schtasks copy
        {
            let app = self.clone();
            self.designer.btn_parse_cmd.on().bn_clicked(move || {
                let raw = app.designer.import_cmd.text().unwrap_or_default();
                let parsed = parse_command_line(&raw);
                {
                    let mut st = app.state.lock().unwrap();
                    st.job = parsed.job;
                    st.refresh_derived();
                    st.status_message = if parsed.warnings.is_empty() {
                        "Parsed Robocopy command line into GUI.".to_string()
                    } else {
                        format!("Parsed with {} notice(s).", parsed.warnings.len())
                    };
                }
                app.sync_job_to_controls();
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.designer.btn_copy_sch.on().bn_clicked(move || {
                let cmd = app.designer.sch_preview.text().unwrap_or_default();
                if copy_to_clipboard(&cmd) {
                    let mut st = app.state.lock().unwrap();
                    st.status_message = "Copied schtasks command to clipboard.".to_string();
                    app.refresh_status_bar(&st);
                }
                Ok(())
            });
        }

        // Logging log file browse
        {
            let app = self.clone();
            self.log_pg.btn_log_browse.on().bn_clicked(move || {
                if let Ok(Some(path)) = pick_save_file(
                    app.wnd.hwnd(),
                    "Select Robocopy Log Output Path",
                    "robocopy.log",
                    "log",
                    &[("Log File (*.log)", "*.log"), ("All Files", "*.*")],
                ) {
                    let _ = app.log_pg.log_path.set_text(&path.display().to_string());
                    if app.log_pg.log_mode.items().selected_index() == Some(0) {
                        app.log_pg.log_mode.items().select(Some(1));
                    }
                    app.sync_controls_to_job();
                }
                Ok(())
            });
        }

        // Live console filter & exports
        {
            let app = self.clone();
            self.console_pg.filter_combo.on().cbn_sel_change(move || {
                app.refresh_action_list();
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.console_pg.btn_copy_log.on().bn_clicked(move || {
                let txt = {
                    let st = app.state.lock().unwrap();
                    st.live_log_lines.iter().cloned().collect::<Vec<_>>().join("\n")
                };
                if copy_to_clipboard(&txt) {
                    let mut st = app.state.lock().unwrap();
                    st.status_message = "Copied console log to clipboard.".to_string();
                    app.refresh_status_bar(&st);
                }
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.console_pg.btn_export_html.on().bn_clicked(move || {
                app.export_report(ReportFormat::Html);
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.console_pg.btn_export_all.on().bn_clicked(move || {
                app.export_report(ReportFormat::All);
                Ok(())
            });
        }

        // Batch Queue & History controls
        {
            let app = self.clone();
            self.batch_pg.btn_add.on().bn_clicked(move || {
                {
                    let mut st = app.state.lock().unwrap();
                    st.add_current_to_batch();
                    app.refresh_status_bar(&st);
                }
                app.refresh_batch_and_history_lists();
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.batch_pg.chk_stop_err.on().bn_clicked(move || {
                let checked = app.batch_pg.chk_stop_err.is_checked();
                app.state.lock().unwrap().batch_stop_on_error = checked;
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.batch_pg.btn_clear_queue.on().bn_clicked(move || {
                {
                    let mut st = app.state.lock().unwrap();
                    if !st.is_running {
                        st.batch_items.clear();
                    }
                }
                app.refresh_batch_and_history_lists();
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.batch_pg.btn_remove.on().bn_clicked(move || {
                if let Some(sel) = app.batch_pg.batch_list.items().iter_selected().next() {
                    let idx = sel.index() as usize;
                    let mut st = app.state.lock().unwrap();
                    if !st.is_running && idx < st.batch_items.len() {
                        st.batch_items.remove(idx);
                    }
                }
                app.refresh_batch_and_history_lists();
                Ok(())
            });
        }
        for (button, dry_mode) in [
            (&self.batch_pg.btn_dry_queue, true),
            (&self.batch_pg.btn_run_queue, false),
        ] {
            let app = self.clone();
            button.on().bn_clicked(move || {
                let first_job = {
                    let mut st = app.state.lock().unwrap();
                    if st.is_running || st.batch_items.is_empty() {
                        return Ok(());
                    }
                    for item in &mut st.batch_items {
                        item.status = BatchStatus::Pending;
                        item.note = "Pending".to_string();
                    }
                    st.batch_running = true;
                    st.batch_dry_run = dry_mode;
                    st.active_batch_idx = Some(0);
                    st.batch_items[0].status = BatchStatus::Running;
                    st.batch_items[0].note = "Running...".to_string();
                    if dry_mode {
                        st.batch_items[0].job.with_list_only()
                    } else {
                        st.batch_items[0].job.clone()
                    }
                };
                app.refresh_batch_and_history_lists();
                app.start_execution(first_job);
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.batch_pg.btn_reload_hist.on().bn_clicked(move || {
                if let Some(sel) = app.batch_pg.hist_list.items().iter_selected().next() {
                    let idx = sel.index() as usize;
                    let mut st = app.state.lock().unwrap();
                    if let Some(entry) = st.run_history.get(idx).cloned() {
                        st.job = parse_command_line(&entry.command_line).job;
                        st.refresh_derived();
                        st.status_message = format!("Reloaded command from '{}'.", entry.job_name);
                    }
                    drop(st);
                    app.sync_job_to_controls();
                }
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.batch_pg.btn_clear_hist.on().bn_clicked(move || {
                {
                    let mut st = app.state.lock().unwrap();
                    let _ = st.store.clear_history();
                    st.run_history.clear();
                }
                app.refresh_batch_and_history_lists();
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.batch_pg.btn_decode.on().bn_clicked(move || {
                let code: i32 = app
                    .batch_pg
                    .code_edit
                    .text()
                    .ok()
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);
                let info = decode_exit_code(code);
                let _ = app
                    .batch_pg
                    .decode_lbl
                    .hwnd()
                    .SetWindowText(&format!("{} | {}", info.short_label, info.summary));
                Ok(())
            });
        }

        // Bottom Command Bar
        {
            let app = self.clone();
            self.btn_copy_cmd.on().bn_clicked(move || {
                let cmd = app.state.lock().unwrap().cached_command.clone();
                if copy_to_clipboard(&cmd) {
                    let mut st = app.state.lock().unwrap();
                    st.status_message = "Copied Robocopy command line to clipboard.".to_string();
                    app.refresh_status_bar(&st);
                }
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.btn_exp_rcj.on().bn_clicked(move || {
                let content = serialize_rcj(&app.state.lock().unwrap().job);
                app.export_text_artifact(
                    "Export .rcj Job File",
                    "robocopy_job.rcj",
                    "rcj",
                    ("Robocopy Job (*.rcj)", "*.rcj"),
                    content,
                );
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.btn_exp_bat.on().bn_clicked(move || {
                let content = build_batch_script(&app.state.lock().unwrap().job);
                app.export_text_artifact(
                    "Export .bat Script",
                    "robocopy_job.bat",
                    "bat",
                    ("Batch Script (*.bat)", "*.bat"),
                    content,
                );
                Ok(())
            });
        }
        {
            let app = self.clone();
            self.btn_exp_ps1.on().bn_clicked(move || {
                let content = build_powershell_script(&app.state.lock().unwrap().job);
                app.export_text_artifact(
                    "Export PowerShell Script",
                    "robocopy_job.ps1",
                    "ps1",
                    ("PowerShell Script (*.ps1)", "*.ps1"),
                    content,
                );
                Ok(())
            });
        }

        // Native Menu Bar Commands
        let wire_menu = |id: u16, handler: Box<dyn Fn(&Win32App)>| {
            let app = self.clone();
            self.wnd.on().wm_command_acc_menu(id, move || {
                handler(&app);
                Ok(())
            });
        };
        wire_menu(IDM_IMPORT_RCJ, Box::new(|a| a.import_rcj_dialog()));
        wire_menu(
            IDM_EXPORT_RCJ,
            Box::new(|a| {
                let content = serialize_rcj(&a.state.lock().unwrap().job);
                a.export_text_artifact(
                    "Export .rcj Job File",
                    "robocopy_job.rcj",
                    "rcj",
                    ("Robocopy Job (*.rcj)", "*.rcj"),
                    content,
                );
            }),
        );
        wire_menu(
            IDM_EXPORT_BAT,
            Box::new(|a| {
                let content = build_batch_script(&a.state.lock().unwrap().job);
                a.export_text_artifact(
                    "Export .bat Script",
                    "robocopy_job.bat",
                    "bat",
                    ("Batch Script (*.bat)", "*.bat"),
                    content,
                );
            }),
        );
        wire_menu(
            IDM_EXPORT_PS1,
            Box::new(|a| {
                let content = build_powershell_script(&a.state.lock().unwrap().job);
                a.export_text_artifact(
                    "Export PowerShell Script",
                    "robocopy_job.ps1",
                    "ps1",
                    ("PowerShell Script (*.ps1)", "*.ps1"),
                    content,
                );
            }),
        );
        wire_menu(IDM_REPORT_HTML, Box::new(|a| a.export_report(ReportFormat::Html)));
        wire_menu(IDM_REPORT_ALL, Box::new(|a| a.export_report(ReportFormat::All)));
        wire_menu(
            IDM_EXIT,
            Box::new(|a| {
                let _ = a.wnd.hwnd().DestroyWindow();
            }),
        );
        wire_menu(
            IDM_RESET_JOB,
            Box::new(|a| {
                {
                    let mut st = a.state.lock().unwrap();
                    st.job = RobocopyJob::default();
                    st.refresh_derived();
                    st.status_message = "Reset to default Robocopy job.".to_string();
                }
                a.sync_job_to_controls();
            }),
        );
        wire_menu(
            IDM_SWAP_PATHS,
            Box::new(|a| {
                {
                    let mut st = a.state.lock().unwrap();
                    let j = &mut st.job;
                    std::mem::swap(&mut j.source, &mut j.destination);
                    st.refresh_derived();
                }
                a.sync_job_to_controls();
            }),
        );
        wire_menu(
            IDM_COPY_CMD,
            Box::new(|a| {
                let cmd = a.state.lock().unwrap().cached_command.clone();
                if copy_to_clipboard(&cmd) {
                    let mut st = a.state.lock().unwrap();
                    st.status_message = "Copied Robocopy command line to clipboard.".to_string();
                    a.refresh_status_bar(&st);
                }
            }),
        );
        wire_menu(IDM_SAVE_PROFILE, Box::new(|a| a.save_current_profile()));
        wire_menu(
            IDM_QUEUE_ADD,
            Box::new(|a| {
                {
                    let mut st = a.state.lock().unwrap();
                    st.add_current_to_batch();
                    a.refresh_status_bar(&st);
                }
                a.refresh_batch_and_history_lists();
            }),
        );
        wire_menu(IDM_DRY_RUN, Box::new(|a| a.trigger_dry_run()));
        wire_menu(IDM_RUN_JOB, Box::new(|a| a.trigger_execute()));
        wire_menu(
            IDM_STOP_JOB,
            Box::new(|a| {
                let mut st = a.state.lock().unwrap();
                if let Some(tok) = &st.cancel_token {
                    tok.cancel();
                    st.batch_running = false;
                    st.active_batch_idx = None;
                    st.status_message = "Cancellation requested...".to_string();
                    a.refresh_status_bar(&st);
                }
            }),
        );
        wire_menu(
            IDM_ABOUT,
            Box::new(|a| {
                let _ = a.wnd.hwnd().MessageBox(
                    concat!(
                        "Robocopy2Rust Management Console v",
                        env!("CARGO_PKG_VERSION"),
                        "\nNative Win32 Edition (WinSafe, zero GPU dependencies)\nPart of the GUAP Ecosystem (guap.ca)\nLicensed under Apache-2.0",
                    ),
                    "About Robocopy2Rust",
                    co::MB::ICONINFORMATION,
                );
            }),
        );
    }
}
