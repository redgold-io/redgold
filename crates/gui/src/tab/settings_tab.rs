use crate::common::{bounded_text_area, valid_label};
use crate::dependencies::gui_depends::GuiDepends;
use eframe::egui::{Context, Ui};
use redgold_schema::config_data::ConfigData;
use redgold_schema::ErrorInfoContext;
use std::sync::Arc;

#[derive(Clone)]
pub struct SettingsState {
    pub local_ser_config: String,
    valid: bool,
    all_configurations: bool,
    last_parsed_config: ConfigData
}

#[derive(Clone, Default)]
pub struct SettingsDelta {
    pub updated_config: Option<ConfigData>,
    pub overwrite_all: bool
}

impl SettingsState {
    pub fn new(
        config_data: &Arc<ConfigData>
    ) -> SettingsState {
        let c = config_data.clone();
        let dat = (*c).clone();
        Self {
            local_ser_config: toml::to_string(&dat).unwrap(),
            valid: true,
            all_configurations: false,
            last_parsed_config: (*c).clone(),
        }
    }
    pub fn settings_tab<G>(&mut self, ui: &mut Ui, _ctx: &Context, g: &G)
    -> SettingsDelta
    where G: GuiDepends + Clone + Send + 'static
    {
        ui.heading("Settings");
        ui.separator();
        ui.checkbox(&mut self.all_configurations,"Update ALL Configurations");
        ui.label("Config TOML");
        let changed = bounded_text_area(ui, &mut self.local_ser_config);
        valid_label(ui, self.valid);
        if changed {
            let result = toml::from_str::<ConfigData>(&self.local_ser_config).error_info("toml parse failure");
            match result {
                Ok(c) => {
                    self.last_parsed_config = c;
                }
                Err(e) => {
                    self.valid = false;
                }
            }
        }
        let mut delta = SettingsDelta::default();
        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                delta.updated_config = Some(self.last_parsed_config.clone());
                delta.overwrite_all = self.all_configurations;
            }
            if ui.button("Reset Input to Existing Config").clicked() {
                let config = g.get_config();
                self.last_parsed_config = config.clone();
                self.local_ser_config = toml::to_string(&config).unwrap();
            }
        });
        delta
    }
}