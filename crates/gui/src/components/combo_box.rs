use eframe::egui::{ComboBox, Ui};
use std::fmt::{Debug, Display};

pub fn combo_box<T>(
    ui: &mut Ui,
    selector_var: &mut T,
    label: impl Into<String>,
    potential_values: Vec<T>,
    locked: bool,
    width: f32,
    id: Option<String>
) -> bool where T: Clone + PartialEq + Debug {
    let mut changed = false;
    let mut c = selector_var.clone();
    let selector = if locked {
        &mut c
    } else {
        selector_var
    };
    let string = label.into();
    let id = id.unwrap_or(string.clone());
    ui.push_id(id, |ui| {
        ComboBox::from_label(string)
            .width(width)
            .selected_text(format!("{:?}", selector))
            .show_ui(ui, |ui| {
                for style in potential_values.into_iter() {
                    if ui.selectable_value(selector, style.clone(), format!("{:?}", style)).changed() {
                        changed = true;
                    }
                }
            });
    });
    changed
}
