use eframe::egui;
use eframe::egui::{Image, Separator, Ui, Vec2};
use log::info;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::structs::{PublicKey, SupportedCurrency};
use crate::dependencies::gui_depends::GuiDepends;
use crate::functionality::qr_render::qr_encode_image;

#[derive(Clone, Debug)]
pub struct ReceiveData {
    rdg_address: Image<'static>,
    btc_address: Image<'static>,
    eth_address: Image<'static>,
}

impl ReceiveData {

    pub fn from_public_key<G>(public_key: &PublicKey, g: &G) -> Self where G: GuiDepends + Send + Clone + 'static {
        let addrs = g.to_all_address(public_key);
        info!("receive data addrs: {}", addrs.iter().map(|a| a.render_string().unwrap()).collect::<Vec<String>>().join(", "));
        addrs.iter().map(|a| a.as_external().currency_or().json_or()).for_each(|c| info!("receive data currency: {}", c));

        let encode = |supported_currency: SupportedCurrency| {
            let address = addrs.iter()
                .filter(|a| a.as_external().currency_or() == supported_currency).next().unwrap().clone()
                .render_string().unwrap();
            info!("receive data {} address: {}", supported_currency.json_or(), address);
            let mut image = qr_encode_image(address);
            image.fit_to_exact_size(Vec2::new(250.0, 250.0))
        };

        Self {
            rdg_address: encode(SupportedCurrency::Redgold),
            btc_address: encode(SupportedCurrency::Bitcoin),
            eth_address: encode(SupportedCurrency::Ethereum),
        }
    }

    pub fn view(&self, ui: &mut Ui) {

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading("Redgold Address");
                ui.add(self.rdg_address.clone());
            });
            ui.add(Separator::default().vertical());
            ui.vertical(|ui| {
                ui.heading("Bitcoin Address");
                ui.add(self.btc_address.clone());
            });
            ui.add(Separator::default().vertical());
            ui.vertical(|ui| {
                ui.heading("Ethereum Address");
                ui.add(self.eth_address.clone());
            });
        });
    }
}