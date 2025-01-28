use crate::gui::app_loop::{LocalState, LocalStateAddons};

use crate::gui::tabs::transact::wallet_tab;
use eframe::egui;
use eframe::egui::{ScrollArea, Ui, Widget};
use itertools::Itertools;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::conf::local_stored_state::AccountKeySource;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::{ErrorInfoContext, RgResult};
use tracing::info;

fn parse_xpub_rows(str: &str) -> RgResult<Vec<AccountKeySource>> {
    let mut rdr = csv::Reader::from_reader(str.as_bytes());
    let mut res = vec![];
    for result in rdr.deserialize() {
        // Notice that we need to provide a type hint for automatic
        // deserialization.
        let record: AccountKeySource = result.error_info("server line parse failure")?;
        res.push(record);
    }
    Ok(res)
}


// TODO: This window exceeds the max size bound for some crazy reason??
pub fn window_xpub_loader<E>(
    _ui: &mut Ui,
    ls: &mut LocalState<E>,
    ctx: &egui::Context,
) where E: ExternalNetworkResources + 'static + Sync + Send + Clone {
    let mut show = ls.wallet.show_xpub_loader_window;
    let mut hide = false;
    egui::Window::new("Xpub Loader")
        .open(&mut show)
        .resizable(false)
        .collapsible(false)
        .min_width(300.0)
        .default_width(300.0)
        .constrain(true)
        .fixed_size((300.0, 300.0))
        .show(ctx, |ui| {
            // Layout doesn't seem to work here.
            // let layout = egui::Layout::top_down(egui::Align::Center);
            // ui.with_layout(layout, |ui| {
            ui.vertical(|ui| {
                ui.label("Enter CSV data with format name,derivation_path,xpub");

                ScrollArea::vertical().show(ui, |ui| {
                    egui::TextEdit::multiline(&mut ls.wallet.xpub_loader_rows)
                        .desired_rows(3)
                        .desired_width(200.0)
                        .ui(ui);
                });

                ui.checkbox(&mut ls.wallet.purge_existing_xpubs_on_save, "Purge all existing xpubs on load");
                ui.checkbox(&mut ls.wallet.allow_xpub_name_overwrite, "Allow overwrite of xpub by name");
                ui.label(ls.wallet.xpub_loader_error_message.clone());
                if ui.button("Save Internal").clicked() {
                    let data = ls.wallet.xpub_loader_rows.clone();
                    let parsed = parse_xpub_rows(&*data).ok();
                    if let Some(rows) = parsed {
                        // send_update(&ls.updates, move |lss| {
                            let rows2 = rows.clone();
                            info!("Parsed Xpub rows: {:?}", rows2.json_or());
                            let names = rows2.iter().map(|n| n.name.clone()).collect_vec();
                            let has_existing = ls.local_stored_state.keys
                                .as_ref()
                                .map(|x| x.iter().find(|n| names.contains(&n.name)).is_some()).unwrap_or(false);
                            if has_existing && !ls.wallet.allow_xpub_name_overwrite {
                                ls.wallet.xpub_loader_error_message = "Existing xpubs found, please enable overwrite".to_string();
                            } else {
                                if ls.wallet.purge_existing_xpubs_on_save {
                                    ls.local_stored_state.keys = Some(vec![]);
                                }
                                // TODO: Render error msg
                                ls.add_named_xpubs(ls.wallet.allow_xpub_name_overwrite, rows2, false).ok();
                                hide = true;
                            }
                        // });
                    } else {
                        ls.wallet.xpub_loader_error_message = "Failed to parse rows".to_string();
                    }
                }
            });
        });
    ls.wallet.show_xpub_loader_window = show;
    if hide {
        ls.wallet.show_xpub_loader_window = false;
    }
}
