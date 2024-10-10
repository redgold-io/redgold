use eframe::egui::Ui;
use redgold_keys::address_external::{ToBitcoinAddress, ToEthereumAddress};
use redgold_schema::structs::{NetworkEnvironment, PublicKey};

// TODO Inject GUI dependencies to translate PK to all addresses
pub fn rdg_explorer(ui: &mut Ui, network: &NetworkEnvironment, pk: &PublicKey) {
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
        let btc_address = pk.to_bitcoin_address_typed(&network).unwrap().render_string().unwrap();
        let mut net = "testnet/";
        if is_main {
            net = "";
        }
        let eth_url = if is_main {
            "https://etherscan.io/address/"
        } else {
            "https://sepolia.etherscan.io/address/"
        };
        let eth_address = pk.to_ethereum_address().unwrap();
        ui.hyperlink_to("BTC Explorer", format!("https://blockstream.info/{net}address/{btc_address}"));
        ui.hyperlink_to("ETH Explorer", format!("{}{}", eth_url, eth_address));
    });
}
