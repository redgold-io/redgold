use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::pin::pin;
use crossbeam::atomic::AtomicCell;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use async_trait::async_trait;
use bdk::sled::Tree;

use crate::core::internal_message;
use crate::core::internal_message::{Channel, new_channel};
use crate::schema::structs::{
    ErrorCode, ErrorInfo, NodeState, PeerMetadata, SubmitTransactionRequest, SubmitTransactionResponse,
};
use dashmap::DashMap;
use flume::{Receiver, Sender};
use futures::{future, TryFutureExt};
use futures::stream::FuturesUnordered;
use futures::task::SpawnExt;
use itertools::Itertools;
use log::{error, info};
use metrics::counter;
use rocket::form::FromForm;
use rocket::http::ext::IntoCollection;
use tokio::runtime::Runtime;
use tokio::sync::MutexGuard;
use tracing::trace;
use redgold_schema::{error_info, ErrorInfoContext, RgResult, struct_metadata_new, structs};
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::structs::{AboutNodeRequest, Address, ContentionKey, ContractStateMarker, CurrencyAmount, DynamicNodeMetadata, GossipTransactionRequest, Hash, HashType, HealthRequest, InitiateMultipartyKeygenRequest, InitiateMultipartySigningRequest, MultipartyIdentifier, NodeMetadata, ObservationProof, Output, PartitionInfo, PartyId, PeerId, PeerIdInfo, PeerNodeInfo, PublicKey, Request, ResolveHashRequest, Response, RoomId, State, SupportedCurrency, Transaction, TrustData, UtxoEntry, UtxoId, ValidationType};
use crate::core::transact::tx_builder_supports::TransactionBuilder;
use crate::core::discover::peer_discovery::DiscoveryMessage;

use crate::core::internal_message::PeerMessage;
use crate::core::internal_message::RecvAsyncErrorInfo;
use crate::core::internal_message::TransactionMessage;
use crate::core::process_transaction::{RequestProcessor, UTXOContentionPool};
use redgold_data::data_store::DataStore;
use redgold_data::peer::PeerTrustQueryResult;
use redgold_keys::eth::eth_wallet::EthWalletWrapper;
use redgold_keys::request_support::{RequestSupport, ResponseSupport};
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::util::btc_wallet::SingleKeyBitcoinWallet;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::proto_serde::{ProtoHashable, ProtoSerde};
use redgold_schema::transaction::{amount_to_raw_amount, TransactionMaybeError};
use redgold_schema::util::lang_util::WithMaxLengthString;
use crate::core::transact::tx_builder_supports::TransactionBuilderSupport;
use redgold_schema::util::xor_distance::{xorf_conv_distance, xorfc_hash};
use crate::core::contract::contract_state_manager::ContractStateMessage;
use crate::node_config::NodeConfig;
use crate::schema::structs::{Observation, ObservationMetadata};
use crate::schema::SafeOption;
use crate::scrape::external_networks::ExternalNetworkData;
use crate::util;
use crate::util::keys::ToPublicKey;

#[derive(Clone)]
pub struct TransactionErrorCache {
    pub process_time: u64,
    pub error: ErrorCode,
}

#[derive(Clone)]
pub struct TrustUpdate {
    pub update: PeerMetadata,
    pub remove_peer: Option<Vec<u8>>,
}
//
// #[derive(Clone)]
// pub struct MultipartyRequestResponse {
//     pub request: Option<MultipartyThresholdRequest>,
//     pub response: Option<MultipartyThresholdResponse>,
//     pub sender: Option<flume::Sender<MultipartyThresholdResponse>>,
//     pub origin: Option<NodeMetadata>,
//     pub internal_subscribe: Option<MultipartyRoomInternalSubscribe>
// }
//
// impl MultipartyRequestResponse {
//
//     pub fn empty() -> Self {
//         Self {
//             request: None,
//             response: None,
//             sender: None,
//             origin: None,
//             internal_subscribe: None,
//         }
//     }
// }
//
// #[derive(Clone)]
// pub struct MultipartyRoomInternalSubscribe {
//     pub room_id: String,
//     pub sender: flume::Sender<MultipartySubscribeEvent>
// }

#[derive(Clone)]
pub struct ObservationMetadataInternalSigning {
    pub observation_metadata: ObservationMetadata,
    pub sender: flume::Sender<ObservationProof>
}


// TODO: There's a better pattern here than mutex, but wrapping this here for clarity
// for a future change.
#[derive(Clone, Default)]
pub struct ReadManyWriteOne<T> where T: Clone {
    pub inner: Arc<tokio::sync::Mutex<T>>
}

impl<T> ReadManyWriteOne<T> where T: Clone  {
    pub async fn write(&self, t: T) -> () {
        let mut guard = self.lock().await;
        *guard = t;
        ()
    }

    async fn lock(&self) -> MutexGuard<'_, T> {
        self.inner.lock().await
    }

    pub async fn clone_read(&self) -> T {
        let guard = self.lock().await;
        (*guard).clone()
    }

    pub async fn read(&self) -> MutexGuard<'_, T> {
        self.lock().await
    }

}

#[derive(Clone)]
pub struct Relay {
    /// Internal configuration
    pub node_config: NodeConfig,
    /// Incoming transactions
    pub mempool: Channel<TransactionMessage>,
    pub transaction_process: Channel<TransactionMessage>,
    /// Externally received observations TODO: Merge this into transaction
    pub observation: Channel<Transaction>,
    /// Threshold encryption multiparty signing flow
    // pub multiparty: Channel<MultipartyRequestResponse>,
    /// Internal signing stream for handling some validated data that is to be observed and signed
    pub observation_metadata: Channel<ObservationMetadataInternalSigning>,
    /// Incoming interface for receiving messages from other peers
    pub peer_message_tx: Channel<PeerMessage>,
    /// Outgoing interface for sending messages to other peers
    pub peer_message_rx: Channel<PeerMessage>,
    /// Internal persistent data storage on disk, main access instance for everything persistence
    /// related
    pub ds: DataStore,
    /// All transactions currently in process, this needs to incorporate a priority mempool
    /// And be updated to deal with priority queue + persisted processing transactions
    pub transaction_channels: Arc<DashMap<Hash, RequestProcessor>>,
    /// TODO: This really needs to incorporate some kind of UTXO stream handler?
    pub utxo_channels: Arc<DashMap<UtxoId, UTXOContentionPool>>,
    /// Some update associated with the trust model or change in rating label
    pub trust: Channel<TrustUpdate>,
    /// This isn't really used anywhere, but might be useful for keeping track of some kind of
    /// state information, most operations are actually designed to avoid any dependence on this
    /// but maybe useful for information purposes / observability
    pub node_state: Arc<AtomicCell<NodeState>>,
    /// Channel for outgoing messages over UDP, this streams to the UDP stream socket
    /// This should only be used internally by outgoing peer handler
    pub udp_outgoing_messages: Channel<PeerMessage>,
    /// Haha, discovery channel
    /// Used for immediate discovery messages for unknown or unrecognized message
    pub discovery: Channel<DiscoveryMessage>,
    /// Authorization channel for multiparty keygen to determine room_id and participating keys
    pub mp_keygen_authorizations: Arc<Mutex<HashMap<RoomId, InitiateMultipartyKeygenRequest>>>,
    pub mp_signing_authorizations: Arc<Mutex<HashMap<RoomId, InitiateMultipartySigningRequest>>>,
    pub contract_state_manager_channels: Vec<Channel<ContractStateMessage>>,
    pub contention: Vec<Channel<ContentionMessage>>,
    pub predicted_trust_overall_rating_score: Arc<Mutex<HashMap<PeerId, f64>>>,
    pub unknown_resolved_inputs: Channel<ResolvedInput>,
    pub mempool_entries: Arc<DashMap<Hash, Transaction>>,
    pub faucet_rate_limiter: Arc<Mutex<HashMap<String, (Instant, i32)>>>,
    pub tx_writer: Channel<TxWriterMessage>,
    pub peer_send_failures: Arc<tokio::sync::Mutex<HashMap<PublicKey, (ErrorInfo, i64)>>>,
    pub external_network_shared_data: ReadManyWriteOne<HashMap<PublicKey, PartyInternalData>>,
    pub btc_wallets: Arc<tokio::sync::Mutex<HashMap<PublicKey, Arc<tokio::sync::Mutex<SingleKeyBitcoinWallet<Tree>>>>>>,

}

impl Relay {

    pub async fn btc_wallet(&self, pk: &PublicKey) -> RgResult<Arc<tokio::sync::Mutex<SingleKeyBitcoinWallet<Tree>>>> {
        let mut guard = self.btc_wallets.lock().await;
        let result = guard.get(pk);
        let mutex = match result {
            Some(w) => {
                w.clone()
            }
            None => {
                let new_wallet = SingleKeyBitcoinWallet::new_wallet_db_backed(
                    pk.clone(), self.node_config.network.clone(), true,
                    self.node_config.env_data_folder().bdk_sled_path()
                )?;
                let w = Arc::new(tokio::sync::Mutex::new(new_wallet));
                guard.insert(pk.clone(), w.clone());
                w
            }
        };
        Ok(mutex)
    }

    pub fn eth_wallet(&self) -> RgResult<EthWalletWrapper> {
        EthWalletWrapper::new(&self.node_config.keypair().to_private_hex(), &self.node_config.network)
    }

    pub fn default_fee_addrs(&self) -> Vec<Address> {
        self.node_config.seed_peer_addresses()
    }

    pub async fn discover_peer(&self, nmd: &NodeMetadata) -> RgResult<()> {
        self.discovery.send(DiscoveryMessage::new(nmd.clone(), None)).await
    }
    pub async fn write_transaction(
        &self,
        transaction: &Transaction,
        time: i64,
        rejection_reason: Option<ErrorInfo>,
        update_utxo: bool
    ) -> Result<(), ErrorInfo> {
        let (s,r) = flume::bounded(1);
        self.tx_writer.sender.send_rg_err(
            TxWriterMessage::WriteTransaction(TransactionWithSender {
                transaction: transaction.clone(),
                sender: s,
                time,
                rejection_reason,
                update_utxo,
            })
        )?;
        // TODO: NodeConfig specific timeout
        r.recv_async_err_timeout(self.node_config.default_timeout).await??;
        Ok(())
    }


    pub async fn mark_peer_send_failure(&self, pk: &PublicKey, error: &ErrorInfo) -> RgResult<()> {
        let mut l = self.peer_send_failures.safe_lock().await?;
        let mut v = l.get(pk).map(|v| v.clone()).unwrap_or(
            (error.clone(), util::current_time_millis_i64()));
        l.insert(pk.clone(), v);
        Ok(())
    }

    pub async fn mark_peer_send_success(&self, pk: &PublicKey) -> RgResult<()> {
        let mut l = self.peer_send_failures.safe_lock().await?;
        l.remove(pk);
        Ok(())
    }

    pub fn check_rate_limit(&self, ip: &String) -> RgResult<bool> {
        let mut l = self.faucet_rate_limiter.lock()
            .map_err(|e| error_info(format!("Failed to lock faucet_rate_limiter {}", e.to_string())))?;
        if l.len() > 1_000_000 {
            Err(error_info("Faucet rate limiter has exceeded 1 million entries, this is a problem"))?;
        }
        let now = Instant::now();
        match l.get(ip) {
            None => {
                l.insert(ip.clone(), (now, 0));
                Ok(true)
            }
            Some((v, count)) => {
                let greater_than_a_day = now.duration_since(v.clone()).as_secs() > (3600*24);
                let count2 = count.clone();
                let count_exceeded = count2 > 30i32;

                if greater_than_a_day {
                    l.insert(ip.clone(), (now, 1));
                    Ok(true)
                } else {
                    if count_exceeded {
                        Ok(false)
                    } else {
                        l.insert(ip.clone(), (now, count2 + 1));
                        Ok(true)
                    }
                }
            }
        }
    }

    pub async fn transaction_known(&self, hash: &Hash) -> RgResult<bool> {
        if self.mempool_entries.contains_key(hash) {
            return Ok(true);
        }
        if self.transaction_channels.contains_key(hash) {
            return Ok(true);
        }
        if self.ds.transaction_store.query_maybe_transaction(hash).await?.is_some() {
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn lookup_transaction_maybe_error_hex(&self, hash: impl Into<String>) -> RgResult<Option<TransactionMaybeError>> {
        let hash = Hash::from_hex(hash)?;
        self.lookup_transaction_maybe_error(&hash).await
    }
    pub async fn lookup_transaction_maybe_error(&self, hash: &Hash) -> RgResult<Option<TransactionMaybeError>> {
        let res = match self.lookup_transaction(hash).await? {
            None => {
                self.ds.transaction_store.query_rejected_transaction(hash).await?.map(
                    |(t, e)| TransactionMaybeError::new_error(t, e)
                )
            }
            Some(t) => {
                Some(TransactionMaybeError::new(t))
            }
        };
        Ok(res)
    }

    pub async fn lookup_transaction(&self, hash: &Hash) -> RgResult<Option<Transaction>> {
        let res = match self.mempool_entries.get(hash)
            .map(|t| t.clone())
            .or_else( ||
                self.transaction_channels.get(hash).map(|t| t.transaction.clone())
            )
        {
            None => {
                self.ds.transaction_store.query_accepted_transaction(hash).await?
            }
            Some(t) => {
                Some(t)
            }
        };

        Ok(res)
    }
}


impl Relay {



}

/**
Deliberately unclone-able structure that tracks strict unshared dependencies which
are instantiated by the node
*/

use crate::core::internal_message::SendErrorInfo;
use crate::core::transport::peer_rx_event_handler::PeerRxEventHandler;
use crate::core::resolver::{resolve_input, ResolvedInput, validate_single_result};
use crate::core::transact::contention_conflicts::{ContentionMessage, ContentionMessageInner, ContentionResult};
use crate::core::transact::tx_writer::{TransactionWithSender, TxWriterMessage};
use crate::e2e::tx_gen::SpendableUTXO;
use crate::party::data_enrichment::PartyInternalData;

pub struct StrictRelay {}
// Relay should really construct a bunch of non-clonable channels and return that data
// as the other 'half' here.
impl Relay {

    pub async fn get_trust(&self) -> RgResult<HashMap<PeerId, f64>> {

        let peer_tx = self.peer_tx().await?;
        let pd = peer_tx.peer_data()?;

        let mut hm = self.predicted_trust_overall_rating_score.lock().map_err(
            |e| error_info(format!(
                "Failed to lock predicted_trust_overall_rating_score {}", e.to_string()))
        )?.clone();

        for label in pd.labels {
            let peer_id: Option<PeerId> = label.peer_id;
            if let Some(peer_id) = peer_id {
                for d in label.trust_data {
                    if !d.allow_model_override {
                        if let Some(r) = d.maybe_label() {
                            hm.insert(peer_id.clone(), r);
                        }
                    }
                }
            }
        }

        let seed_scores = self.node_config.seeds_now().iter().filter_map(|s|
            s.peer_id.clone().map(|p| (p, s.trust.get(0).map(|t| t.label()).unwrap_or(0.8)))
        ).collect::<HashMap<PeerId, f64>>();

        for (k, v) in seed_scores {
            if !hm.contains_key(&k) {
                hm.insert(k, v);
            }
        }

        Ok(hm)
        // Err(error_info("test"))
    }

    pub async fn is_seed(&self, pk: &PublicKey) -> bool {
        self.node_config.seeds_now().iter()
            .filter(|s| s.public_key.as_ref().filter(|&p| p == pk).is_some())
            .next().is_some()
    }

    pub async fn seed_trust(&self, pk: &PublicKey) -> Option<Vec<TrustData>> {
        self.node_config.seeds_now().iter()
            .filter(|s| s.public_key.as_ref().filter(|&p| p == pk).is_some())
            .next().map(|s| s.trust.clone())
    }

    // Refactor this and below later to optimize lookup.
    pub async fn get_trust_of_peer(&self, peer_id: &PeerId) -> RgResult<Option<f64>> {
        let hm = self.get_trust().await?;
        Ok(hm.get(peer_id).cloned())
    }

    pub async fn get_security_rating_trust_of_node(&self, public_key: &PublicKey) -> RgResult<Option<f64>> {
        let hm = self.get_trust().await?;
        Ok(self.ds.peer_store.peer_id_for_node_pk(public_key).await?
            .and_then(|p| hm.get(&p).cloned()))
    }

    pub async fn peer_id_for_node_pk(&self, public_key: &PublicKey) -> RgResult<Option<PeerId>> {
        if &self.node_config.public_key() == public_key {
            return Ok(Some(self.peer_id_from_node_tx().await?))
        }
        self.ds.peer_store.peer_id_for_node_pk(public_key).await
    }

    pub async fn get_trust_of_node_as_query(&self, public_key: &PublicKey) -> RgResult<Option<PeerTrustQueryResult>> {
        let hm = self.get_trust().await?;
        let pid = self.ds.peer_store.peer_id_for_node_pk(public_key).await?;
        if let Some(pid) = pid {
            if let Some(t) = hm.get(&pid).cloned() {
                return Ok(Some(PeerTrustQueryResult {
                    peer_id: pid,
                    trust: t,
                }))
            }
        }
        Ok(None)
    }

    pub async fn contention_message(&self, key: &ContentionKey, msg: ContentionMessageInner) -> RgResult<Receiver<RgResult<ContentionResult>>> {
        let (s, r) = flume::bounded::<RgResult<ContentionResult>>(1);
        let msg = ContentionMessage::new(&key, msg, s);
        let index = key.div_mod(self.node_config.contention.bucket_parallelism.clone());
        self.contention[index as usize].sender.send_rg_err(msg)?;
        Ok(r)
    }

    pub async fn send_contract_ordering_message(&self, tx: &Transaction, output: &Output) -> RgResult<ContractStateMarker> {
        let ck = output.request_contention_key()?;
        let h = ck.div_mod(self.node_config.contract.bucket_parallelism.clone());
        let c = self.contract_state_manager_channels.get(h as usize).expect("missing channel");
        let (s,r) = flume::bounded::<RgResult<ContractStateMarker>>(1);
        let msg = ContractStateMessage::ProcessTransaction {
                transaction: tx.clone(),
                output: output.clone(),
                response: s
        };
        c.sender.send_rg_err(msg)?;
        r.recv_async_err().await?
    }

    pub async fn node_tx(&self) -> RgResult<Transaction> {
        let tx = self.ds.config_store.get_node_tx().await?;
        if let Some(tx) = tx {
            Ok(tx)
        } else {
            let pd = self.peer_tx().await?.peer_data()?;
            let matching_peer_tx = pd.node_metadata.iter().filter(
                |nmd| nmd.public_key == Some(self.node_config.public_key())
            ).collect_vec();
            trace!("Node tx node metadata length: {}", pd.node_metadata.len());
            let opt = matching_peer_tx.get(0).cloned();
            if opt.is_none() {
                info!("No peer tx found for this node, generating new one");
            }
            trace!("First generation of node tx from peer tx: {:?}", opt.cloned());
            let tx = self.node_config.node_tx_fixed(opt);
            self.ds.config_store.set_node_tx(&tx).await?;
            Ok(tx)
        }
    }

    pub async fn partition_info(&self) -> RgResult<Option<PartitionInfo>> {
        Ok(self.node_metadata().await?.partition_info)
    }

    pub async fn tx_hash_distance(&self, hash: &Hash) -> RgResult<bool> {
        let d = xorfc_hash(hash, &self.node_config.public_key());
        let pi = self.partition_info().await?;
        Ok(pi.and_then(|pi| pi.transaction_hash)
            .map(|d_max| d < d_max).unwrap_or(true))
    }

    pub async fn utxo_hash_distance(&self, utxo_id: &UtxoId) -> RgResult<bool> {
        let vec = utxo_id.utxo_id_vec();
        let marker = self.node_config.public_key().vec();
        let d = xorf_conv_distance(&vec, &marker);
        let pi = self.partition_info().await?;
        Ok(pi.and_then(|pi| pi.utxo)
            .map(|d_max| d < d_max).unwrap_or(true))
    }

    pub async fn peer_tx(&self) -> RgResult<Transaction> {
        let tx = self.ds.config_store.get_peer_tx().await?;
        if let Some(tx) = tx {
            Ok(tx)
        } else {
            let tx = self.node_config.peer_tx_fixed();
            self.ds.config_store.set_peer_tx(&tx).await?;
            Ok(tx)
        }
    }

    pub async fn last_6_peer_id(&self) -> RgResult<String> {
        Ok(self.peer_tx().await?.peer_data()?
            .peer_id.ok_msg("Peer ID not found")?.peer_id.ok_msg("Peer ID not found")?
            .hex().last_n(6))
    }

    pub async fn gauge_labels(&self) -> RgResult<[(String, String); 2]> {
        let last_6 = self.last_6_peer_id().await?;
        let labels = [
            ("public_key".to_string(), self.node_config.short_id().expect("short id")),
            ("peer_id".to_string(), last_6),
        ];
        Ok(labels)
    }

    pub async fn peer_id_from_node_tx(&self) -> RgResult<PeerId> {
        let res = self.node_tx()
            .await
            .and_then(|n| n.node_metadata())
            .and_then(|n| n.peer_id.ok_or(error_info("Missing peer_id")));
        res
    }

    pub async fn dynamic_node_metadata(&self) -> RgResult<DynamicNodeMetadata> {
        let tx = self.ds.config_store.get_dynamic_md().await?;
        if let Some(tx) = tx {
            Ok(tx)
        } else {
            let tx = self.node_config.dynamic_node_metadata_fixed();
            self.ds.config_store.set_dynamic_md(&tx).await?;
            Ok(tx)
        }
    }

    pub async fn peer_node_info(&self) -> RgResult<PeerNodeInfo> {
        Ok(PeerNodeInfo {
            latest_peer_transaction: Some(self.peer_tx().await?),
            latest_node_transaction: Some(self.node_tx().await?),
            dynamic_node_metadata: Some(self.dynamic_node_metadata().await?),
        })
    }

    // TODO: This is incorrect, it should issue queries to each node to get their latest
    // Otherwise rely on data store query for each public key.
    pub async fn peer_id_info(&self) -> RgResult<PeerIdInfo> {
        Ok(PeerIdInfo {
            latest_peer_transaction: Some(self.peer_tx().await?),
            peer_node_info: vec![self.peer_node_info().await?],
        })
    }


    pub async fn update_dynamic_node_metadata(&self, d: &DynamicNodeMetadata) -> RgResult<()> {
        let d2 = d.clone();
        // TODO: Sign here, increment height.
        self.ds.config_store.set_dynamic_md(&d2).await?;
        Ok(())
    }

    // TODO: Unify with liveE2E, something weird going on between these functions
    pub async fn get_self_fee_utxo(&self) -> RgResult<Option<UtxoEntry>> {
        let address = self.node_config.public_key().address()?;
        let result = self.ds.transaction_store.query_utxo_address(&address).await?;
        let vec = result.iter()
            .filter(|r| r.opt_amount().map(|a| a.amount > CurrencyAmount::min_fee().amount).unwrap_or(false))
            .collect_vec();
        let mut err_str = format!("Address {}", address.render_string().expect(""));
        let mut res = None;
        for u in vec {
            if let Ok(id) = u.utxo_id() {
                if self.utxo_channels.contains_key(id) {
                    continue
                }
                err_str.push_str(&format!(" UTXO ID: {}", id.json_or()));
                if self.ds.utxo.utxo_id_valid(id).await? {
                    let childs = self.ds.utxo.utxo_children(id).await?;
                    if childs.len() == 0 {
                        res = Some(u.clone());
                        break
                    } else {
                        error!("UTXO has children not valid! {} {}", err_str, childs.json_or());
                    }
                } else {
                    error!("UTXO ID not valid! {}", err_str);
                }
            }
        }
        Ok(res)
    }

    pub async fn update_node_metadata(&self, node_metadata: &NodeMetadata) -> RgResult<()> {
        // let tx = self.node_tx().await?;
        let mut tx_b = TransactionBuilder::new(&self.node_config);
        tx_b.allow_bypass_fee = true;

        // let option = self.get_self_fee_utxo().await?;
        // let has_currency = option.is_some();
        // if let Some(utxo) = option {
        //     tx_b.with_unsigned_input(utxo)?;
        // }
        //
        // let utxo = tx.nmd_utxo()?;
        // let h = utxo.height()?;
        let address = self.node_config.public_key().address()?;
        // tx_b.with_nmd_utxo(&utxo)?;
        // tx_b.with_output_node_metadata(&address, node_metadata.clone(), h+1);
        tx_b.with_output_node_metadata(&address, node_metadata.clone(), 1);
        // TODO: Add fee here from internal node wallet
        // if has_currency {
        //     tx_b.with_default_fee()?;
        // };

        let updated = tx_b.build()?.sign(&self.node_config.keypair())?;
        self.ds.config_store.set_node_tx(&updated).await?;
        // TODO:
        // Really we should just gossip the transaction here.. or rely on discovery
        // let _ = self.submit_transaction_with(&updated, false).await?;
        Ok(())
    }

    // Don't use this until everything else is updated.
    // pub async fn update_nmd_auto(&self) -> RgResult<()> {
    //     let nmd = self.update_with_version_info().await?;
    //     self.update_node_metadata(&nmd).await?;
    //     Ok(())
    // }

    pub async fn update_with_live_info(&self, nmd: NodeMetadata) -> RgResult<NodeMetadata> {
        let mut nmd = nmd.clone();
        let vii = self.node_config.version_info();
        if let Some(vi) = nmd.version_info.as_mut() {
            vi.commit_hash = vii.commit_hash;
            vi.executable_checksum = vii.executable_checksum;
            vi.build_number = vii.build_number;
        };
        Ok(nmd)
    }

    // pub async fn force_update_nmd_auto_peer_tx(&self) -> RgResult<()> {
    //     let tx = self.node_config.peer_tx_fixed();
    //     self.ds.config_store.set_peer_tx(&tx).await?;
    //     let nmd = self.node_config.node_tx_fixed(None);
    //     self.ds.config_store.set_node_tx(&nmd).await?;
    //     Ok(())
    // }

    pub async fn add_party_id(&self, d: &PartyId) -> RgResult<()> {
        let mut nmd = self.node_metadata().await?;
        let contains = nmd.parties.iter()
            .filter(|d2| d2.public_key == d.public_key)
            .next().is_some();
        if contains {
            return Ok(());
        }
        nmd.parties.push(d.clone());
        self.update_node_metadata(&nmd).await
    }

    pub async fn sign_request(&self, req: Request) -> RgResult<Request> {
        Ok(req
            .with_metadata(self.node_metadata().await?)
            .with_auth(&self.node_config.keypair()).clone())
    }


    pub async fn node_metadata(&self) -> RgResult<NodeMetadata> {
        self.node_tx().await?.node_metadata()
    }

    pub fn authorize_signing(&self, p0: InitiateMultipartySigningRequest) -> RgResult<()> {
        let mut l = self.mp_signing_authorizations.lock().map_err(|e| error_info(format!("Failed to lock mp_authorizations {}", e.to_string())))?;
        l.insert(p0.signing_room_id.safe_get_msg("room id signing missing")?.clone(), p0);
        Ok(())
    }
    pub fn remove_signing_authorization(&self, room_id: &RoomId) -> RgResult<()> {
        let mut l = self.mp_signing_authorizations.lock().map_err(|e| error_info(format!("Failed to lock mp_authorizations {}", e.to_string())))?;
        l.remove(room_id);
        Ok(())
    }
    pub fn check_signing_authorized(&self, room_id: &RoomId, public_key: &structs::PublicKey) -> RgResult<Option<usize>> {
        let l = self.mp_signing_authorizations.lock().map_err(|e| error_info(format!("Failed to lock mp_authorizations {}", e.to_string())))?;
        Ok(l.get(room_id).safe_get_msg("missing room_id")
            .and_then(|x| x.identifier.safe_get_msg("missing identifier")
            ).map(|p| p.party_index(public_key)).unwrap_or(None))
    }

    pub fn authorize_keygen(&self, p0: InitiateMultipartyKeygenRequest) -> RgResult<()> {
        let mut l = self.mp_keygen_authorizations.lock().map_err(|e| error_info(format!("Failed to lock mp_authorizations {}", e.to_string())))?;
        l.insert(p0.identifier.safe_get_msg("missing identifier")?.room_id.safe_get_msg("rid")?.clone(), p0);
        Ok(())
    }
    pub fn remove_keygen_authorization(&self, room_id: &RoomId) -> RgResult<()> {
        let mut l = self.mp_keygen_authorizations.lock().map_err(|e| error_info(format!("Failed to lock mp_authorizations {}", e.to_string())))?;
        l.remove(room_id);
        Ok(())
    }
    pub fn check_keygen_authorized(&self, room_id: &RoomId, public_key: &structs::PublicKey) -> RgResult<Option<usize>> {
        let l = self.mp_keygen_authorizations.lock().map_err(|e| error_info(format!("Failed to lock mp_authorizations {}", e.to_string())))?;
        Ok(l.get(room_id).safe_get_msg("missing room_id")
            .and_then(|x| x.identifier.safe_get_msg("missing identifier")
            ).map(|p| p.party_index(public_key)).unwrap_or(None))
    }
    pub fn check_mp_authorized(&self, room_id: &RoomId, public_key: &structs::PublicKey) -> RgResult<Option<usize>> {
        let room_id = room_id.uuid.safe_get()?.clone();
        let stripped_one = room_id.strip_suffix("-online").unwrap_or(room_id.as_str());
        let room_id = stripped_one.strip_suffix("-offline").unwrap_or(stripped_one).to_string();
        let room_id = RoomId::from(room_id);
        Ok(self.check_keygen_authorized(&room_id, public_key)?.or(self.check_signing_authorized(&room_id, public_key)?))
    }


    pub async fn observe(&self, mut om: ObservationMetadata) -> Result<ObservationProof, ErrorInfo> {
        om.with_hash();
        let (sender, r) = flume::unbounded::<ObservationProof>();
        let omi = ObservationMetadataInternalSigning {
            observation_metadata: om,
            sender,
        };
        self.observation_metadata.sender.send_rg_err(omi)?;
        let res = tokio::time::timeout(
            Duration::from_secs(self.node_config.observation_formation_millis.as_secs() + 60),
            r.recv_async_err()
        ).await.error_info("Timeout waiting for internal observation formation")??;
        Ok(res)
    }

    pub async fn observe_tx(
        &self,
        tx_hash: &Hash,
        state: State,
        validation_type: ValidationType,
        liveness: structs::ValidationLiveness
    ) -> Result<ObservationProof, ErrorInfo> {
        let mut hash = tx_hash.clone();
        hash.hash_type = HashType::Transaction as i32;
        let mut om = structs::ObservationMetadata::default();
        om.observed_hash = Some(hash);
        om.state = state as i32;
        om.struct_metadata = struct_metadata_new();
        om.observation_type = validation_type as i32;
        om.validation_liveness = liveness as i32;
            // TODO: It might be nice to grab the proof of a signature here?
        self.observe(om).await
    }

    // TODO: add timeout
    pub async fn send_message_await_response(&self, request: Request, node: structs::PublicKey, timeout: Option<Duration>) -> Result<Response, ErrorInfo> {
        let timeout = timeout.unwrap_or(Duration::from_secs(60));
        let (s, r) = flume::unbounded::<Response>();
        let mut pm = PeerMessage::from_pk(&request, &node.clone());
        pm.response = Some(s);
        self.peer_message_tx.sender.send_rg_err(pm)?;
        let res = tokio::time::timeout(timeout, r.recv_async_err()).await
            .map_err(|e| error_info(e.to_string()))??;
        Ok(res)
    }

    // Try to eliminate this function
    pub async fn send_message_sync_static(relay: Relay, request: Request, node: structs::PublicKey, timeout: Option<Duration>) -> Result<Response, ErrorInfo> {
        let (s, r) = flume::bounded::<Response>(1);
        let mut pm = PeerMessage::from_pk(&request, &node.clone());
        pm.response = Some(s);
        if let Some(t) = timeout {
            pm.send_timeout = t;
        }
        relay.peer_message_tx.sender.send_rg_err(pm)?;
        let res = r.recv_async_err().await?;
        res.as_error_info()?;
        Ok(res)
    }

    pub async fn receive_request_send_internal(&self, request: Request, timeout: Option<Duration>) -> Result<Response, ErrorInfo> {
        let timeout = timeout.unwrap_or(self.node_config.default_timeout.clone());
        let (s, r) = flume::bounded::<Response>(1);
        let key = request.proof.clone().and_then(|p| p.public_key);
        let mut pm = PeerMessage::empty();
        pm.request = request;
        pm.response = Some(s);
        pm.public_key = key;
        self.peer_message_rx.sender.send_rg_err(pm).add("receive_message_sync")?;
        let res = r.recv_async_err_timeout(timeout).await?;
        Ok(res)
    }

    pub async fn gossip(&self, tx: &Transaction) -> Result<(), ErrorInfo> {
        counter!("redgold_gossip_outgoing").increment(1);
        let all = self.ds.peer_store.select_gossip_peers(tx).await?;
        trace!("Gossiping transaction to {} peers {}", all.len(), all.json_or());
        for p in all {
            let mut req = Request::default();
            let mut gtr = GossipTransactionRequest::default();
            gtr.transaction = Some(tx.clone());
            req.gossip_transaction_request = Some(gtr);
            self.send_message(req, p).await?;
        }
        Ok(())
    }

    pub async fn gossip_req(&self, req: &Request, hash: &Hash) -> Result<(), ErrorInfo> {
        let all = self.ds.peer_store.peers_near(hash, |p| p.transaction_hash).await?;
        for p in all {
            self.send_message(req.clone(), p).await?;
        }
        Ok(())
    }

    pub async fn utxo_id_valid_peers(&self, utxo_id: &UtxoId) -> RgResult<Option<Transaction>> {
        let peers = self.ds.peer_store
            .peers_near(&utxo_id.as_hash(), |p| p.utxo).await?;
        let mut request = Request::empty();
        request.utxo_valid_request = Some(utxo_id.clone());
        let res = self.broadcast_async(peers, request, Some(Duration::from_secs(10))).await?;
        // verify majority here.
        let mut sum: f64 = 0.;
        let mut hm: HashMap<Transaction, f64> = HashMap::new();
        for r in res {
            if let Ok(r) = &r {
                if let Some(pk) = r.proof.as_ref().and_then(|p| p.public_key.as_ref()) {
                    if let Some(utxo_r) = &r.utxo_valid_response {
                        if let Some(r) = &utxo_r.valid {
                            if let Some(t) = self.get_security_rating_trust_of_node(pk).await? {
                                if r.clone() {
                                    sum += t;
                                } else {
                                    sum -= t;
                                }
                                if let (Some(h), Some(_i)) = (&utxo_r.child_transaction, &utxo_r.child_transaction_input) {
                                    if hm.contains_key(h) {
                                        hm.insert(h.clone(), hm.get(h).unwrap() + t);
                                    } else {
                                        hm.insert(h.clone(), t);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        let tx = hm.iter()
            .max_by(|a, b| a.1.partial_cmp(b.1)
                .unwrap_or(Ordering::Equal)).map(|(h, _)| h.clone());
        if sum > 0. {
            Ok(None)
        } else {
            Ok(tx)
        }
    }

    pub async fn health_request(&self, nodes: &Vec<PublicKey>) -> RgResult<Vec<RgResult<Response>>> {
        let mut req = Request::default();
        req.health_request = Some(HealthRequest::default());
        self.broadcast_async(nodes.clone(), req, None).await
    }

    pub async fn health_request_all(&self, nodes: &Vec<PublicKey>) -> RgResult<()> {
        for r in self.health_request(nodes).await? {
            r?;
        }
        Ok(())
    }

    // the new function everything should use
    pub async fn broadcast_async(
        &self,
        nodes: Vec<PublicKey>,
        request: Request,
        timeout: Option<Duration>
    ) -> RgResult<Vec<RgResult<Response>>> {
        let mut results = vec![];
        for p in nodes {
            let req = request.clone();
            let timeout = Some(timeout.unwrap_or(Duration::from_secs(30)));
            let res = self.send_message_async(&req, &p, timeout).await?;
            results.push((p, res));
        }
        let mut responses = vec![];
        for (pk, r) in &results {
            let fut = async {
                let result = r.recv_async_err().await;
                result
            };
            responses.push(fut);
        }
        let res = futures::future::join_all(responses).await;
        Ok(res)
    }

    pub async fn lookup_transaction_serial(&self, h: &Hash) -> RgResult<Option<Transaction>> {
         let peers = self.ds.peer_store
                .peers_near(&h, |p| p.transaction_hash).await?;
        let mut request = Request::empty();
        request.lookup_transaction_request = Some(h.clone());
        for p in peers {
            let res = self.send_message_await_response(request.clone(), p, Some(Duration::from_secs(10))).await;
            if let Ok(r) = res {
                if let Some(t) = r.lookup_transaction_response {
                    if &t.hash_or() == h {
                        return Ok(Some(t))
                    }
                }
            }
        }
        return Ok(None)
            // verify majority here.
    }

    // old function
    pub async fn broadcast(
        relay: Relay,
        nodes: Vec<structs::PublicKey>,
        request: Request,
        // runtime: Arc<Runtime>,
        timeout: Option<Duration>
        // TODO: remove the publickey here not necessary
    ) -> Vec<(structs::PublicKey, Result<Response, ErrorInfo>)> {
        let timeout = timeout.unwrap_or(Duration::from_secs(20));
        // let mut fu = FuturesUnordered::new();
        let mut fu = vec![];
        for (_,node) in nodes.iter().enumerate() {
            let relay2 = relay.clone();
            // let runtime2 = runtime.clone();
            let request2 = request.clone();
            let jh = async move {
                (
                node.clone(),
                {

                    tokio::spawn(
                        Relay::send_message_sync_static(relay2.clone(),
                                                        request2.clone(), node.clone(), Some(timeout))
                    ).await.error_info("join handle failure on broadcast").and_then(|e| e)
                }
            )};
            fu.push(jh);
        }

        future::join_all(fu).await
    }

    pub async fn send_message(&self, request: Request, node: structs::PublicKey) -> Result<(), ErrorInfo> {
        let pm = PeerMessage::from_pk(&request, &node);
        self.peer_message_tx.sender.send_rg_err(pm)?;
        Ok(())
    }


    pub async fn send_message_async(
        &self,
        request: &Request,
        node: &PublicKey,
        timeout: Option<Duration>
    ) -> Result<flume::Receiver<Response>, ErrorInfo> {
        let (s, r) = flume::bounded(1);
        let mut pm = PeerMessage::from_pk(&request, &node);
        pm.response = Some(s);
        if let Some(t) = timeout {
            pm.send_timeout = t;
        }
        self.peer_message_tx.sender.send_rg_err(pm)?;
        Ok(r)
    }

    pub async fn send_message_async_pm(&self, pm: PeerMessage) -> Result<flume::Receiver<Response>, ErrorInfo> {
        let (_s, r) = flume::bounded(1);
        self.peer_message_tx.send(pm).await?;
        Ok(r)
    }


    pub async fn send_message_sync_pm(&self, mut pm: PeerMessage, timeout: Option<Duration>) -> RgResult<Response> {
        let (s, r) = flume::bounded(1);
        pm.response = Some(s);
        let duration = timeout.unwrap_or(Duration::from_secs(20));
        pm.send_timeout = duration;
        self.peer_message_tx.send(pm).await?;
        r.recv_async_err().await
    }


    pub async fn submit_transaction_sync(
        &self,
        tx: &Transaction,
    ) -> Result<SubmitTransactionResponse, ErrorInfo> {
        self.submit_transaction(SubmitTransactionRequest{
            transaction: Some(tx.clone()),
            sync_query_response: true,
        }, None, None).await
    }

    pub async fn submit_transaction(
        &self,
        tx_req: SubmitTransactionRequest,
        origin: Option<PublicKey>,
        origin_ip: Option<String>,
    ) -> Result<SubmitTransactionResponse, ErrorInfo> {
        let (s, r) = flume::bounded(1);
        let response_channel = if tx_req.sync_query_response {
            Some(s)
        } else {
            None
        };
        let mut tx = tx_req
            .transaction
            .safe_get_msg("Missing transaction field on submit request")?
            .clone();
        tx.with_hash();
        // info!("Relay submitting transaction");
        self.mempool
            .send(TransactionMessage {
                transaction: tx.clone(),
                response_channel,
                origin,
                origin_ip,
            })
            .await?;

        let mut response = SubmitTransactionResponse {
            transaction_hash: tx.clone().hash_or().into(),
            query_transaction_response: None,
            transaction: Some(tx.clone()),
        };
        if tx_req.sync_query_response {
            let response1 = r.recv_async_err().await?;
            response1.as_error_info()?;
            response = response1.submit_transaction_response.safe_get()?.clone();
            return Ok(response);
        }
        Ok(response)
    }

    pub async fn default() -> Self {
        Self::new(NodeConfig::default_debug()).await
    }
    pub async fn new(node_config: NodeConfig) -> Self {
        // Inter thread processes
        let ds = node_config.data_store().await;
        let mut contract_state_manager_channels = vec![];
        for _ in 0..node_config.contract.bucket_parallelism {
            contract_state_manager_channels.push(
                internal_message::new_bounded_channel::<ContractStateMessage>(
                    node_config.contract.contract_state_channel_bound.clone()
                )
            );
        }
        let mut contention = vec![];
        for _ in 0..node_config.contention.bucket_parallelism {
            contention.push(
                internal_message::new_bounded_channel::<ContentionMessage>(
                    node_config.contention.channel_bound.clone()
                )
            );
        }

        Self {
            node_config: node_config.clone(),
            mempool: internal_message::new_bounded_channel::<TransactionMessage>(node_config.mempool.channel_bound),
            transaction_process: internal_message::new_bounded_channel(node_config.tx_config.channel_bound),
            // TODO: Remove and merge this into tx
            observation: internal_message::new_channel::<Transaction>(),
            // multiparty: internal_message::new_channel::<MultipartyRequestResponse>(),
            observation_metadata: internal_message::new_channel::<ObservationMetadataInternalSigning>(),
            peer_message_tx: internal_message::new_channel::<PeerMessage>(),
            peer_message_rx: internal_message::new_channel::<PeerMessage>(),
            ds,
            transaction_channels: Arc::new(DashMap::new()),
            utxo_channels: Arc::new(DashMap::new()),
            trust: internal_message::new_channel::<TrustUpdate>(),
            node_state: Arc::new(AtomicCell::new(NodeState::Initializing)),
            udp_outgoing_messages: internal_message::new_channel::<PeerMessage>(),
            discovery: internal_message::new_bounded_channel(100),
            mp_keygen_authorizations: Arc::new(Mutex::new(Default::default())),
            mp_signing_authorizations: Arc::new(Mutex::new(Default::default())),
            contract_state_manager_channels,
            contention,
            predicted_trust_overall_rating_score: Arc::new(Mutex::new(Default::default())),
            unknown_resolved_inputs: internal_message::new_channel(),
            mempool_entries: Arc::new(Default::default()),
            faucet_rate_limiter: Arc::new(Mutex::new(Default::default())),
            tx_writer: new_channel(),
            peer_send_failures: Arc::new(Default::default()),
            external_network_shared_data: Default::default(),
            btc_wallets: Arc::new(Default::default()),
        }
    }
}


impl Relay {
    pub(crate) async fn dev_default() -> Self {
        Self::new(NodeConfig::dev_default().await).await
    }
}



#[async_trait]
pub trait SafeLock<T> where T: ?Sized + std::marker::Send {
    async fn safe_lock(&self) -> RgResult<tokio::sync::MutexGuard<T>>;
}
#[async_trait]
impl<T> SafeLock<T> for tokio::sync::Mutex<T> where T: ?Sized + std::marker::Send {
    async fn safe_lock(&self) -> RgResult<tokio::sync::MutexGuard<T>> {
        // May need a timeout here in the future, hence wrapping it
        Ok(self.lock().await)
    }
}

// https://doc.rust-lang.org/book/ch15-04-rc.html
