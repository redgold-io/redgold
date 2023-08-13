use eframe::egui;
use eframe::egui::Ui;
use itertools::Itertools;
use redgold_keys::util::mnemonic_support::WordsPass;
use crate::gui::app_loop::LocalState;
use crate::gui::common::{data_item, editable_text_input_copy, valid_label};
use crate::gui::wallet_tab::WalletState;




pub fn hot_header(state: &mut WalletState, ui: &mut Ui, _ctx: &egui::Context) {
    let check = state.mnemonic_checksum.clone();
    ui.label(format!("Hot Wallet Checksum: {check}"));

    if state.public_key.is_none() {
        let m = state.hot_mnemonic();
        let check = m.checksum_words().unwrap_or("".to_string());
        // derivation_path_section(ui, &state, m);
        let pk = m.public_at(state.derivation_path.clone());
        state.public_key = pk.ok();
        state.mnemonic_checksum = check;
    }

}


pub(crate) fn init_state(state: &mut WalletState) {
    // TODO: From constant or function for account zero.
    state.derivation_path = "m/44'/16180'/0'/0/0".to_string();
    state.public_key = None;
}