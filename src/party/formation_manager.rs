use std::collections::HashMap;
use itertools::Itertools;
use log::{error, info};
use redgold_schema::{RgResult, SafeOption};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::structs::{Hash, HealthRequest, PublicKey, Request};
use crate::multiparty_gg20::initiate_mp;
use crate::party::data_enrichment::PartyInternalData;
use crate::party::deposit_key_allocation::DepositKeyAllocation;
use crate::party::party_watcher::PartyWatcher;

impl PartyWatcher {

    pub async fn initial_formation(&self) -> RgResult<()> {
        info!("Initial party key formation");
        // Initiate MP keysign etc. gather public key and original proof and params
        // TODO: Add deposit trust scored peers.
        let seeds = self.relay.node_config.non_self_seeds_pk().clone();
        let party_peers = seeds.clone();
        self.form_keygen_group(party_peers).await?;
        Ok(())
    }

    async fn form_keygen_group(&self, party_peers: Vec<PublicKey>) -> RgResult<()> {
        let results = self.relay.health_request(&party_peers).await?;
        let errs = results.iter().filter_map(|r| r.as_ref().err()).collect_vec();
        if errs.len() > 0 {
            error!("Error in initial party key formation {}", errs.json_or());
            return Ok(())
        }

        let res = initiate_mp::initiate_mp_keygen(
            self.relay.clone(),
            None,
            true,
            Some(party_peers),
            false
        ).await.log_error();
        // TODO: Get this from local share instead of from a second keysign round.
        if let Ok(r) = res {
            let test_sign = r.identifier.room_id.safe_get()?.uuid.safe_get()?.clone();
            let h = Hash::from_string_calculate(&test_sign);
            let bd = h.bytes.safe_get_msg("Missing bytes in immediate hash calculation")?;
            let _ksr = initiate_mp::initiate_mp_keysign(
                self.relay.clone(), r.identifier.clone(),
                bd.clone(),
                r.identifier.party_keys.clone(),
                None,
                None
            ).await.log_error();
        }
        Ok(())
    }

    pub async fn tick_formations(&self, shared_data: HashMap<PublicKey, PartyInternalData>) -> RgResult<()> {
        info!("Party formation tick");
        let self_host = shared_data.iter()
            .filter(|(k,v)| v.party_info.self_initiated.unwrap_or(false))
            .filter(|(k,v)| v.not_debug())
            .collect_vec();
        if self_host.len() == 0 {
            self.initial_formation().await.log_error().ok();
        }

        Ok(())
    }

}