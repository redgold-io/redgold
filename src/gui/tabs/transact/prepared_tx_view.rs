// use eframe::egui;
// use eframe::egui::Ui;
// use redgold_keys::transaction_support::TransactionSupport;
// use redgold_keys::util::btc_wallet::SingleKeyBitcoinWallet;
// use redgold_schema::helpers::easy_json::EasyJsonDeser;
// use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
// use redgold_schema::structs::{PublicKey, SupportedCurrency, Transaction};
// use redgold_schema::util::lang_util::JsonCombineResult;
// use crate::gui::app_loop::LocalState;
// use redgold_gui::common;
// use redgold_gui::common::data_item;
// use redgold_gui::tab::transact::states::WalletTab;
// use crate::gui::tabs::transact::{broadcast_tx, hardware_signing, prepare_tx};
// use redgold_gui::tab::transact::states::SendReceiveTabs;
// use redgold_schema::observability::errors::Loggable;
// pub fn prepared_view(ui: &mut Ui, ls: &mut LocalState, pk: &PublicKey, is_hot: bool) {
//
//     if ui.button("Prepare Transaction").clicked() {
//         if ls.wallet.send_currency_type == SupportedCurrency::Bitcoin {
//             if let Ok(amount) = ls.wallet.amount_input.parse::<f64>() {
//
//                 let mut w = SingleKeyBitcoinWallet::new_wallet(
//                     pk.clone(), ls.node_config.network, true
//                 ).expect("w");
//                 let result = w.prepare_single(
//                     ls.wallet.destination_address.clone(),
//                     amount
//                 );
//                 ls.wallet.signing_flow_transaction_box_msg = Some(
//                     result.clone().json_or_combine()
//                 );
//                 let status = result.as_ref().map(|_x| "Transaction Prepared".to_string())
//                     .unwrap_or("Preparation Failed".to_string());
//                 ls.wallet.signing_flow_status = Some(status);
//                 ls.wallet.transaction_prepared_success = result.is_ok();
//             }
//         } else {
//             match &ls.wallet.address_info {
//                 None => {
//                     ls.wallet.signing_flow_status = Some("Missing UTXO info".to_string());
//                 }
//                 Some(ai) => {
//                     let result = prepare_tx::prepare_transaction(
//                         ai,
//                         &ls.wallet.amount_input,
//                         &ls.wallet.destination_address,
//                         &ls.wallet,
//                         &ls.node_config
//                     );
//                     ls.wallet.update_unsigned_tx(Some(result.clone()));
//                     ls.wallet.signing_flow_transaction_box_msg = Some(
//                         result.clone().json_or_combine()
//                     );
//                     let status = result.map(|_x| "Transaction Prepared".to_string())
//                         .unwrap_or("Preparation Failed".to_string());
//                     ls.wallet.signing_flow_status = Some(status);
//                 }
//             }
//         }
//         if ls.wallet.send_receive == SendReceiveTabs::Custom {
//             ls.wallet.prepared_transaction = Some(
//                 ls.wallet.custom_tx_json.json_from::<Transaction>()
//             )
//         }
//     }
//     if let Some(p) = &ls.wallet.signing_flow_transaction_box_msg {
//         // ui.with_layout(
//         //     Layout::centered_and_justified(Direction::TopDown)
//         //     ,|ui|
//         ui.label("Rendered Transaction Information"); //);
//         ui.spacing();
//         let string1 = &mut p.clone();
//         common::bounded_text_area(ui, string1);
//     }
//
//     if ls.wallet.send_currency_type != SupportedCurrency::Redgold && ls.wallet.transaction_prepared_success {
//         // BTC
//
//         if ui.button("Sign Transaction").clicked() {
//             match &ls.wallet.send_currency_type {
//                 SupportedCurrency::Bitcoin => {
//                     if let Ok(amount) = ls.wallet.amount_input.parse::<f64>() {
//                         // TODO: Support single private key also
//                         let kp = ls.wallet.hot_mnemonic().keypair_at(ls.keytab_state.derivation_path_xpub_input_account.derivation_path());
//                         if let Ok(pk_hex) = kp.map(|kp| kp.to_private_hex()) {
//                             let mut w = SingleKeyBitcoinWallet::new_wallet(
//                                 pk.clone(), ls.node_config.network, true
//                             ).expect("w");
//                             let result = w.prepare_single_sign(
//                                 ls.wallet.destination_address.clone(),
//                                 amount,
//                                 pk_hex
//                             ).log_error();
//
//                             ls.wallet.signing_flow_transaction_box_msg = Some(
//                                 result.clone().json_or_combine()
//                             );
//                             let status = result.as_ref().map(|_x| "Transaction Signed".to_string())
//                                 .unwrap_or("Signing Failed".to_string());
//                             ls.wallet.signing_flow_status = Some(status);
//                             ls.wallet.transaction_sign_success = result.is_ok();
//                         }
//                     }
//                 }
//                 _ => {}
//             }
//         }
//         if ls.wallet.transaction_sign_success {
//             if let Some(h) = ls.wallet.signed_transaction_hash.clone() {
//                 data_item(ui, "TXID:", h);
//             }
//             if ui.button("Broadcast Transaction").clicked() {
//                 match &ls.wallet.send_currency_type {
//                     SupportedCurrency::Bitcoin => {
//                         if let Ok(amount) = ls.wallet.amount_input.parse::<f64>() {
//                             // TODO: Support single private key also
//                             let kp = ls.wallet.hot_mnemonic().keypair_at(ls.keytab_state.derivation_path_xpub_input_account.derivation_path());
//                             if let Ok(pk_hex) = kp.map(|kp| kp.to_private_hex()) {
//                                 let mut w = SingleKeyBitcoinWallet::new_wallet(
//                                     pk.clone(), ls.node_config.network, true
//                                 ).expect("w");
//                                 let result = w.prepare_single_sign_and_broadcast(
//                                     ls.wallet.destination_address.clone(),
//                                     amount,
//                                     pk_hex
//                                 ).log_error();
//                                 if let Ok(r) = &result {
//                                     ls.wallet.signed_transaction_hash = Some(r.clone());
//                                 }
//                                 let status = result.map(|_x| "Transaction Broadcast".to_string())
//                                     .unwrap_or("Broadcast Failed".to_string());
//                                 ls.wallet.signing_flow_status = Some(status);
//                                 // ls.wallet_state.transaction_sign_success = result.is_ok();
//                             }
//                         }
//                     }
//                     _ => {}
//                 }
//             }
//         }
//
//
//     }
//
//     if let Some(res) = &ls.wallet.prepared_transaction {
//         if let Some(t) = res.as_ref().ok() {
//             ui.allocate_ui(egui::Vec2::new(500.0, 0.0), |ui| {
//                 ui.centered_and_justified(|ui| {
//                     data_item(ui, "Raw TX Hash:".to_string(), t.hash_hex());
//                 });
//             });
//             if ui.button("Sign Transaction").clicked() {
//                 if ls.wallet.send_currency_type == SupportedCurrency::Redgold {
//                     if !is_hot {
//                             hardware_signing::initiate_hardware_signing(
//                                 t.clone(),
//                                 ls.wallet.updates.sender.clone(),
//                                 pk.clone().clone(),
//                                 ls.keytab_state.derivation_path_xpub_input_account.derivation_path()
//                             );
//                             ls.wallet.signing_flow_status = Some("Awaiting hardware response...".to_string());
//                         } else {
//                             let kp = ls.wallet.hot_mnemonic().keypair_at(ls.wallet.derivation_path.clone()).expect("kp");
//                             let mut t2 = t.clone();
//                             let signed = t2.sign(&kp);
//                             ls.wallet.update_signed_tx(Some(signed));
//                         }
//                     }
//                 } else if ls.wallet.send_currency_type == SupportedCurrency::Bitcoin {
//                     match ls.wallet.tab {
//                         WalletTab::Hardware => {
//                             // error!("Hardware signing not supported yet for btc");
//                         }
//                         WalletTab::Software => {
//                             // error!("Software signing not yet supported for btc");
//                             // let mut w = SingleKeyBitcoinWallet::new_wallet(
//                             //     pk.clone(), ls.node_config.network, true
//                             // ).expect("w");
//                             // let result = w.prepare_single_sign(
//                             //     ls.wallet_state.destination_address.clone(),
//                             //     ls.wallet_state.amount_input.parse::<f64>().expect("f64")
//                             // );
//                             // if let Ok(tx) = result {
//                             //     let signed = w.sign_single(&tx);
//                             //     ls.wallet_state.update_signed_tx(Some(signed));
//                         }
//                     }
//                 }
//             }
//     }
//     if let Some(m) = &ls.wallet.signing_flow_status {
//         ui.label(m);
//     }
//     if let Some(t) = &ls.wallet.signed_transaction {
//         if let Some(t) = t.as_ref().ok() {
//             data_item(ui, "Signed TX Hash:", ls.wallet.signed_transaction_hash.clone().unwrap_or("error".to_string()));
//             if ui.button("Broadcast Transaction").clicked() {
//                 broadcast_tx::broadcast_transaction(
//                     ls.node_config.clone(),
//                     t.clone(),
//                     ls.wallet.updates.sender.clone(),
//                 );
//                 ls.wallet.signing_flow_status = Some("Awaiting broadcast response...".to_string());
//             }
//         }
//     }
// }
