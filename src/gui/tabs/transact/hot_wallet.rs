use eframe::egui;
use eframe::egui::Ui;
use crate::gui::app_loop::LocalState;
use crate::gui::components::key_source_sel::key_source;
use crate::gui::components::save_key_window;
use crate::gui::tabs::transact::wallet_tab::WalletState;


pub fn hot_header(ls: &mut LocalState, ui: &mut Ui, _ctx: &egui::Context) {

    save_key_window::save_key_window(ui, ls, _ctx);

    key_source(ui, ls);

    let state = &mut ls.wallet_state;

    let check = state.mnemonic_or_key_checksum.clone();
    ui.label(format!("Hot Wallet Checksum: {check}"));

    if state.public_key.is_none() {
        state.update_hot_mnemonic_or_key_info();
    }

}


pub(crate) fn init_state(state: &mut WalletState) {
    // TODO: From constant or function for account zero.
    state.derivation_path = "m/44'/16180'/0'/0/0".to_string();
    state.xpub_derivation_path = "m/44'/16180'/0'".to_string();
    state.public_key = None;
}