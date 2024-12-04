use eframe::egui;
use eframe::egui::Context;
use redgold_keys::xpub_wrapper::ValidateDerivationPath;
use redgold_schema::conf::local_stored_state::AccountKeySource;
use crate::gui::app_loop::LocalState;
use redgold_gui::common::{bounded_text_area_size, copy_to_clipboard, medium_data_item};

pub(crate) fn show_xpub_window(
    ctx: &Context, ls: &mut LocalState, xpub: AccountKeySource
) {

    egui::Window::new("XPub")
        .open(&mut ls.keytab_state.show_xpub)
        .resizable(false)
        .collapsible(false)
        .min_width(300.0)
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                medium_data_item(ui, "Name", xpub.name.clone());
                medium_data_item(ui, "Derivation Path", xpub.derivation_path.as_account_path().expect("acc"));
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
