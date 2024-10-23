use eframe::egui::{Color32, RichText, Ui};
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::conf::local_stored_state::XPubLikeRequestType;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::structs::{PublicKey, Transaction};
use crate::common::bounded_text_area;
use crate::components::tx_progress::TransactionProgressFlow;
use crate::data_query::data_query::DataQueryInfo;
use crate::dependencies::gui_depends::{GuiDepends, TransactionSignInfo};

#[derive(Clone)]
pub struct CustomTxState {
    tx: TransactionProgressFlow,
    input: String,
    valid: bool,
    invalidation_reason: String,
    tx_input: Option<Transaction>
}

impl Default for CustomTxState {
    fn default() -> Self {
        Self {
            tx: TransactionProgressFlow::default(),
            input: "".to_string(),
            valid: false,
            invalidation_reason: "".to_string(),
            tx_input: None,
        }
    }
}

impl CustomTxState {
    pub fn view<E, G>(&mut self, ui: &mut Ui, g: &G, tsi: &TransactionSignInfo, csi: &TransactionSignInfo, allowed: &Vec<XPubLikeRequestType>)
    where
        E: ExternalNetworkResources + Clone + Send + 'static,
        G: GuiDepends + Clone + Send + 'static
    {
        ui.label("Enter custom transaction JSON:");

        if !self.tx.locked() {
            let changed = bounded_text_area(ui, &mut self.input);
            if changed {
                let tx = self.input.json_from::<Transaction>();
                match tx {
                    Ok(tx) => {self.tx_input = Some(tx); self.valid = true; self.invalidation_reason = "".to_string();}
                    Err(e) => {
                        self.valid = false;
                        self.invalidation_reason = e.json_or();
                    }
                }
            }
        }

        if self.valid {
            self.tx.info_box_view(ui, allowed);
            let ev = self.tx.progress_buttons(ui, g, tsi, csi);
            if ev.next_stage_create {
                if let Some(txx) = self.tx_input.clone() {
                    self.tx.with_built_rdg_tx(Ok(txx));
                }
            }
        } else {
            ui.label(RichText::new(format!("Invalid transaction: {}", self.invalidation_reason)).color(Color32::RED));
        }
    }
}