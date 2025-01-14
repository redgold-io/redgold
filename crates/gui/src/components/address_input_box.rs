use eframe::egui;
use eframe::egui::{ComboBox, Ui};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, EnumString};
use redgold_schema::structs::{Address, SupportedCurrency};
use crate::common::valid_label;
use crate::dependencies::gui_depends::GuiDepends;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, EnumString, EnumIter)]
pub enum AddressInputMode {
    Raw,
    Saved,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AddressInputBox {
    pub input_box_str: String,
    pub locked: bool,
    pub input_locked: bool,
    pub valid: bool,
    pub input_has_changed: bool,
    pub mode_has_changed: bool,
    pub address_currency: SupportedCurrency,
    pub address: Address,
    pub address_changed_and_valid: bool,
    pub use_label_input: bool,
    pub address_input_mode: AddressInputMode,
    pub filter: String,
    pub selected_label: String,
    pub allow_mode_change: bool,
    pub address_box_label: String,
    pub show_valid: bool,
}

impl Default for AddressInputBox {
    fn default() -> Self {
        Self {
            input_box_str: "".to_string(),
            locked: false,
            input_locked: false,
            valid: false,
            input_has_changed: false,
            mode_has_changed: false,
            address_currency: SupportedCurrency::Redgold,
            address: Address::default(),
            address_changed_and_valid: false,
            use_label_input: false,
            address_input_mode: AddressInputMode::Raw,
            filter: "".to_string(),
            selected_label: "Select".to_string(),
            allow_mode_change: true,
            address_box_label: "Destination:".to_string(),
            show_valid: false,
        }
    }
}

impl Default for AddressInputMode {
    fn default() -> Self {
        AddressInputMode::Raw
    }
}

impl AddressInputBox {

    pub fn reset(&mut self) {
        self.input_box_str = "".to_string();
        self.valid = false;
        self.input_has_changed = false;
        self.mode_has_changed = false;
        self.address = Address::default();
        self.address_changed_and_valid = false;
        self.selected_label = "Select".to_string();
    }

    fn address_input_mode(&mut self, ui: &mut Ui) {
        ComboBox::from_label("Address Input Mode")
            .width(80.0)
            .selected_text(format!("{:?}", self.address_input_mode))
            .show_ui(ui, |ui| {
                let styles = AddressInputMode::iter();
                for style in styles {
                    let mut mode = &mut self.address_input_mode;
                    let mut mode2 = mode.clone();
                    if self.locked {
                        mode = &mut mode2;
                    }
                    if ui.selectable_value(mode, style.clone(), format!("{:?}", style)).changed() {
                        self.mode_has_changed = true;
                    }
                }
            });
    }

    fn input_box(&mut self, ui: &mut Ui) {
        let mut text = &mut self.input_box_str;
        let mut string = text.clone();
        if self.locked || self.input_locked {
            text = &mut string;
        }
        let edit = egui::TextEdit::singleline(text).desired_width(500.0);

        let response = ui.add(edit);
        if response.changed() {
            self.input_has_changed = true;
        }
    }


    pub fn view<G>(&mut self, ui: &mut Ui, saved_labeled: Vec<(String, Address)>, g: &G) where G: GuiDepends + Send + Clone + 'static {
        self.input_has_changed = false;
        self.address_changed_and_valid = false;
        self.mode_has_changed = false;
        self.input_locked = self.address_input_mode != AddressInputMode::Raw;

        if self.address_input_mode == AddressInputMode::Saved {
            ui.horizontal(|ui| {
                ui.label("Saved Address:");
                ui.push_id("saved_address", |ui| {
                    ComboBox::from_label("")
                        .width(250.0)
                        .selected_text(self.selected_label.clone())
                        .show_ui(ui, |ui| {
                            let vec = saved_labeled.iter().filter(|(label, _)| {
                                label.contains(&self.filter)
                            }).collect::<Vec<_>>();
                            for (label, address) in vec {
                                if ui.selectable_value(&mut self.address, address.clone(), label).changed() {
                                    self.address_changed_and_valid = true;
                                    self.valid = true;
                                    self.address = address.clone();
                                    self.input_box_str = address.render_string().unwrap_or("".to_string());
                                    self.selected_label = label.clone();
                                }
                            }
                        });
                });
                ui.label("Filter:");
                let edit = egui::TextEdit::singleline(&mut self.filter).desired_width(100.0);
                ui.add(edit);
            });
        }

        ui.horizontal(|ui| {
            ui.label(self.address_box_label.clone());
            self.input_box(ui);
            if self.allow_mode_change {
                self.address_input_mode(ui);
            }
            if self.show_valid {
                valid_label(ui, self.valid);
            }
        });

        if self.input_has_changed {
            if let Ok(result) = g.parse_address(&self.input_box_str) {
                self.address = result;
                self.address_changed_and_valid = true;
                self.valid = true;
            } else {
                self.address_changed_and_valid = false;
            }
        }
    }
}