use eframe::egui;
use eframe::egui::{Color32, Response, RichText, TextEdit, TextStyle, Ui, Widget};

pub fn valid_label(ui: &mut Ui, bool: bool) {
    if bool {
        green_label(ui, "Valid");
    } else {
        ui.label(RichText::new("Invalid").color(Color32::RED));
    }
}

pub fn green_label(ui: &mut Ui, text: impl Into<String>) -> Response {
    ui.label(RichText::new(text.into()).color(Color32::GREEN))
}

pub fn editable_text_input_copy(
    ui: &mut Ui, label: impl Into<String>, edit_str: &mut String, width: f32
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label.into());
        let r = ui.add(egui::TextEdit::singleline(edit_str).desired_width(width));
        changed = r.changed();
        copy_to_clipboard(ui, edit_str.clone());
    });
    changed
}

pub fn copy_to_clipboard(ui: &mut Ui, text: impl Into<String>) {
    let style = ui.style_mut();
    style.override_text_style = Some(TextStyle::Small);
    copy_button(ui, text);
}

pub fn copy_button(ui: &mut Ui, text: impl Into<String> + Sized) {
    if ui.small_button("Copy").clicked() {
        ui.ctx().output_mut(|o| o.copied_text = text.into().clone());
    }
}

pub fn data_item(ui: &mut Ui, label: impl Into<String>, text: impl Into<String>) {
    ui.horizontal(|ui| {
        let style = ui.style_mut();
        style.override_text_style = Some(TextStyle::Small);
        ui.label(label.into());
        let string = text.into();
        let text_line = &mut string.clone();
        ui.add(egui::TextEdit::singleline(text_line).clip_text(false));
        copy_to_clipboard(ui, string.clone());
    });
}

pub fn data_item_hyperlink(ui: &mut Ui, label: impl Into<String>, text: impl Into<String>, to: impl Into<String>) {
    ui.horizontal(|ui| {
        let style = ui.style_mut();
        style.override_text_style = Some(TextStyle::Small);
        // ui.label(label.into());
        ui.hyperlink_to(label.into(), to.into());
        let string = text.into();
        let text_line = &mut string.clone();
        ui.add(egui::TextEdit::singleline(text_line).clip_text(false));
        copy_to_clipboard(ui, string.clone());
    });
}

pub fn data_item_multiline_fixed(ui: &mut Ui, label: impl Into<String>, text: impl Into<String>, width: impl Into<f32>) {
    ui.horizontal(|ui| {
        let style = ui.style_mut();
        style.override_text_style = Some(TextStyle::Small);
        ui.label(label.into());
        let string = text.into();
        let text_line = &mut string.clone();
        TextEdit::multiline(text_line)
            .lock_focus(false)
            .desired_width(width.into())
            .desired_rows(2)
            .show(ui);
        copy_to_clipboard(ui, string.clone());
    });
}

pub fn medium_data_item(ui: &mut Ui, label: impl Into<String>, text: impl Into<String>) {
    ui.horizontal(|ui| {
        ui.label(label.into());
        ui.spacing();
        let x = text.into();
        ui.label(x.clone());
        let style = ui.style_mut();
        style.override_text_style = Some(TextStyle::Small);
        copy_to_clipboard(ui, x.clone());
    });
}

pub fn medium_data_item_vertical(ui: &mut Ui, label: impl Into<String>, text: impl Into<String>) {
    ui.vertical(|ui| {
        ui.label(label.into());
        ui.spacing();
        let x = text.into();
        ui.label(x.clone());
        let style = ui.style_mut();
        style.override_text_style = Some(TextStyle::Small);
        copy_to_clipboard(ui, x.clone());
    });
}

pub fn big_button<S: Into<String>>(ui: &mut Ui, lb: S) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let style = ui.style_mut();
        style.override_text_style = Some(TextStyle::Heading);
        changed = ui.button(lb.into()).clicked()
    });
    changed
}

pub fn bounded_text_area(ui: &mut Ui, string1: &mut String) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            let res = egui::TextEdit::multiline(string1)
                .desired_width(600.0)
                .desired_rows(2)
                .clip_text(true)
                .ui(ui);
            changed = res.changed()
        });
    });
    changed
}

pub fn bounded_text_area_size(ui: &mut Ui, string1: &mut String, width: f32, height: usize) {
    ui.horizontal(|ui| {
        let _area = egui::ScrollArea::vertical()
            .max_height(height as f32 * 100.)
            .show(ui, |ui| {
            egui::TextEdit::multiline(string1)
                .desired_width(width)
                .desired_rows(height)
                .clip_text(true)
                .ui(ui);
        });
    });
}

pub fn bounded_text_area_size_id(ui: &mut Ui, string1: &mut String, width: f32, height: usize, id_source: impl Into<String>) {
    ui.horizontal(|ui| {
        let _area = egui::ScrollArea::vertical()
            .id_salt(id_source.into())
            .max_height(height as f32 * 100.)
            .show(ui, |ui| {
            egui::TextEdit::multiline(string1)
                .desired_width(width)
                .desired_rows(height)
                .clip_text(true)
                .ui(ui);
        });
    });
}

pub fn bounded_text_area_size_focus(ui: &mut Ui, string1: &mut String, width: f32, height: usize) {
    ui.horizontal(|ui| {
        let area1 = egui::ScrollArea::vertical();
        let height_lines = 25.0 * height as f32;
        let area = area1.stick_to_bottom(true)
            .id_salt("bounded_text_area_size_focus")
            .max_height(height_lines)
            .min_scrolled_height(height_lines/2.0);
            // .max_width(width)
            // .min_scrolled_width(width)
            // .auto_shrink(true);
        let res = area.show(ui, |ui| {
            egui::TextEdit::multiline(string1)
                .desired_width(width)
                .desired_rows(height)
                .clip_text(true)
                .ui(ui);
        });
        res.state
    });
}



pub fn password_single(
    text: &mut String,
    label: impl Into<String>,
    ui: &mut Ui,
    show: &mut bool
) {
    ui.horizontal(|ui| {
        ui.label(label.into());
        TextEdit::singleline(text)
            .password(show.clone())
            .desired_width(250f32)
            .desired_rows(1)
            .show(ui);
        if ui.button("Show").clicked() {
            *show = !*show;
        }
    });
}
