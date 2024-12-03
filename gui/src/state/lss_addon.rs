use redgold_schema::conf::local_stored_state::LocalStoredState;
use redgold_schema::structs::{Address, SupportedCurrency};
use crate::dependencies::gui_depends::GuiDepends;

pub trait LssAddon {
    fn address_labels<G>(&self, g: &G) -> Vec<(String, Address)> where G: GuiDepends + Send + 'static;
}

impl LssAddon for LocalStoredState {

    fn address_labels<G>(&self, g: &G) -> Vec<(String, Address)> where G: GuiDepends + Send + 'static {
        let mut all = self.saved_addresses.as_ref().unwrap_or(&vec![]).iter()
            .map(|a| (a.name.clone(), a.address.clone())).collect::<Vec<(String, Address)>>();
        self.xpubs.as_ref().unwrap_or(&vec![]).iter().for_each(|x| {
            let all_address = if let Some(all_address) = &x.all_address {
                all_address.clone()
            } else {
                if let Ok(pk) = g.xpub_public(x.xpub.clone(), x.derivation_path.clone()) {
                    let mut addrs = g.to_all_address(&pk);
                    if x.definitely_not_hot() {
                        addrs = addrs.iter().filter(|a| {
                            let mut a2 = a.clone().clone();
                            a2.mark_external().currency_or() == SupportedCurrency::Bitcoin
                        }).cloned().collect();
                    }
                    addrs
                } else {
                    vec![]
                }
            };
            all.extend(all_address.iter().map(|a| {
                    let address = a.clone();
                    let c = address.clone().mark_external().currency_or();
                    (format!("xpub-{} - {}", x.name.clone(), c.abbreviated()), address)
                }).collect::<Vec<(String, Address)>>());
        });
        all
    }

}