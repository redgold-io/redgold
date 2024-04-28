use std::collections::HashMap;
use itertools::Itertools;
use log::{error, info};
use redgold_schema::{error_info, RgResult, SafeOption};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::{EnhanceErrorInfo, Loggable};
use redgold_schema::structs::{ErrorInfo, Hash, HealthRequest, PublicKey, Request};
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
        info!("Initial party key formation success");
        Ok(())
    }

    async fn form_keygen_group(&self, party_peers: Vec<PublicKey>) -> RgResult<()> {
        let results = self.relay.health_request(&party_peers).await?;
        let errs = results.iter().filter_map(|r| r.as_ref().err()).collect_vec();
        if errs.len() > 0 {
            // error!("Error in initial party key formation {}", errs.json_or());
            return Err(error_info("Error in initial party key formation {}"))
                .with_detail("party_peers", party_peers.json_or())
                .with_detail("errors", errs.json_or())
        }
        let r = initiate_mp::initiate_mp_keygen(
            self.relay.clone(),
            None,
            true,
            Some(party_peers),
            false
        ).await?;
        // TODO: Get this from local share instead of from a second keysign round.
        let test_sign = r.identifier.room_id.safe_get()?.uuid.safe_get()?.clone();
        let h = Hash::from_string_calculate(&test_sign);
        let bd = h.bytes.safe_get_msg("Missing bytes in immediate hash calculation")?;
        let _ksr = initiate_mp::initiate_mp_keysign(
            self.relay.clone(), r.identifier.clone(),
            bd.clone(),
            r.identifier.party_keys.clone(),
            None,
            None
        ).await?;
        Ok(())
    }

    pub async fn tick_formations(&self, shared_data: HashMap<PublicKey, PartyInternalData>) -> RgResult<()> {
        let self_host = shared_data.iter()
            .filter(|(k,v)| v.party_info.self_initiated.unwrap_or(false))
            .filter(|(k,v)| v.not_debug())
            .collect_vec();

        info!("Party formation tick self_host_len: {}", self_host.len());

        if self_host.len() == 0 {
            self.initial_formation().await.log_error().ok();
        }

        Ok(())
    }

}