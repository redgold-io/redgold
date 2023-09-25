use eframe::egui::{Context, Ui};
use serde::Serialize;
use redgold_schema::EasyJsonDeser;
use redgold_schema::local_stored_state::LocalStoredState;
use crate::gui::app_loop::LocalState;
use crate::gui::common::{bounded_text_area, valid_label};


#[derive(Clone)]
pub struct SettingsState {
    lss_serialized: String,
    last_lss_serialized: String,
    new_lss: Option<LocalStoredState>,
    valid_json: bool
}

impl SettingsState {
    pub(crate) fn new(lss: String) -> SettingsState {
        Self {
            lss_serialized: lss,
            last_lss_serialized: "".to_string(),
            new_lss: None,
            valid_json: true,
        }
    }
}


pub fn settings_tab(ui: &mut Ui, ctx: &Context, ls: &mut LocalState) {
    ui.heading("Settings");

    ui.label("Local stored state json");
    bounded_text_area(ui, &mut ls.settings_state.lss_serialized);

    valid_label(ui,ls.settings_state.valid_json);

    if ls.settings_state.last_lss_serialized != ls.settings_state.lss_serialized {
        ls.settings_state.last_lss_serialized = ls.settings_state.lss_serialized.clone();
        let result = ls.settings_state.lss_serialized.json_from::<LocalStoredState>();
        if let Ok(lss) = result {
            ls.settings_state.valid_json = true;
            ls.settings_state.new_lss = Some(lss);
        } else {
            ls.settings_state.valid_json = false;
        }
    }

    if ui.button("Save Json").clicked() {
        if let Some(lss) = &ls.settings_state.new_lss {
            ls.local_stored_state = lss.clone();
            ls.persist_local_state_store();
        }
    }


}