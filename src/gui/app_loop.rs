#![allow(dead_code)]

use itertools::Itertools;
use redgold_schema::structs::PublicKey;
use strum::IntoEnumIterator;
use redgold_common::external_resources::ExternalNetworkResources;
// 0.17.1


use redgold_gui::tab::tabs::Tab;
use redgold_keys::xpub_wrapper::XpubWrapper;
use redgold_schema::conf::local_stored_state::{AccountKeySource, Identity, LocalStoredState, StoredMnemonic, StoredPrivateKey};

use redgold_schema::{error_info, RgResult};

use redgold_gui::dependencies::gui_depends::{GuiDepends, HardwareSigningInfo, TransactionSignInfo};
use redgold_gui::state::local_state::LocalState;
use redgold_keys::util::mnemonic_support::MnemonicSupport;

pub trait PublicKeyStoredState {
    fn public_key(&self, xpub_name: String) -> Option<PublicKey>;
}

impl PublicKeyStoredState for LocalStoredState {
    fn public_key(&self, xpub_name: String) -> Option<PublicKey> {
        let pk = self.keys.as_ref().and_then(|x| x.iter().find(|x| x.name == xpub_name)
            .and_then(|g| XpubWrapper::new(g.xpub.clone()).public_at(0, 0).ok()));
        pk
    }
}
