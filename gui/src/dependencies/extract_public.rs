use redgold_schema::conf::local_stored_state::LocalStoredState;
use redgold_schema::structs::PublicKey;
use crate::dependencies::gui_depends::GuiDepends;

pub trait ExtractorPublicKey {
    fn extract<G>(&self, g: &G) -> Vec<PublicKey> where G: GuiDepends + Send + Clone;
}

impl ExtractorPublicKey for LocalStoredState {
    fn extract<G>(&self, g: &G) -> Vec<PublicKey> where G: GuiDepends + Send + Clone {
        let mut res = vec![];
        for x in self.keys.clone().unwrap_or(vec![]).iter() {
            if let Ok(p) = g.xpub_public(x.xpub.clone(), x.derivation_path.clone()) {
                res.push(p)
            }
        }
        // for x in self.mnemonics.iter() {
        //
        // }
        res
    }
}