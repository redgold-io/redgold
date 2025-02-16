use eframe::egui::Ui;
use redgold_schema::structs::{NetworkEnvironment, PublicKey, SupportedCurrency};
use std::collections::HashMap;
use crate::dependencies::gui_depends::GuiDepends;

// TODO Inject GUI dependencies to translate PK to all addresses
pub fn rdg_explorer<G>(ui: &mut Ui, network: &NetworkEnvironment, pk: &PublicKey, g: &G) where G: GuiDepends {
    let mut explorer_prefix = network.to_std_string();
    let is_main = explorer_prefix == "main".to_string();
    if is_main {
        explorer_prefix = "".to_string();
    } else {
        explorer_prefix = format!("{}.", explorer_prefix);
    }
    ui.horizontal(|ui| {
        let rdg_address = pk.address().unwrap().render_string().unwrap();
        ui.hyperlink_to("RDG Explorer", format!("https://{}explorer.redgold.io/hash/{}", explorer_prefix, rdg_address));
        let btc_address = g.form_btc_address(pk).unwrap().render_string().unwrap();
        let mut net = "testnet/";
        if is_main {
            net = "";
        }
        let eth_url = if is_main {
            "https://etherscan.io/address/"
        } else {
            "https://sepolia.etherscan.io/address/"
        };
        let eth_address = g.form_eth_address(pk).unwrap().render_string().unwrap();
        ui.hyperlink_to("BTC Explorer", format!("https://blockstream.info/{net}address/{btc_address}"));
        ui.hyperlink_to("ETH Explorer", format!("{}{}", eth_url, eth_address));
    });
}

pub fn rdg_explorer_links<G>(network: &NetworkEnvironment, pk: &PublicKey, g: &G) -> HashMap<SupportedCurrency, String> where G: GuiDepends {
    let mut explorer_prefix = network.to_std_string();
    let is_main = explorer_prefix == "main".to_string();
    if is_main {
        explorer_prefix = "".to_string();
    } else {
        explorer_prefix = format!("{}.", explorer_prefix);
    }
    let rdg_address = pk.address().unwrap().render_string().unwrap();
    let btc_address = g.form_btc_address(pk).unwrap().render_string().unwrap();
    let mut net = "testnet/";
    if is_main {
        net = "";
    }
    let eth_url = if is_main {
        "https://etherscan.io/address/"
    } else {
        "https://sepolia.etherscan.io/address/"
    };
    let eth_address = g.form_eth_address(pk).unwrap().render_string().unwrap();
    let rdg_explorer = format!("https://{}explorer.redgold.io/hash/{}", explorer_prefix, rdg_address);
    let btc_explorer = format!("https://blockstream.info/{net}address/{btc_address}");
    let eth_explorer = format!("{}{}", eth_url, eth_address);
    let mut map = HashMap::new();
    map.insert(SupportedCurrency::Redgold, rdg_explorer);
    map.insert(SupportedCurrency::Bitcoin, btc_explorer);
    map.insert(SupportedCurrency::Ethereum, eth_explorer);
    map
}