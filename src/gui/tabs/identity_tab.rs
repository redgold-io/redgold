use std::collections::HashMap;
use eframe::egui::{ComboBox, Context, Ui};
use itertools::Itertools;
use redgold_schema::tx::tx_builder::TransactionBuilderSupport;
use redgold_schema::{error_info, RgResult, SafeOption};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::local_stored_state::Identity;
use redgold_schema::servers::ServerOldFormat;
use redgold_schema::structs::{PeerMetadata, Transaction};
use redgold_schema::tx::tx_builder::TransactionBuilder;
use crate::gui::app_loop::{LocalState, PublicKeyStoredState};
use redgold_gui::common::{bounded_text_area, bounded_text_area_size, editable_text_input_copy};
use crate::gui::tabs::transact::wallet_tab::StateUpdate;
use crate::node_config::ApiNodeConfig;

#[derive(Clone)]
pub struct IdentityState {
    selected_name: String,
    last_selected_name: String,
    selected_identity: Option<Identity>,
    identity_name_edit: String,
    edit_peer_id_index: String,
    edit_xpub_name: String,
    peer_tx: Option<Transaction>,
    updated_peer_tx: Option<Transaction>,
    peer_request_status: Option<String>,
    peer_generate_status: Option<String>
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
            updated_peer_tx: None,
            peer_request_status: None,
            peer_generate_status: None,
        }
    }
}

pub fn identity_tab(ui: &mut Ui, _ctx: &Context, ls: &mut LocalState) {
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
            ls.upsert_identity(i.clone());
            ls.identity_state.selected_identity = Some(i)
        }
    }

    if ui.button("Request Peer Tx").clicked() {
        let id = &ls.identity_state.selected_identity;
        let pk = id.as_ref()
            .and_then(|i| ls.local_stored_state.public_key(i.xpub_name.clone()));
        let u = ls.updates.sender.clone();
        let c = ls.node_config.api_client();
        tokio::spawn(async move {
            // TODO: Replace this with get latest transaction matching pk address.
            let option = c.client_wrapper().get_peers().await.ok();
            let tx = option.as_ref()
                .and_then(|r| r.get_peers_info_response.as_ref())
                .and_then(|r| r.peer_info.iter().find(|p|
                    pk == p.latest_peer_transaction.as_ref()
                        .and_then(|p| p.peer_data().ok())
                        .and_then(|p| p.peer_id)
                        .and_then(|p| p.peer_id)
                )).and_then(|p| p.latest_peer_transaction.clone());
            u.send(StateUpdate{update: Box::new(move |ls2: &mut LocalState| {
                if tx.is_none() {
                    ls2.identity_state.peer_request_status = Some("No peer tx found".to_string());
                }
                ls2.identity_state.peer_tx = tx.clone();
            })}).unwrap();
        });
    };

    if let Some(tx) = &ls.identity_state.peer_request_status {
        ui.label(tx.clone());
    }

    if let Some(peer_tx_existing) = &ls.identity_state.peer_tx {
        ui.label("Existing Peer Transaction:");
        bounded_text_area(ui, &mut peer_tx_existing.json_or());
    }

    if ui.button("Generate New Peer Tx").clicked() {
        let res = generate_peer_tx(ls);
        if let Err(e) = res {
            ls.identity_state.peer_generate_status = Some(e.json_or());
        }
    }

    if let Some(status) = &ls.identity_state.peer_generate_status {
        ui.label(status.clone());
    }

    if let Some(tx) = &ls.identity_state.updated_peer_tx {
        ui.label("Updated Peer Transaction:");
        ui.horizontal(|ui|
        bounded_text_area_size(ui, &mut tx.json_or(), 600.0, 3));
    }

}

fn generate_peer_tx(ls: &mut LocalState) -> RgResult<()> {
    let tx = ls.identity_state.peer_tx.as_ref().ok_or(error_info("No peer tx"))?;
    let i = ls.identity_state.selected_identity.as_ref()
        .ok_or(error_info("No identity"))?;
    let p = ls.local_stored_state.public_key(i.xpub_name.clone())
        .ok_or(error_info("No public key for xpub"))?;

    let mut tb = TransactionBuilder::new(&ls.node_config);
    let mut pkmap = HashMap::default();
    pkmap.insert(i.peer_id_index, p);
    let s = ls.local_stored_state.servers.iter()
        .filter(|s| s.peer_id_index == i.peer_id_index)
        .map(|c| c.clone())
        .collect_vec();
    let mut peer_data = PeerMetadata::default();
    ServerOldFormat::peer_data(
        s,
        &mut peer_data,
        i.peer_id_index,
        pkmap,
        ls.node_config.executable_checksum.clone().expect("exe"),
        ls.node_config.network.clone(),
        None
    );
    let t = ls.local_stored_state.trust
        .iter().filter(|p| p.peer_id_index == i.peer_id_index)
        .map(|p| p.labels.clone())
        .flatten()
        .collect_vec();
    peer_data.labels = t;
    let utxo = tx.first_peer_utxo()?;
    let o = utxo.output.safe_get_msg("Missing utxo Output")?;
    let d = o.data.safe_get_msg("Missing data")?;
    let h = tx.height().ok().or(d.height).safe_get_msg("Missing height")?.clone();
    tb.with_unsigned_input(utxo.clone()).expect("");
    tb.with_output_peer_data(
        &utxo.address().expect(""),
        peer_data,
        h + 1
    );
    ls.identity_state.updated_peer_tx = Some(tb.transaction.clone());
    ls.identity_state.peer_generate_status = Some("Generated Success".to_string());
    Ok(())
}
