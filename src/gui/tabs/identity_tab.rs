use eframe::egui::{ComboBox, Context, Ui};
use redgold_schema::local_stored_state::{Identity, NamedXpub};
use redgold_schema::structs::{PublicKey, Transaction};
use crate::gui::app_loop::LocalState;
use crate::gui::common::editable_text_input_copy;


#[derive(Clone)]
pub struct IdentityState {
    selected_name: String,
    selected_identity: Option<Identity>,
    identity_name_edit: String,
    peer_tx: Option<Transaction>
}

impl IdentityState {
    pub(crate) fn new() -> IdentityState {
        Self {
            selected_name: "Add New Identity".to_string(),
            selected_identity: None,
            identity_name_edit: "".to_string(),
            peer_tx: None,
        }
    }
}

pub fn identity_tab(ui: &mut Ui, ctx: &Context, ls: &mut LocalState) {
    ui.heading("Identity");
    ui.label("WIP TAB -- not yet implemented");

    ComboBox::from_label("Select Identity")
        .selected_text(ls.identity_state.selected_name.clone())
        .show_ui(ui, |ui| {
            for style in ls.local_stored_state.identities.iter().map(|x| x.name.clone()) {
                ui.selectable_value(&mut ls.identity_state.selected_name, style.clone(), style.to_string());
            }
        });

    ls.local_stored_state.identities.iter().find(|p| p.name == ls.identity_state.selected_name).map(|x| {
        ls.identity_state.selected_identity = Some(x.clone());
    });
    //
    // match ls.identity_state.selected_identity {
    //     None => {
    //
    //     }
    //     Some(ident) => {
    //
    //     }
    // }
    //
    // editable_text_input_copy(ui, "Identity Name: ", &mut ls.identity_state.selected_name, 200.0);")
    //
    // if ui.button("Request Peer Tx").clicked() {
    //
    // }
}
