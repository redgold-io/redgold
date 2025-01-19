use eframe::egui::{Color32, RichText, Ui};
use redgold_gui::dependencies::gui_depends::GuiDepends;
use redgold_gui::tab::transact::wallet_state::WalletState;

pub fn hardware_connected<G>(ui: &mut Ui, state: &mut WalletState, g: &G) where G: GuiDepends + Clone + Send + 'static + Sync {
    state.update_hardware(g);
    ui.horizontal(|ui| {
        ui.label("Hardware Wallet: ");
        let connected = state.device_list_status.device_output.is_some();
        if connected {
            ui.label(RichText::new("Connected").color(Color32::GREEN));
        } else {
            ui.label(RichText::new("Not Connected").color(Color32::RED));
        }
    });
    // ui.spacing();
    ui.label(state.device_list_status.device_output.clone().unwrap_or("".to_string()));
}
