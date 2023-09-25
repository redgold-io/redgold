use eframe::egui::{Color32, RichText, Ui};
use crate::gui::wallet_tab::WalletState;

pub fn cold_header(ui: &mut Ui, state: &mut WalletState) {
    state.update_hardware();
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
