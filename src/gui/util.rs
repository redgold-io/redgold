use eframe::egui::{Color32, RichText, Ui};

pub fn valid_label(ui: &mut Ui, bool: bool) {
    if bool {
        ui.label(RichText::new("Valid").color(Color32::GREEN));
    } else {
        ui.label(RichText::new("Invalid").color(Color32::RED));
    }
}