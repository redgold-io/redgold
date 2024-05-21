use eframe::egui::{Context, Ui};
use serde::Serialize;
use redgold_schema::helpers::easy_json::EasyJsonDeser;
use redgold_schema::local_stored_state::LocalStoredState;
use crate::gui::app_loop::LocalState;
use crate::gui::common::{bounded_text_area, editable_text_input_copy, valid_label};


#[derive(Clone)]
pub struct SettingsState {
    pub(crate) lss_serialized: String,
    last_lss_serialized: String,
    new_lss: Option<LocalStoredState>,
    valid_json: bool,
    data_folder: String,
    secure_data_folder: String,
}

impl SettingsState {
    pub(crate) fn new(
        lss: String,
        data_folder: String,
        secure_data_folder: String
    ) -> SettingsState {
        Self {
            lss_serialized: lss,
            last_lss_serialized: "".to_string(),
            new_lss: None,
            valid_json: true,
            data_folder,
            secure_data_folder
        }
    }
}


pub fn settings_tab(ui: &mut Ui, _ctx: &Context, ls: &mut LocalState) {
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

    // TODO: Change to separate window to isolate the immutable change
    if ui.button("Save Json").clicked() {
        if let Some(lss) = &ls.settings_state.new_lss {
            ls.local_stored_state = lss.clone();
            ls.persist_local_state_store();
        }
    }
    //
    // editable_text_input_copy(ui,"Data Directory for .rg folder", &mut ls.settings_state.lss_serialized);
    //
    // if ui.button("Update Settings").clicked() {
    //     if let Some(lss) = &ls.settings_state.new_lss {
    //         ls.local_stored_state = lss.clone();
    //         ls.persist_local_state_store();
    //     }
    // }


}