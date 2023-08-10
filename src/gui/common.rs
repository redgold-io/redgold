use eframe::egui;
use eframe::egui::{Color32, RichText, TextStyle, Ui};

pub fn valid_label(ui: &mut Ui, bool: bool) {
    if bool {
        ui.label(RichText::new("Valid").color(Color32::GREEN));
    } else {
        ui.label(RichText::new("Invalid").color(Color32::RED));
    }
}

pub fn editable_text_input_copy(
    ui: &mut Ui, label: impl Into<String>, edit_str: &mut String, width: f32
) {
    ui.horizontal(|ui| {
        ui.label(label.into());
        ui.add(egui::TextEdit::singleline(edit_str).desired_width(width));
        copy_to_clipboard(ui, edit_str.clone());
    });
}

pub fn copy_to_clipboard(ui: &mut Ui, text: impl Into<String>) {
    let style = ui.style_mut();
    style.override_text_style = Some(TextStyle::Small);
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

pub fn big_button<S: Into<String>>(mut ui: Ui, lb: S) {
    ui.horizontal(|ui| {
        let style = ui.style_mut();
        style.override_text_style = Some(TextStyle::Heading);
        ui.button(lb.into())
    });
}
