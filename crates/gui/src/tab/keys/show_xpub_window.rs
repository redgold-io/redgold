use crate::state::local_state::LocalState;
use eframe::egui;
use eframe::egui::Context;
use redgold_common::external_resources::ExternalNetworkResources;
use crate::common::{bounded_text_area_size, copy_to_clipboard, medium_data_item};
use redgold_schema::conf::local_stored_state::AccountKeySource;
use crate::dependencies::gui_depends::GuiDepends;

pub fn show_xpub_window<E, G>(
    ctx: &Context, ls: &mut LocalState<E>, xpub: AccountKeySource, g: &G
) where E: ExternalNetworkResources + 'static + Sync + Send + Clone, G: GuiDepends {

    egui::Window::new("XPub")
        .open(&mut ls.keytab_state.show_xpub)
        .resizable(false)
        .collapsible(false)
        .min_width(300.0)
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                medium_data_item(ui, "Name", xpub.name.clone());
                medium_data_item(ui, "Derivation Path", G::as_account_path(xpub.derivation_path).unwrap_or("".to_string()));
                if let Some(ho) = xpub.hot_offset {
                    medium_data_item(ui, "Hot Offset", ho);
                }
                if let Some(a) = xpub.request_type {
                    medium_data_item(ui, "Request Type", format!("{:?}", a));
                }
                if let Some(k) = xpub.key_name_source {
                    medium_data_item(ui, "Key Name Source", k);
                }
                let mut string = xpub.xpub.clone();
                bounded_text_area_size(ui, &mut string, 300.0, 4);
                copy_to_clipboard(ui, xpub.xpub.clone());

            });
        });
}
