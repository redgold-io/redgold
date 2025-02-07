use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use redgold_common::external_resources::PeerBroadcast;
use redgold_common::flume_send_help::SendErrorInfo;
use redgold_common_no_wasm::ssh_like::{LocalSSHLike, SSHOrCommandLike};
use redgold_common_no_wasm::stream_handlers::{IntervalFoldOrReceive, TryRecvForEach};
use redgold_keys::monero::node_wrapper::{MoneroNodeRpcInterfaceWrapper, MoneroWalletMultisigRpcState, PartySecretInstanceData};
use redgold_keys::monero::rpc_multisig::MoneroWalletRpcMultisigClient;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::{RgResult, SafeOption};
use redgold_schema::structs::PublicKey;
use redgold_schema::util::times::current_time_millis;

#[derive(Clone)]
pub struct ActiveOperation {
    pub id: String,
    pub started_at: i64,
    pub join_handle: Arc<JoinHandle<()>>,
    pub on_final_stage: bool
}

#[derive(Clone)]
pub struct MoneroWalletSyncWriter<B: PeerBroadcast + 'static> {
    pub wallet_interface: Arc<Mutex<MoneroNodeRpcInterfaceWrapper<LocalSSHLike>>>,
    pub operation: Option<ActiveOperation>,
    pub peer_broadcast: B
}

#[derive(Clone, Debug)]
pub enum MoneroWalletMessage {
    MultisigCreateNext,
    CreateMultsigAsProposer
}

#[derive(Clone, Debug)]
pub struct MoneroSyncInteraction {
    pub message: MoneroWalletMessage,
    pub wallet_id: String,
    pub all_pks: Vec<PublicKey>,
    pub peer_strings: Vec<String>,
    pub threshold: i64,
    pub response: flume::Sender<RgResult<MoneroWalletResponse>>,
    pub operation_initialization: bool
}

#[derive(Clone, Debug)]
pub enum MoneroWalletResponse {
    PeerCreate(String),
    InstanceCreate(PartySecretInstanceData)
}

#[async_trait]
pub trait MoneroWalletSender {
    async fn send(&self, message: MoneroWalletMessage) -> RgResult<()>;
}

#[async_trait]
impl<B: 'static> TryRecvForEach<MoneroSyncInteraction> for MoneroWalletSyncWriter<B> where B: PeerBroadcast {
    async fn try_recv_for_each(&mut self, message: MoneroSyncInteraction) -> RgResult<()> {
        let ct = current_time_millis();
        if let Some(o) = &self.operation {
            // Cleanup old operations.
            if o.on_final_stage && o.join_handle.is_finished() {
                self.operation = None;
            } else if (ct - o.started_at) > 1000 * 120 {
                o.join_handle.abort();
                self.operation = None;
            }
        }
        if let Some(o) = &self.operation {
            if message.operation_initialization {
                message.response.send_rg_err(
                    "Operation initialization requested but operation already in progress".to_error()
                ).log_error().ok();
                return Ok(())
            } else if message.wallet_id != o.id{
                    message.response.send_rg_err(
                        "Operation requested with different wallet id from the one in progress, busy".to_error()
                    ).log_error().ok();
                    return Ok(())
            } else if !o.join_handle.is_finished() {
                message.response.send_rg_err(
                    "Operation requested while join handle in progress, busy".to_error()
                ).log_error().ok();
                return Ok(())
            }
        }
        match message.message {
            MoneroWalletMessage::CreateMultsigAsProposer => {
                if self.operation.is_some() {
                    message.response.send_rg_err(
                        "Operation requested while join handle in progress, busy".to_error()
                    ).log_error().ok();
                    return Ok(())
                }
                self.wallet_interface.lock().await.reset();
                let all_pks = message.all_pks.clone();
                let threshold = message.threshold;
                let peer_broadcast = self.peer_broadcast.clone();
                let sender = message.response.clone();
                let iface = self.wallet_interface.clone();
                let jh = tokio::spawn(async move {
                    let result = iface.lock().await.multisig_create_loop(
                        &all_pks,
                        threshold,
                        &peer_broadcast
                    ).await.map(|x| {
                        MoneroWalletResponse::InstanceCreate(x)
                    });
                    sender.send_rg_err(result).log_error().ok();
                });
                self.operation = Some(ActiveOperation{
                    id: message.wallet_id.clone(),
                    started_at: ct,
                    join_handle: Arc::new(jh),
                    on_final_stage: true,
                });

            }
            MoneroWalletMessage::MultisigCreateNext => {
                if message.operation_initialization {
                    if self.operation.is_some() {
                        message.response.send_rg_err(
                            "Operation initialization requested but operation already in progress".to_error()
                        ).log_error().ok();
                        return Ok(())
                    }
                    self.wallet_interface.lock().await.reset();
                }
                let peer_strings = Some(message.peer_strings);
                let thresh = Some(message.threshold);
                let wallet_id = message.wallet_id.clone();
                let sender = message.response.clone();
                let final_state = self.wallet_interface.lock().await.state.is_before_final_state();
                let iface = self.wallet_interface.clone();
                let jh = tokio::spawn(async move {
                    let result = iface.lock().await
                        .multisig_create_next(peer_strings, thresh, &wallet_id)
                        .await;
                    let response = result.and_then(|r| r.multisig_info_string().ok_msg("No multisig info string"))
                        .map(|x| MoneroWalletResponse::PeerCreate(x));
                    sender.send_rg_err(response).log_error().ok();
                });

                if message.operation_initialization {
                    self.operation = Some(ActiveOperation {
                        id: message.wallet_id.clone(),
                        started_at: ct,
                        join_handle: Arc::new(jh),
                        on_final_stage: final_state,
                    })
                } else if let Some(o) = &mut self.operation {
                    o.join_handle = Arc::new(jh);
                    o.on_final_stage = final_state;
                } else {
                    message.response.send_rg_err(
                        "No operation initialization requested but no operation in progress".to_error()
                    ).log_error().ok();
                }
            }
        }
        Ok(())
    }
}
