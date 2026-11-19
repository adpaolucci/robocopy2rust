pub mod batch_queue;
pub mod command_bar;
pub mod console_tree;
pub mod designer;
pub mod history_view;
pub mod live_console;
pub mod modals;
pub mod options_tabs;
pub mod ribbon;
pub mod status_bar;

use eframe::egui::{self, Color32, RichText, Stroke};
use robocopy2rust_core::FileAttrFlags;

pub fn win32_subheader(ui: &mut egui::Ui, title: &str, right_content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::NONE
        .fill(Color32::from_rgb(235, 235, 235))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(180, 180, 180)))
        .inner_margin(egui::Margin::symmetric(5, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(title).size(11.5).strong().color(Color32::BLACK));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), right_content);
            });
        });
}

pub fn win32_group_box(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::NONE
        .fill(Color32::from_rgb(244, 243, 238))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(172, 168, 153)))
        .inner_margin(egui::Margin::same(6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(title)
                    .size(11.5)
                    .strong()
                    .color(Color32::from_rgb(0, 51, 153)),
            );
            ui.separator();
            add_contents(ui);
        });
}

pub fn opt_u32_field(
    ui: &mut egui::Ui,
    label: &str,
    tip: &str,
    opt: &mut Option<u32>,
    default_val: u32,
    range: std::ops::RangeInclusive<u32>,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let mut enabled = opt.is_some();
        if ui.checkbox(&mut enabled, label).on_hover_text(tip).changed() {
            if enabled {
                *opt = Some(default_val);
            } else {
                *opt = None;
            }
            changed = true;
        }
        if let Some(val) = opt
            && ui
                .add(egui::DragValue::new(val).range(range).speed(0.2))
                .on_hover_text(tip)
                .changed()
        {
            changed = true;
        }
    });
    changed
}

pub fn opt_u16_field(
    ui: &mut egui::Ui,
    label: &str,
    tip: &str,
    opt: &mut Option<u16>,
    default_val: u16,
    range: std::ops::RangeInclusive<u16>,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let mut enabled = opt.is_some();
        if ui.checkbox(&mut enabled, label).on_hover_text(tip).changed() {
            if enabled {
                *opt = Some(default_val);
            } else {
                *opt = None;
            }
            changed = true;
        }
        if let Some(val) = opt
            && ui
                .add(egui::DragValue::new(val).range(range).speed(0.2))
                .on_hover_text(tip)
                .changed()
        {
            changed = true;
        }
    });
    changed
}

pub fn opt_u64_field(ui: &mut egui::Ui, label: &str, tip: &str, opt: &mut Option<u64>, default_val: u64) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let mut enabled = opt.is_some();
        if ui.checkbox(&mut enabled, label).on_hover_text(tip).changed() {
            if enabled {
                *opt = Some(default_val);
            } else {
                *opt = None;
            }
            changed = true;
        }
        if let Some(val) = opt
            && ui
                .add(egui::DragValue::new(val).range(0..=u64::MAX).speed(1024.0))
                .on_hover_text(tip)
                .changed()
        {
            changed = true;
        }
    });
    changed
}

pub fn opt_string_field(
    ui: &mut egui::Ui,
    label: &str,
    hint: &str,
    tip: &str,
    opt: &mut Option<String>,
    width: f32,
) -> bool {
    let mut changed = false;
    let mut buf = opt.clone().unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label(label).on_hover_text(tip);
        let resp = ui
            .add_sized([width, 20.0], egui::TextEdit::singleline(&mut buf).hint_text(hint))
            .on_hover_text(tip);
        if resp.changed() {
            let trimmed = buf.trim().to_string();
            *opt = if trimmed.is_empty() { None } else { Some(trimmed) };
            changed = true;
        }
    });
    changed
}

pub fn file_attr_editor(ui: &mut egui::Ui, label: &str, flags: &mut FileAttrFlags) -> bool {
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(label).size(11.0).strong());
        for (bit_label, tip, field) in [
            ("R", "Read-only", &mut flags.read_only),
            ("A", "Archive", &mut flags.archive),
            ("S", "System", &mut flags.system),
            ("H", "Hidden", &mut flags.hidden),
            ("C", "Compressed", &mut flags.compressed),
            ("N", "Not content indexed", &mut flags.not_indexed),
            ("E", "Encrypted", &mut flags.encrypted),
            ("T", "Temporary", &mut flags.temporary),
            ("O", "Offline", &mut flags.offline),
        ] {
            if ui.checkbox(field, bit_label).on_hover_text(tip).changed() {
                changed = true;
            }
        }
    });
    changed
}
