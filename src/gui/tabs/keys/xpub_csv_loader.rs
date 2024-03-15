use eframe::egui;
use eframe::egui::{ScrollArea, Ui, Widget};
use tracing::info;
use itertools::Itertools;
use redgold_schema::local_stored_state::NamedXpub;
use redgold_schema::{EasyJson, ErrorInfoContext, RgResult};
use crate::gui::app_loop::LocalState;
use crate::gui::tabs::transact::wallet_tab;


fn parse_xpub_rows(str: &str) -> RgResult<Vec<NamedXpub>> {
    let mut rdr = csv::Reader::from_reader(str.as_bytes());
    let mut res = vec![];
    for result in rdr.deserialize() {
        // Notice that we need to provide a type hint for automatic
        // deserialization.
        let record: NamedXpub = result.error_info("server line parse failure")?;
        res.push(record);
    }
    Ok(res)
}


// TODO: This window exceeds the max size bound for some crazy reason??
pub fn window_xpub_loader(
    _ui: &mut Ui,
    ls: &mut LocalState,
    ctx: &egui::Context,
) {
    egui::Window::new("Xpub Loader")
        .open(&mut ls.wallet_state.show_xpub_loader_window)
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
                    egui::TextEdit::multiline(&mut ls.wallet_state.xpub_loader_rows)
                        .desired_rows(3)
                        .desired_width(200.0)
                        .ui(ui);
                });

                ui.checkbox(&mut ls.wallet_state.purge_existing_xpubs_on_save, "Purge all existing xpubs on load");
                ui.checkbox(&mut ls.wallet_state.allow_xpub_name_overwrite, "Allow overwrite of xpub by name");
                ui.label(ls.wallet_state.xpub_loader_error_message.clone());
                if ui.button("Save Internal").clicked() {
                    let data = ls.wallet_state.xpub_loader_rows.clone();
                    let parsed = parse_xpub_rows(&*data).ok();
                    if let Some(rows) = parsed {
                        LocalState::send_update(&ls.updates, move |lss| {
                            let rows2 = rows.clone();
                            info!("Parsed Xpub rows: {:?}", rows2.json_or());
                            let names = rows2.iter().map(|n| n.name.clone()).collect_vec();
                            let has_existing = lss.local_stored_state.xpubs.iter().find(|n| names.contains(&n.name)).is_some();
                            if has_existing && !lss.wallet_state.allow_xpub_name_overwrite {
                                lss.wallet_state.xpub_loader_error_message = "Existing xpubs found, please enable overwrite".to_string();
                            } else {
                                if lss.wallet_state.purge_existing_xpubs_on_save {
                                    lss.local_stored_state.xpubs = vec![];
                                }
                                // TODO: Render error msg
                                lss.add_named_xpubs(lss.wallet_state.allow_xpub_name_overwrite, rows2, false).ok();
                                lss.wallet_state.show_xpub_loader_window = false;
                            }
                        });
                    } else {
                        ls.wallet_state.xpub_loader_error_message = "Failed to parse rows".to_string();
                    }
                }
            });
        });
}
