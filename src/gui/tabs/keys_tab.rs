use eframe::egui;
use eframe::egui::{ScrollArea, Ui};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, EnumString};
use crate::gui::app_loop::LocalState;
use crate::gui::components::key_source_sel::{add_new_key_button, key_source};
use crate::gui::components::save_key_window;
use crate::gui::tabs::keygen_subtab;
use crate::gui::tabs::keygen_subtab::keys_screen_scroll;


#[derive(Debug, EnumIter, Clone, Serialize, Deserialize, EnumString)]
#[repr(i32)]
enum KeygenSubTab {
    Manage,
    Generate,
}

pub struct KeyTabState {
    pub keygen_subtab: KeygenSubTab,
}

impl Default for KeyTabState {
    fn default() -> Self {
        KeyTabState {
            keygen_subtab: KeygenSubTab::Manage,
        }
    }
}


pub fn manage_view(ui: &mut Ui, ctx: &egui::Context, ls: &mut LocalState) {
    ui.heading("Manage");
    ui.separator();

    // Add New Stuff buttons
    add_new_key_button(ls, ui);

    save_key_window::save_key_window(ui, ls, ctx);
    key_source(ui, ls);
}

pub fn keys_tab(ui: &mut Ui, ctx: &egui::Context, local_state: &mut LocalState) {
    ui.heading("Keys");
    ui.separator();

    ui.horizontal(|ui| {
    KeygenSubTab::iter().for_each(|subtab| {
        if ui.button(format!("{:?}", subtab)).clicked() {
            local_state.keytab_state.keygen_subtab = subtab;
        }
    })
    });
    match local_state.keytab_state.keygen_subtab {
        KeygenSubTab::Manage => {
            manage_view(ui, ctx, local_state);
        }
        KeygenSubTab::Generate => {
            keygen_subtab::keys_screen(ui, ctx, local_state);
        }
    }
}