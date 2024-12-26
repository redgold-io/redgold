use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use flume::Sender;
use log::info;
use rand::Rng;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_common::flume_send_help::{new_channel, Channel};
use redgold_common_no_wasm::data_folder_read_ext::EnvFolderReadExt;
use redgold_gui::components::balance_table::queryable_balances;
use redgold_gui::components::tx_progress::{PreparedTransaction, TransactionProgressFlow};
use redgold_gui::data_query::data_query::DataQueryInfo;
use redgold_gui::dependencies::gui_depends::{GuiDepends, TransactionSignInfo};
use redgold_gui::state::local_state::LocalStateUpdate;
use redgold_gui::tab::tabs::Tab;
use redgold_keys::address_external::{ToBitcoinAddress, ToEthereumAddress};
use redgold_keys::util::mnemonic_support::WordsPass;
use redgold_keys::xpub_wrapper::{ValidateDerivationPath, XpubWrapper};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::conf::local_stored_state::{AccountKeySource, XPubLikeRequestType};
use redgold_schema::observability::errors::Loggable;
use redgold_schema::structs::{ErrorInfo, PublicKey, SupportedCurrency};
use crate::gui::app_loop::{LocalState, LocalStateAddons};
use crate::gui::components::tx_signer::{TxBroadcastProgress, TxSignerProgress};
use redgold_gui::tab::home::HomeState;
use redgold_schema::party::party_internal_data::PartyInternalData;
use crate::gui::tabs::identity_tab::IdentityState;
use redgold_gui::tab::keys::keygen::KeygenState;
use crate::gui::tabs::settings_tab::SettingsState;
use crate::gui::tabs::transact::wallet_tab::{StateUpdate, WalletState};
use crate::integrations::external_network_resources::ExternalNetworkResourcesImpl;
use crate::node_config::{ApiNodeConfig, DataStoreNodeConfig};
use crate::util;
use crate::util::sym_crypt;


pub async fn local_state_from<G>(
    node_config: Box<NodeConfig>,
    res: ExternalNetworkResourcesImpl,
    gui_depends: G,
    party_data: HashMap<PublicKey, PartyInternalData>
) -> Result<LocalState, ErrorInfo>
where G: Send + Clone + GuiDepends {
    let node_config = node_config.clone();

    let hot_mnemonic = node_config.secure_mnemonic_words_or();

    let config = gui_depends.get_config();
    let local_stored_state = config.local.unwrap_or_default();

    let ss = redgold_gui::tab::deploy::deploy_state::ServersState::default();


    let first_party = party_data.clone().into_values().next();

    // ss.genesis = node_config.opts.development_mode;
    let mut ls = LocalState {
        active_tab: Tab::Home,
        data: DataQueryInfo::new(&res),
        node_config: *node_config.clone(),
        // runtime,
        home_state: HomeState::default(),
        server_state: ss,
        current_time: util::current_time_millis_i64(),
        keygen_state: KeygenState::new(
            node_config.clone().executable_checksum.clone().unwrap_or("".to_string())
        ),
        wallet: WalletState::new(hot_mnemonic, local_stored_state.keys.as_ref().and_then(|x| x.first())),
        qr_state: Default::default(),
        qr_show_state: Default::default(),
        identity_state: IdentityState::new(),
        settings_state: SettingsState::new(local_stored_state.json_or(),
                                           node_config.data_folder.clone().path.parent().unwrap().to_str().unwrap().to_string(),
                                           node_config.secure_data_folder.clone().unwrap_or(node_config.data_folder.clone())
                                               .path.parent().unwrap().to_str().unwrap().to_string()
        ),
        address_state: Default::default(),
        otp_state: Default::default(),
        local_stored_state,
        // updates: new_channel(),
        keytab_state: Default::default(),
        is_mac: env::consts::OS == "macos",
        is_linux: env::consts::OS == "linux",
        is_wasm: false,
        swap_state: Default::default(),
        external_network_resources: res,
        airgap_signer: Default::default(),
        persist_requested: false,
        local_messages: Channel::new(),
        latest_local_messages: vec![],
        portfolio_tab_state: Default::default(),
    };




    ls.wallet.send_tx_progress.with_config(&node_config);
    ls.wallet.stake.complete.with_config(&node_config);
    ls.wallet.stake.deposit.with_config(&node_config);
    ls.wallet.stake.withdrawal.with_config(&node_config);
    ls.swap_state.tx_progress.with_config(&node_config);
    ls.wallet.custom_tx.tx.with_config(&node_config);
    ls.wallet.port.tx.with_config(&node_config);
    ls.wallet.port.liquidation_tx.with_config(&node_config);
    // ls.airgap_signer.interior_view()

    // info!("Price map price_map_usd_pair_incl_rdg: {}", ls.data.price_map_usd_pair_incl_rdg.json_or());

    // info!("Delta 24hr external: {}", ls.data.delta_24hr_external.json_or());

    if node_config.development_mode() {
        ls.server_state.ops = false;
        ls.server_state.system = false;
        if node_config.network.is_main()  {
            ls.server_state.words_and_id = false;
        }
    }

    let mut new_xpubs = vec![];

    if let Some(df) = node_config.secure_data_folder.as_ref() {
        if let Ok(m) = df.all().mnemonic().await {
            if let Ok(w) = WordsPass::new_validated(m.clone(), None) {
                let key_name = "secure_df_all".to_string();
                ls.wallet.selected_key_name = key_name.clone();
                ls.add_mnemonic(key_name.clone(), m, false);
                if let Ok(xpub) = w.named_xpub(key_name, true, &node_config.network) {
                    new_xpubs.push(xpub);
                }
            }
        }
    }

    if let Ok(m) = node_config.data_folder.all().mnemonic().await {
        if let Ok(w) = WordsPass::new_validated(m.clone(), None) {
            let key_name = "df_all".to_string();
            ls.add_mnemonic(key_name.clone(), m, false);
            if let Ok(xpub) = w.named_xpub(key_name, true, &node_config.network) {
                new_xpubs.push(xpub);
            }
        }
    }

    if let Ok(m) = std::env::var("REDGOLD_TEST_WORDS") {
        if let Ok(w) = WordsPass::new_validated(m.clone(), None) {
            let key_name = "test_words".to_string();
            ls.add_mnemonic(key_name.clone(), m, false);
            if let Ok(xpub) = w.named_xpub(&key_name, true, &node_config.network) {
                new_xpubs.push(xpub);
            }
            let dp_btc_faucet = "m/84'/0'/0'/0/0".to_string();
            if let Ok(xpub) = w.xpub_str(&dp_btc_faucet.as_account_path().expect("acc")) {
                let pk = XpubWrapper::new(xpub.clone()).public_at(0, 0).unwrap();
                let mut named = AccountKeySource::default();
                named.all_address = Some(gui_depends.to_all_address(&pk));
                let key_into = key_name.clone();
                named.name = format!("{}_840", key_into);
                named.xpub = xpub;
                named.public_key = Some(pk);
                named.key_name_source = Some(key_into);
                named.request_type = Some(XPubLikeRequestType::Hot);
                named.skip_persist = Some(true);
                named.derivation_path = dp_btc_faucet.clone();
                new_xpubs.push(named);
            }
        }
    }

    if !new_xpubs.is_empty() {
        let first_xpub = new_xpubs.get(0).unwrap().clone();
        ls.wallet.selected_xpub_name = first_xpub.name.clone();
        ls.add_named_xpubs(true, new_xpubs, true).expect("Adding xpubs");
    }

    if ls.local_stored_state.keys.clone().unwrap_or_default().len() > 2 {
        ls.wallet.send_address_input_box.address_input_mode = redgold_gui::components::address_input_box::AddressInputMode::Saved;
    }

    // TODO: Add from environment

    Ok(ls)
}
fn random_bytes() -> [u8; 32] {
    return rand::thread_rng().gen::<[u8; 32]>();
}
//
// pub fn send_update_sender<F: FnMut(&mut LocalState) + Send + 'static>(updates: &Sender<StateUpdate>, p0: F) {
//     updates.send(StateUpdate { update: Box::new(p0) }).unwrap();
// }
pub fn send_update<F: FnMut(&mut LocalState) + Send + 'static>(updates: &Channel<StateUpdate>, p0: F) {
    updates.sender.send(StateUpdate { update: Box::new(p0) }).unwrap();
}

pub fn create_swap_tx(ls: &mut LocalState) {
    let party_pk = ls
        .data
        .first_party
        .as_ref()
        .lock().ok()
        .and_then(|p| p.party_info.party_key.clone())
        .unwrap();
    let party_addr = party_pk.address().unwrap();

    let mut res = ls.external_network_resources.clone();
    let config = ls.node_config.clone();
    let currency = ls.swap_state.currency_input_box.input_currency.clone();
    let pk = ls.wallet.public_key.clone().unwrap();
    let kp = ls.wallet.hot_mnemonic().keypair_at(ls.keytab_state.derivation_path_xpub_input_account.derivation_path()).unwrap();
    let kp_eth_addr = kp.public_key().to_ethereum_address_typed().unwrap();
    info!("kp_eth_addr: {}", kp_eth_addr.render_string().unwrap());
    let map = ls.data.price_map_usd_pair_incl_rdg.clone();
    let amount = ls.swap_state.currency_input_box.input_currency_amount(&map);
    let mut from_eth_addr_dir = pk.to_ethereum_address_typed().unwrap();
    info!("from_eth_addr_dir: {}", from_eth_addr_dir.render_string().unwrap());
    from_eth_addr_dir.mark_external();
    info!("from_eth_addr_dir after mark external: {}", from_eth_addr_dir.render_string().unwrap());
    let from_eth_addr = from_eth_addr_dir.clone();
    info!("from_eth_addr: {}", from_eth_addr.render_string().unwrap());

    let ksi = TransactionSignInfo::PrivateKey(kp.to_private_hex());

    let to = match ls.swap_state.currency_input_box.input_currency {
        SupportedCurrency::Redgold => {
            match ls.swap_state.output_currency {
                SupportedCurrency::Bitcoin => {
                    pk.to_bitcoin_address_typed(&config.network).unwrap().clone()
                }
                SupportedCurrency::Ethereum => {
                    let addr = pk.to_ethereum_address_typed().unwrap();
                    addr.clone()
                }
                _ => panic!("Unsupported currency")
            }
        }
        SupportedCurrency::Bitcoin => {
            party_pk.to_bitcoin_address_typed(&config.network).unwrap().mark_external().clone()
        }
        SupportedCurrency::Ethereum => {
            let mut addr = party_pk.to_ethereum_address_typed().unwrap();
            addr.mark_external();
            addr.clone()
        }
        _ => panic!("Unsupported currency")
    };
    let address_info = ls.wallet.address_info.clone();

    // let secret = ls.wallet_state.hot_secret_key.clone().unwrap();
    let channel = ls.local_messages.clone();
    tokio::spawn(async move {
        let res = TransactionProgressFlow::make_transaction(
            &config,
            &mut res,
            &currency,
            &pk,
            &to,
            &amount,
            address_info.as_ref(),
            Some(&party_addr),
            None,
            Some(from_eth_addr),
            &ksi,
        ).await;
        // info!("prepared transaction: {}", res.json_or());

        channel.send(LocalStateUpdate::SwapResult(res)).await.ok();

    });
}
// pub fn sign_swap(ls: &mut LocalState, tx: PreparedTransaction) {
//     let ups = ls.updates.sender.clone();
//     let res = ls.external_network_resources.clone();
//     tokio::spawn(async move {
//         let res = tx.sign(res).await;
//         send_update_sender(&ups, move |lss| {
//             let (err, tx) = match &res {
//                 Ok(tx) => (None, Some(tx)),
//                 Err(e) => (Some(e.json_or()), None)
//             };
//             lss.swap_state.tx_progress.signed(tx.cloned(), err);
//             lss.swap_state.changing_stages = false;
//         });
//     });
// }
//
// pub fn broadcast_swap(ls: &mut LocalState, tx: PreparedTransaction) {
//     let ups = ls.updates.sender.clone();
//     let res = ls.external_network_resources.clone();
//     tokio::spawn(async move {
//         let res = tx.broadcast(res).await;
//         send_update_sender(&ups, move |lss| {
//             let (err, tx) = match &res {
//                 Ok(tx) => (None, Some(tx)),
//                 Err(e) => (Some(e.json_or()), None)
//             };
//             lss.swap_state.tx_progress.broadcast(tx.cloned(), err);
//             lss.swap_state.changing_stages = false;
//         });
//     });
// }
