use std::collections::HashMap;
use redgold_schema::RgResult;
use redgold_schema::structs::PublicKey;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::party::party_internal_data::PartyInternalData;
use crate::party::party_watcher::PartyWatcher;

impl<T> PartyWatcher<T> where T: ExternalNetworkResources + Send {

    pub async fn handle_key_rotations(&self, data: &mut HashMap<PublicKey, PartyInternalData>) -> RgResult<()> {
        // TODO Mark successor keys, also in party internal data mark its current state
        for (key, dat) in data.iter_mut() {
        }
        Ok(())
    }
}