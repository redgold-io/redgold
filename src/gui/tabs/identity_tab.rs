use eframe::egui::{ComboBox, Context, Ui};
use redgold_schema::local_stored_state::NamedXpub;
use redgold_schema::structs::PublicKey;
use crate::gui::app_loop::LocalState;


#[derive(Clone)]
pub struct IdentityState {
    selected: String,
    name: String,
    peer_id_index: i64,
    xpubs: Vec<String>,
}

impl IdentityState {
    pub(crate) fn new() -> IdentityState {
        Self {
            selected: "Add New Identity".to_string(),
            name: "".to_string(),
            peer_id_index: 0,
            xpubs: vec![],
        }
    }
}

pub fn identity_tab(ui: &mut Ui, ctx: &Context, ls: &mut LocalState) {
    ui.heading("Identity");
    ui.label("WIP TAB -- not yet implemented");

    ComboBox::from_label("Select Identity")
        .selected_text(ls.identity_state.selected.clone())
        .show_ui(ui, |ui| {
            for style in ls.local_stored_state.identities.iter().map(|x| x.name.clone()) {
                ui.selectable_value(&mut ls.identity_state.selected, style.clone(), style.to_string());
            }
        });

    if ui.button("Request Peer Tx").clicked() {

    }
}
