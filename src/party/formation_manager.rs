use crate::multiparty_gg20::initiate_mp;
use crate::party::deposit_key_allocation::DepositKeyAllocation;
use crate::party::party_watcher::PartyWatcher;
use itertools::Itertools;
use metrics::{counter, gauge};
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::{EnhanceErrorInfo, Loggable};
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::structs::{ErrorInfo, Hash, HealthRequest, NetworkEnvironment, PublicKey, Request};
use redgold_schema::{error_info, RgResult, SafeOption};
use std::collections::HashMap;
use tracing::{error, info};

impl<T> PartyWatcher<T> where T: ExternalNetworkResources + Send {

    pub async fn initial_formation(&self) -> RgResult<()> {
        counter!("redgold_party_initial_formation").increment(1);

        // info!("Initial party key formation");
        // Initiate MP keysign etc. gather public key and original proof and params
        // TODO: Add deposit trust scored peers.
        let mut seeds = self.relay.node_config.non_self_seeds_pk().clone();
        gauge!("redgold_party_initial_formation_non_self_seeds").set(seeds.len() as f64);
        if self.relay.node_config.network == NetworkEnvironment::Local {
            seeds = self.relay.ds.peer_store.active_nodes(None).await.unwrap_or(vec![]);
        }
        if seeds.len() < 2 {
            info!("Not enough peers in local network for party formation");
            return Ok(())
        }
        info!("Local network party formation with {} peers", seeds.len());
        let party_peers = seeds.clone();
        self.form_keygen_group(party_peers).await?;
        // info!("Initial party key formation success");
        Ok(())
    }

    async fn form_keygen_group(&self, party_peers: Vec<PublicKey>) -> RgResult<()> {
        let results = self.relay.health_request(&party_peers).await?;
        let errs = results.iter().filter_map(|r| r.as_ref().err()).collect_vec();
        gauge!("redgold_party_form_keygen_group_peer_errs").set(errs.len() as f64);
        if errs.len() > 0 {

            // info!("Not enough peers in party formation");
            // error!("Error in initial party key formation {}", errs.json_or());
            // return Err(error_info("Error in initial party key formation {}"))
            //     .with_detail("party_peers", party_peers.json_or())
            //     .with_detail("errors", errs.json_or())
            return Ok(())
        }
        let r = initiate_mp::initiate_mp_keygen(
            self.relay.clone(),
            None,
            true,
            Some(party_peers.clone()),
            false,
            vec![]
        ).await?;

        // info!("Keygen group formed with {}", r.json_or());
        // TODO: Get this from local share instead of from a second keysign round.
        let test_sign = r.identifier.room_id.safe_get()?.uuid.safe_get()?.clone();
        let h = Hash::from_string_calculate(&test_sign);
        let bd = h.bytes.safe_get_msg("Missing bytes in immediate hash calculation")?;
        let ksr = initiate_mp::initiate_mp_keysign(
            self.relay.clone(), r.identifier.clone(),
            bd.clone(),
            r.identifier.party_keys.clone(),
            None,
            None,
        ).await?;
        // info!("Keygen signing group formed with {} peers {}", party_peers.len(), ksr.json_or());
        let party_pk = ksr.proof.public_key.safe_get_msg("proof in ksr")?.clone();
        // info!("Party public key: {}", party_pk);
        let api = self.relay.ds.multiparty_store.all_party_info_with_key().await?;
        // info!("All party info with key: {}", api.json_or());
        let pd = self.relay.ds.multiparty_store.party_data(&party_pk).await?;
        let pd = pd.safe_get_msg("party data missing")?;
        // info!("Party data: {}", pd.json_party_internal_data.clone().unwrap_or("".to_string()));


        Ok(())
    }

    pub async fn tick_formations(&self, shared_data: &HashMap<PublicKey, PartyInternalData>) -> RgResult<()> {

        counter!("redgold_party_formation_tick").increment(1);
        let self_host = shared_data.iter()
            .filter(|(k,v)| v.self_initiated_not_debug())
            .collect_vec();

        // info!("Party formation tick self_host_len: {}", self_host.len());

        if self_host.len() == 0 {
            self.initial_formation().await.log_error().ok();
        }

        Ok(())
    }

}