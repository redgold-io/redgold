use std::future::Future;
use std::time::Instant;
use bdk::bitcoin::bech32::ToBase32;
use eframe::egui;
use eframe::egui::{Color32, ComboBox, Context, RichText, ScrollArea, TextStyle, Ui, Widget};
use flume::Sender;
use itertools::{Either, Itertools};
use rocket::form::validate::Contains;
use serde::Deserialize;
use crate::gui::app_loop::{LocalState, LocalStateAddons};

use strum::IntoEnumIterator;
// 0.17.1
use strum_macros::{EnumIter, EnumString};
use tracing::{error, info};
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_common::flume_send_help::{new_channel, Channel, RecvAsyncErrorInfo, SendErrorInfo};
use redgold_keys::address_external::{ToBitcoinAddress, ToEthereumAddress};
use redgold_keys::{KeyPair, TestConstants};
use redgold_keys::address_support::AddressSupport;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::util::btc_wallet::SingleKeyBitcoinWallet;
use redgold_schema::{error_info, ErrorInfoContext, RgResult, SafeOption};
use redgold_schema::structs::{Address, AddressInfo, CurrencyAmount, ErrorInfo, Hash, NetworkEnvironment, PublicKey, SubmitTransactionResponse, SupportedCurrency, Transaction};
use crate::hardware::trezor;
use crate::hardware::trezor::trezor_list_devices;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::transaction::rounded_balance_i64;
use redgold_schema::tx::tx_builder::TransactionBuilder;
use redgold_schema::keys::words_pass::WordsPass;
use redgold_keys::xpub_wrapper::{ValidateDerivationPath, XpubWrapper};
use redgold_schema::helpers::easy_json::EasyJsonDeser;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_gui::common;
use redgold_gui::common::{bounded_text_area, data_item, data_item_multiline_fixed, editable_text_input_copy, medium_data_item, valid_label};
use redgold_gui::components::address_input_box::AddressInputBox;
use redgold_gui::components::balance_table::balance_table;
use redgold_gui::components::currency_input::CurrencyInputBox;
use redgold_gui::dependencies::gui_depends::{GuiDepends, TransactionSignInfo};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::util::lang_util::JsonCombineResult;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::conf::local_stored_state::{AccountKeySource, StoredMnemonic, StoredPrivateKey, XPubLikeRequestType};
use redgold_schema::proto_serde::ProtoSerde;
use redgold_gui::components::passphrase_input::PassphraseInput;
use redgold_gui::components::transaction_table::TransactionTable;
use redgold_gui::components::tx_progress::{PreparedTransaction, TransactionProgressFlow, TransactionStage};
use redgold_gui::data_query::data_query::DataQueryInfo;
use redgold_gui::state::lss_addon::LssAddon;
use redgold_gui::tab::custom_tx::CustomTxState;
use redgold_gui::tab::receive::ReceiveData;
use redgold_gui::tab::stake::StakeState;
use redgold_gui::tab::transact::portfolio_transact::{PortfolioState, PortfolioTransactSubTab};
use redgold_gui::tab::transact::states::{DeviceListStatus, SendReceiveTabs, WalletTab};
use redgold_keys::util::mnemonic_support::MnemonicSupport;
use crate::gui::components::explorer_links::rdg_explorer;
use crate::gui::components::xpub_req;
use crate::gui::ls_ext::create_swap_tx;
use crate::gui::tabs::keys::keys_tab::internal_stored_xpubs;
use crate::gui::tabs::transact::{address_query, broadcast_tx, cold_wallet, hardware_signing, hot_wallet, portfolio_transact, prepare_tx, prepared_tx_view};
use crate::integrations::external_network_resources::ExternalNetworkResourcesImpl;


pub trait DeviceListTrezorNative {
    fn poll() -> Self;
}

impl DeviceListTrezorNative for DeviceListStatus {
    fn poll() -> Self {
        let result = trezor_list_devices().ok().flatten();
        Self {
            device_output: result,
            last_polled: Instant::now(),
        }
    }
}


pub fn wallet_screen<G, E>(ui: &mut Ui, ctx: &egui::Context, local_state: &mut LocalState<E>, has_changed_tab: bool, g: &mut G, d: &DataQueryInfo<E>)
    where G: GuiDepends + Clone + Send + 'static + Sync,
          E: ExternalNetworkResources + Clone + Send + 'static + Sync {
    local_state.wallet.update_hardware(g);
    ui.style_mut().spacing.item_spacing.y = 2f32;

    ScrollArea::vertical().show(ui, |ui| wallet_screen_scrolled(ui, ctx, local_state, has_changed_tab, g, d));
}


pub fn wallet_screen_scrolled<G, E>(ui: &mut Ui, ctx: &egui::Context, ls: &mut LocalState<E>, has_changed_tab: bool, g: &mut G, d: &DataQueryInfo<E>)
    where G: GuiDepends + Clone + Send + 'static + Sync, E: ExternalNetworkResources + Clone + Send + 'static + Sync {

    let (mut update, xpub) =
        internal_stored_xpubs(
            ls, ui, ctx, has_changed_tab, g, None, ls.wallet.public_key.clone(), true
        );
    let mut is_hot = false;
    if has_changed_tab {
        update = true;
    }

    if let Some(x) = xpub {

        let mut passphrase_changed = false;

        if x.request_type != Some(XPubLikeRequestType::Hot) {
            cold_wallet::hardware_connected(ui, &mut ls.wallet, g);
        } else {
            is_hot = true;
            if ls.wallet.view_additional_xpub_details {
                passphrase_changed = ls.wallet.passphrase_input.view(ui);
            }
        }

        if update || passphrase_changed {
            // Are either of these even used?
            // ls.wallet_state.active_xpub = x.xpub.clone();
            // ls.wallet_state.active_derivation_path = ls.keytab_state.derivation_path_xpub_input_account.derivation_path();
            if let Ok(pk) = ls.keytab_state.xpub_key_info.public_key() {
                ls.wallet.public_key = Some(pk.clone());
                if is_hot {
                    if check_assign_hot_key(ls, &x, &pk).is_err() {
                        // Err, refuse to proceed. Show error message to user
                        ls.wallet.public_key = None;
                    } else {

                    }
                }
            } else {
                ls.wallet.public_key = None;
            }
        }
    }

    if let Some(pk) = ls.wallet.public_key.clone() {
        ls.wallet.passphrase_input.err_msg = None;
        if update || (ls.wallet.address_info.is_none() && has_changed_tab) {
            ls.wallet.address_info = None;
            refresh_balance(ls, g);
            ls.wallet.address_labels = ls.local_stored_state.address_labels(g);
            ls.wallet.receive_data = Some(ReceiveData::from_public_key(&pk, g));
        }
        if update {
            // let kp = ls.wallet_state.hot_mnemonic().keypair_at(ls.wallet_state.derivation_path.clone()).expect("kp");


        }
        let allowed = if !is_hot {
            vec![XPubLikeRequestType::Cold, XPubLikeRequestType::File, XPubLikeRequestType::QR]
        } else {
            vec![XPubLikeRequestType::Hot]
        };

        let hot_tsi = ls.hot_transaction_sign_info(g);
        proceed_from_pk(ui, ls, &pk, is_hot, g, d, &allowed, &hot_tsi, &ls.cold_transaction_sign_info(g));
    }

}

fn check_assign_hot_key<E>(ls: &mut LocalState<E>, x: &AccountKeySource, pk: &PublicKey
) -> RgResult<()> where E: ExternalNetworkResources + Clone + Send + 'static + Sync {
    let key_name = x.key_name_source.as_ref().ok_msg("key_name")?;
    let k = ls.local_stored_state.by_key(&key_name).ok_msg("key")?;

    match k {
        Either::Left(m) => {
            let pass = ls.wallet.passphrase_input.passphrase.clone();
            let words = m.mnemonic.clone();
            let w = WordsPass::new(
                &words,
                Some(pass.clone())
            );
            w.validate()?;
            if !ls.keytab_state.derivation_path_xpub_input_account.valid {
                return Ok(())
            }
            let dp = ls.keytab_state.derivation_path_xpub_input_account.derivation_path();
            let dp_xpub =  ls.keytab_state.xpub_key_info.derivation_path.clone();
            let pk2 = w.public_at(
                &dp
            )?;
            if &pk2 != pk {
                info!("Setting public key mismatch error for keyname {} checksum {} {} {} {} {} ",
                    key_name.clone(),
                    w.checksum().expect(""), dp.clone(), dp_xpub.clone(),
                    pk2.hex(), pk.hex()
                );
                ls.wallet.passphrase_input.err_msg = Some("Public key mismatch".to_string());
                return Err(error_info("Public key mismatch".to_string()));
            }
            ls.wallet.active_hot_mnemonic = Some(words);
            ls.wallet.hot_passphrase = pass;
            ls.wallet.derivation_path = dp.clone();
            ls.wallet.derivation_path_valid = true;
            ls.wallet.active_derivation_path = dp.clone();
            ls.wallet.xpub_derivation_path = dp.clone();
        }
        Either::Right(kpo) => {
            // Eek, need to get the xpub diff combo box for private keys? Or include it somehow
            // this is more complex here.
            return Err(error_info("Not yet implemented".to_string()));
            // let kp = KeyPair::from_private_hex(kpo.key_hex.clone())?;
            // let pk2 = kp.public_key();
            // if &pk2 != pk {
            //     return Err(error_info("Public key mismatch".to_string()));
            // }
            // ls.wallet_state.active_hot_private_key_hex = Some(kpo.key_hex.clone());
        }
    };
    Ok(())
}

pub fn hot_passphrase_section<E, G>(ui: &mut Ui, ls: &mut LocalState<E>, g: &G) -> bool
where E: ExternalNetworkResources + Clone + Send + 'static + Sync, G: GuiDepends + Clone + Send + 'static + Sync {

    let mut update_clicked = false;

    if &ls.wallet.hot_passphrase_last != &ls.wallet.hot_passphrase.clone() {
        ls.wallet.hot_passphrase_last = ls.wallet.hot_passphrase.clone();
        update_clicked = true;
    }
    if &ls.wallet.hot_offset_last != &ls.wallet.hot_offset.clone() {
        ls.wallet.hot_offset_last = ls.wallet.hot_offset.clone();
        update_clicked = true;
    }

        ui.horizontal(|ui| {
            ui.label("Passphrase:");
            egui::TextEdit::singleline(&mut ls.wallet.hot_passphrase)
                .desired_width(150f32)
                .password(true).show(ui);
            ui.label("Offset:");
            egui::TextEdit::singleline(&mut ls.wallet.hot_offset)
                .desired_width(150f32)
                .show(ui);
            if ui.button("Update").clicked() {
                update_clicked = true;
            };
        });
    if update_clicked {
        ls.wallet.update_hot_mnemonic_or_key_info(g);
    };
    update_clicked
}

fn proceed_from_pk<G, E>(
    ui: &mut Ui, ls: &mut LocalState<E>, pk: &PublicKey, is_hot: bool, g: &mut G, d: &DataQueryInfo<E>,
    allowed: &Vec<XPubLikeRequestType>, hot_tsi: &TransactionSignInfo, csi: &TransactionSignInfo
)
    where G: GuiDepends + Clone + Send + 'static + Sync,
    E: ExternalNetworkResources + Clone + Send + 'static + Sync {

    // TODO: Include bitcoin address / ETH address for path 0 here for verification.
    ui.separator();

    if ls.wallet.show_xpub_balance_info {
        let data = ls.data.get(&g.get_network());
        if let Some(d) = data {
            balance_table(ui, d, &ls.node_config, None, Some(pk), None, Some("wallet_balance".to_string()));
        }

    }


    send_receive_bar(ui, ls, pk, g);

    ui.separator();
    ui.spacing();

    let mut show_prepared = false;
    match ls.wallet.send_receive {
        SendReceiveTabs::Send => {
            show_prepared = true;
            ls.wallet.send_view_top(
                ui, pk, g, d,
                ls.wallet.address_labels.clone(),
                hot_tsi,
                &ls.node_config,
                csi,
                allowed
            );
        }
        SendReceiveTabs::Receive => {
            if let Some(rd) = ls.wallet.receive_data.as_ref() {
                rd.view(ui);
            }
        }
        SendReceiveTabs::Custom => {
            ls.wallet.custom_tx.view::<E, G>(ui, g, hot_tsi, csi, allowed);
        }
        SendReceiveTabs::Swap => {
            if ls.swap_state.view(ui, g, pk, allowed, csi, hot_tsi, d) {
                // TODO: refactor this out
                create_swap_tx(ls, g);
            }
        }
        SendReceiveTabs::Home => {
            let rows = d.recent_tx(Some(pk), None, true, None, g);
            let mut tx_table = TransactionTable::default();
            tx_table.rows = rows;
            tx_table.full_view::<E>(ui, &g.get_network(), d, Some(pk));
            ui.separator();
        }
        SendReceiveTabs::Portfolio => {
            ls.wallet.port.view(ui, pk, g, hot_tsi, &ls.node_config, d, csi, allowed);
        }
        SendReceiveTabs::Stake => {
            ls.wallet.stake.view(ui, d, g, pk, hot_tsi, &ls.node_config, allowed, csi);
        }

    }
    if show_prepared {
        // prepared_tx_view::prepared_view(ui, ls, pk, is_hot);
    }

}


fn currency_selection_box<E>(ui: &mut Ui, ls: &mut LocalState<E>) where E: ExternalNetworkResources + Clone + Send + 'static + Sync {
    ComboBox::from_label("Currency")
        .selected_text(format!("{:?}", ls.wallet.send_currency_type))
        .show_ui(ui, |ui| {
            let styles = vec![SupportedCurrency::Bitcoin, SupportedCurrency::Redgold];
            for style in styles {
                ui.selectable_value(&mut ls.wallet.send_currency_type, style.clone(), format!("{:?}", style));
            }
        });
}

fn swap_view<E>(_ui: &mut Ui, _ls: &mut LocalState<E>, _pk: &PublicKey) where E: ExternalNetworkResources + Clone + Send + 'static + Sync {
    //
    // ComboBox::from_label("Currency")
    //     .selected_text(format!("{:?}", ls.wallet_state.send_currency_type))
    //     .show_ui(ui, |ui| {
    //         let styles = vec![SupportedCurrency::Bitcoin, SupportedCurrency::Redgold];
    //         for style in styles {
    //             ui.selectable_value(&mut ls.wallet_state.send_currency_type, style.clone(), format!("{:?}", style));
    //         }
    //     });
    // ui.horizontal(|ui| {
    //     ui.label("Destination Address");
    //     let string = &mut ls.wallet_state.destination_address;
    //     ui.add(egui::TextEdit::singleline(string).desired_width(460.0));
    //     common::copy_to_clipboard(ui, string.clone());
    //     let valid_addr = Address::parse(string.clone()).is_ok();
    //     if valid_addr {
    //         ui.label(RichText::new("Valid").color(Color32::GREEN));
    //     } else {
    //         ui.label(RichText::new("Invalid").color(Color32::RED));
    //     }
    // });
    // // TODO: Amount USD and conversions etc.
    // ui.horizontal(|ui| {
    //     ui.label("Amount");
    //     let string = &mut ls.wallet_state.amount_input;
    //     ui.add(egui::TextEdit::singleline(string).desired_width(200.0));
    // });
}

fn send_receive_bar<G, E>(ui: &mut Ui, ls: &mut LocalState<E>, pk: &PublicKey, g: &G) where G: GuiDepends + Clone + Send + 'static + Sync, E: ExternalNetworkResources + Clone + Send + 'static + Sync {
    ui.horizontal(|ui| {
        let style = ui.style_mut();
        style.override_text_style = Some(TextStyle::Heading);

        for t in SendReceiveTabs::iter() {
            if ui.button(format!("{:?}", t)).clicked() {
                ls.wallet.send_receive = t.clone();
            }
        }
        let layout = egui::Layout::right_to_left(egui::Align::RIGHT);

        ui.with_layout(layout, |ui| {

            let url_env = if ls.node_config.network.is_main() {
                "".to_string()
            } else {
                format!("{}.",ls.node_config.network.to_std_string())
            };
            // TODO: Format the address of some xpub.
            let env_formatted_faucet = format!("https://{}explorer.redgold.io/faucet", url_env, );
            ui.hyperlink_to("Faucet", env_formatted_faucet);
            ui.label(ls.wallet.faucet_success.clone());
            if ui.button("Refresh Balance").clicked() {
                refresh_balance(ls, g);
            };
        });
    });
}

fn refresh_balance<G, E>(ls: &mut LocalState<E>, g: &G) where G: GuiDepends + Clone + Send + 'static + Sync, E: ExternalNetworkResources + Clone + Send + 'static + Sync {
    let pk = ls.wallet.public_key.clone().expect("pk");
    if let Some(d) = ls.data.get(&g.get_network()) {
        d.refresh_all_pk(&pk, g);
    }
    address_query::get_address_info(
        &ls.node_config.clone(), pk, ls.local_messages.sender.clone(), g
    );
}
