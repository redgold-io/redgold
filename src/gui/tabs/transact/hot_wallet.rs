use crate::gui::app_loop::LocalState;
use crate::gui::components::key_source_sel::key_source;
use crate::gui::components::save_key_window;
use eframe::egui;
use eframe::egui::Ui;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_gui::dependencies::gui_depends::GuiDepends;
use redgold_gui::tab::transact::wallet_state::WalletState;


pub fn hot_header<E, G>(ls: &mut LocalState<E>, ui: &mut Ui, _ctx: &egui::Context, g: &G
) where E: ExternalNetworkResources + 'static + Sync + Send + Clone, G: GuiDepends + 'static + Sync + Send + Clone {

    save_key_window::save_key_window(ui, ls, _ctx);

    key_source(ui, ls, g);

    let state = &mut ls.wallet;

    let check = state.mnemonic_or_key_checksum.clone();
    ui.label(format!("Hot Wallet Checksum: {check}"));

    if state.public_key.is_none() {
        state.update_hot_mnemonic_or_key_info(g);
    }

}


pub(crate) fn init_state(state: &mut WalletState) {
    // TODO: From constant or function for account zero.
    state.derivation_path = "m/44'/16180'/0'/0/0".to_string();
    state.xpub_derivation_path = "m/44'/16180'/0'".to_string();
    state.public_key = None;
}