use eframe::egui::{ComboBox, Context, Ui};
use redgold_schema::local_stored_state::{Identity, NamedXpub};
use redgold_schema::structs::{PublicKey, Transaction};
use crate::gui::app_loop::LocalState;
use crate::gui::common::editable_text_input_copy;


#[derive(Clone)]
pub struct IdentityState {
    selected_name: String,
    last_selected_name: String,
    selected_identity: Option<Identity>,
    identity_name_edit: String,
    edit_peer_id_index: String,
    edit_xpub_name: String,
    peer_tx: Option<Transaction>
}

impl IdentityState {
    pub(crate) fn new() -> IdentityState {
        Self {
            selected_name: "Add New Identity".to_string(),
            last_selected_name: "".to_string(),
            selected_identity: None,
            identity_name_edit: "".to_string(),
            edit_peer_id_index: "0".to_string(),
            edit_xpub_name: "".to_string(),
            peer_tx: None,
        }
    }
}

pub fn identity_tab(ui: &mut Ui, ctx: &Context, ls: &mut LocalState) {
    ui.heading("Identity");
    // ui.label("WIP TAB -- not yet implemented, for now update json config manually in config tab");

    ComboBox::from_label("Choose Identity")
        .selected_text(ls.identity_state.selected_name.clone())
        .show_ui(ui, |ui| {
            for style in ls.local_stored_state.identities.iter().map(|x| x.name.clone()) {
                ui.selectable_value(&mut ls.identity_state.selected_name, style.clone(), style.to_string());
            }
        });
    if ls.identity_state.last_selected_name != ls.identity_state.selected_name {
        ls.local_stored_state.identities.iter().find(|p| p.name == ls.identity_state.selected_name).map(|x| {
            ls.identity_state.selected_identity = Some(x.clone());
        });
        ls.identity_state.last_selected_name = ls.identity_state.selected_name.clone();
        if let Some(is) = &ls.identity_state.selected_identity {
            ls.identity_state.identity_name_edit = is.name.clone();
            ls.identity_state.edit_peer_id_index = is.peer_id_index.to_string();
            ls.identity_state.edit_xpub_name = is.xpub_name.clone();
        }
    }
    editable_text_input_copy(ui, "Identity Name: ", &mut ls.identity_state.identity_name_edit, 200.0);
    editable_text_input_copy(ui, "Peer Id Index: ", &mut ls.identity_state.edit_peer_id_index, 200.0);
    editable_text_input_copy(ui, "Xpub name: ", &mut ls.identity_state.edit_xpub_name, 200.0);

    if ui.button("Save Identity").clicked() {
        let result = ls.identity_state.edit_peer_id_index.parse::<i64>();
        if let Some(idx) = result.ok() {
            let i = Identity {
                name: ls.identity_state.identity_name_edit.clone(),
                peer_id_index: idx,
                xpub_name: ls.identity_state.edit_xpub_name.clone(),
            };
            ls.upsert_identity(i);
        }
    }

    if ui.button("Request Peer Tx").clicked() {
        // TODO: implement
    };
}
