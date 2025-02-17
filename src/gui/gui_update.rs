
use crate::gui::{top_panel, ClientApp};
use eframe::egui;
use eframe::egui::{Align, ScrollArea, TextStyle};
use itertools::Itertools;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_gui::data_query::data_query::DataQueryInfo;
use redgold_gui::dependencies::extract_public::ExtractorPublicKey;
use redgold_gui::dependencies::gui_depends::{GuiDepends, MnemonicWordsAndPassphrasePath, TransactionSignInfo};
use redgold_gui::state::local_state::{LocalStateAddons, LocalStateUpdate};
use redgold_gui::tab::tabs::Tab;
use redgold_gui::tab::transact::swap::SwapStage;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::structs::SupportedCurrency;
use std::sync::Once;
use strum::IntoEnumIterator;
use redgold_gui::tab::keys::keymanage::keys_tab;
use crate::gui::tabs::server_tab;
use crate::gui::tabs::transact::wallet_tab::wallet_screen;
use crate::util;

static INIT: Once = Once::new();

pub fn app_update<G, E>(app: &mut ClientApp<G, E>, ctx: &egui::Context, _frame: &mut eframe::Frame)
where G: GuiDepends + Clone + Send + 'static + Sync, E: ExternalNetworkResources + Send + Sync + 'static + Clone {

    let g = &mut app.gui_depends;
    let local_state = &mut app.local_state;


    let top_event = top_panel::render_top(ctx, local_state);


    let mut first_call = false;
    // TODO: Replace with config query and check.
    INIT.call_once(|| {
        first_call = true;

        let amt = if local_state.is_mac {
            2.5
        } else if local_state.is_linux {
            1.8
        } else {
            2.0
        };
        ctx.set_pixels_per_point(amt);
    });

    if first_call || top_event.refresh_all_clicked {
        g.initial_queries_prices_parties_etc(
            local_state.local_messages.sender.clone(),
            local_state.external_network_resources.clone()
        );
    }

    if local_state.persist_requested {
        let mut c = g.get_config();
        c.local = Some(local_state.local_stored_state.clone());
        g.set_config(&c, false);
        local_state.persist_requested = false;
    }
    local_state.external_network_resources.set_network(&local_state.node_config.network);
    g.set_network(&local_state.node_config.network);
    if local_state.data.get_mut(&local_state.node_config.network).is_none() {
        let d = DataQueryInfo::new(&local_state.external_network_resources);
        local_state.data.insert(local_state.node_config.network.clone(), d);
        g.initial_queries_prices_parties_etc(
            local_state.local_messages.sender.clone(),
            local_state.external_network_resources.clone()
        );
    }

    let updates = local_state.local_messages.recv_while().unwrap_or_default();
    local_state.latest_local_messages = updates.clone();
    for update in updates {
        match update {
            LocalStateUpdate::PricesPartyInfoAndDelta(p) => {
                if let Some(d) = local_state.data.get_mut(&p.on_network) {
                    d.load_party_data_and_prices(p);
                } else {
                    let mut d = DataQueryInfo::new(&local_state.external_network_resources);
                    d.load_party_data_and_prices(p.clone());
                    local_state.data.insert(p.on_network.clone(), d);
                }

            }
            LocalStateUpdate::BalanceUpdates(b) => {
                local_state.wallet.address_info = b.address_info.clone();
                local_state.wallet.balance_f64 = b.balances.get(&SupportedCurrency::Redgold).cloned();
                local_state.wallet.balance = b.balances.get(&SupportedCurrency::Redgold).cloned().map(|b| b.to_string())
                    .unwrap_or_else(|| "".to_string());
                local_state.wallet.balance_btc_f64 = b.balances.get(&SupportedCurrency::Bitcoin).cloned();
                local_state.wallet.balance_btc = b.balances.get(&SupportedCurrency::Bitcoin).cloned().map(|b| b.to_string());
                local_state.wallet.balance_eth_f64 = b.balances.get(&SupportedCurrency::Ethereum).cloned();
                local_state.wallet.balance_eth = b.balances.get(&SupportedCurrency::Ethereum).cloned().map(|b| b.to_string());

            }
            LocalStateUpdate::SwapResult(res) => {
                // send_update_sender(&ups, move |lss| {
                    let (err, tx) = match &res {
                        Ok(tx) => (None, Some(tx)),
                        Err(e) => (Some(e.json_or()), None)
                    };
                    if err.is_none() {
                        local_state.swap_state.stage = SwapStage::ShowAmountsPromptSigning;
                    }
                local_state.swap_state.tx_progress.created(tx.cloned(), err);
                local_state.swap_state.changing_stages = false;
                // });
            }
            LocalStateUpdate::RequestHardwareRefresh => {
                local_state.wallet.update_hardware(g);
            }
            _ => {}
        }
    }

    local_state.current_time = util::current_time_millis_i64();
    // Continuous mode
    ctx.request_repaint();

    // local_state.process_updates();

    // let mut style: egui::Style = (*ctx.style()).clone();
    // style.visuals.widgets.
    //style.spacing.item_spacing = egui::vec2(10.0, 20.0);
    // ctx.set_style(style);
    // Examples of how to create different panels and windows.
    // Pick whichever suits you.
    // Tip: a good default choice is to just keep the `CentralPanel`.
    // For inspiration and more examples, go to https://emilk.github.io/egui



    // let img = logo;
    // let texture_id = img.texture_id(ctx);

    let mut changed_tab: Option<Tab> = None;

    egui::SidePanel::left("side_panel")
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_max_width(54f32);

            ui.with_layout(
                egui::Layout::top_down_justified(egui::Align::default()),
                |ui| {
                    ui.add_space(10f32);
                    // ui.image(texture_id); //, size);
                    let image = egui::Image::new(egui::include_image!("../resources/images/historical/design_one/logo_orig_crop.png"));
                    // image.load_for_size(ctx, size).expect("works");
                    ui.add(
                        image
                        // egui::Image::new("https://picsum.photos/seed/1.759706314/1024").rounding(10.0),
                    );

                    ui.style_mut().override_text_style = Some(TextStyle::Heading);

                    ui.style_mut().spacing.item_spacing.y = 5f32;
                    ui.add_space(10f32);
                    for tab_i in Tab::iter() {
                        // if !local_state.node_config.development_mode() {
                            if vec![
                                Tab::Contacts,
                                Tab::Ratings, Tab::Identity, Tab::OTP, Tab::Airgap].contains(&tab_i) {
                                continue;
                            }
                        // }
                        let tab_str = format!("{:?}", tab_i);
                        if ui.button(tab_str).clicked() {
                            local_state.active_tab = tab_i.clone();
                            local_state.process_tab_change(tab_i.clone());
                            changed_tab = Some(tab_i.clone());
                        }
                    }
                },
            );
        });

    let has_changed_tab = changed_tab.is_some();

    egui::CentralPanel::default().show(ctx, |ui| {
        // The central panel the region left after adding TopPanel's and SidePanel's
        match local_state.active_tab {
            Tab::Home => {
                let pks = local_state.local_stored_state.extract(g);
                local_state.home_state.home_screen(
                    ui, ctx, g, &local_state.external_network_resources, &local_state.data.get(&local_state.node_config.network).unwrap(),
                    &local_state.node_config, pks.iter().collect_vec(), local_state.current_time,
                    &local_state.local_stored_state
                );
            }
            Tab::Keys => {
                keys_tab(ui, ctx, local_state, has_changed_tab, g);
            }
            Tab::Settings => {
                let delta = local_state.settings_state.settings_tab(ui, ctx, g);
                if let Some(d) = delta.updated_config {
                    g.set_config(&d, delta.overwrite_all);
                    if let Some(c) = d.local.as_ref() {
                        local_state.local_stored_state = c.clone();
                    }

                }
            }
            Tab::Ratings => {}
            Tab::Deploy => {
                ScrollArea::vertical().id_source("wtf").show(ui, |ui| {
                    server_tab::servers_tab(
                        ui,
                        ctx,
                        &mut local_state.server_state,
                        g,
                        &local_state.node_config,
                        local_state.wallet.hot_mnemonic(g).words,
                        local_state.wallet.hot_mnemonic(g).passphrase,
                    );
                });
            }
            Tab::Transact => {
                let d = local_state.data.get(&local_state.node_config.network).unwrap().clone();
                wallet_screen(ui, ctx, local_state, has_changed_tab, g, &d);
            }
            Tab::Identity => {
                // crate::gui::tabs::identity_tab::identity_tab(ui, ctx, local_state);
            }
            Tab::Address => {
                let updated = local_state.address_state.address_tab(ui, ctx, g);
                if let Some(a) = updated.add_new_address {
                    local_state.local_stored_state.saved_addresses.get_or_insert_default().push(a);
                    local_state.persist_local_state_store()
                }
                if let Some(a) = updated.delete_address {
                    local_state.local_stored_state.saved_addresses.get_or_insert_default()
                        .retain(|x| x.address != a.address);
                    local_state.persist_local_state_store()
                }
            },
            Tab::OTP => {
                // otp_tab(ui, ctx, local_state);
            }
            Tab::Airgap => {
                // local_state.wallet.active_hot_mnemonic
                let x = MnemonicWordsAndPassphrasePath{
                    words: local_state.keygen_state.mnemonic_window_state.words.clone(),
                    passphrase: local_state.keygen_state.mnemonic_window_state.passphrase.clone(),
                    path: None,
                };
                let info = TransactionSignInfo::Mnemonic(x);
                local_state.airgap_signer.interior_view(ui, g, Some(&info));
            }
            Tab::Portfolio => {
                local_state.portfolio_tab_state.view(ui, &local_state.data.get(&local_state.node_config.network).unwrap(), local_state.node_config.network.clone());
            }
            _ => {}
        }
        ui.with_layout(egui::Layout::top_down(Align::BOTTOM), |ui| {
            egui::warn_if_debug_build(ui)
        });
    });

    // qr_window(ctx, local_state);
    // qr_show_window(ctx, local_state);

}