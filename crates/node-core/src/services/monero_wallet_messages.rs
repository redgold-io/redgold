use async_trait::async_trait;
use redgold_common::flume_send_help::SendErrorInfo;
use redgold_common_no_wasm::ssh_like::{LocalSSHLike, SSHOrCommandLike};
use redgold_common_no_wasm::stream_handlers::{IntervalFoldOrReceive, TryRecvForEach};
use redgold_keys::monero::node_wrapper::{MoneroNodeRpcInterfaceWrapper, MoneroWalletMultisigRpcState, PartySecretInstanceData};
use redgold_schema::observability::errors::Loggable;
use redgold_schema::RgResult;
use redgold_schema::structs::PublicKey;

#[derive(Clone)]
pub struct ActiveOperation {
    pub id: String,
    pub since: i64
}

#[derive(Clone)]
pub struct MoneroWalletSyncWriter {
    pub wallet_interface: MoneroNodeRpcInterfaceWrapper<LocalSSHLike>,
    pub operation: Option<ActiveOperation>
}

#[derive(Clone, Debug)]
pub struct MultisigCreateNext {
    peer_strings: Vec<String>,
    threshold: Option<i64>,
    response: flume::Sender<RgResult<MoneroWalletMultisigRpcState>>
}

#[derive(Clone, Debug)]
pub struct CreateMultsigAsProposer {
    peers: Vec<PublicKey>,
    threshold: Option<i64>,
    response: flume::Sender<RgResult<PartySecretInstanceData>>
}

#[derive(Clone, Debug)]
pub enum MoneroWalletMessage {
    MultisigCreateNext(MultisigCreateNext),
    CreateMultsigAsProposer(CreateMultsigAsProposer)
}

#[async_trait]
pub trait MoneroWalletSender {
    async fn send(&self, message: MoneroWalletMessage) -> RgResult<()>;
}

#[async_trait]
impl TryRecvForEach<MoneroWalletMessage> for MoneroWalletSyncWriter {
    async fn try_recv_for_each(&mut self, message: MoneroWalletMessage) -> RgResult<()> {
        // if let Some(operation) = &self.operation {
        //     if message
        // }
        match message {
            MoneroWalletMessage::MultisigCreateNext(m) => {
                let result = self.wallet_interface
                .multisig_create_next(
                    m.peer_strings, m.threshold, &"".to_string()
                ).await;
                m.response.send_rg_err(result).log_error().ok();

            }
            MoneroWalletMessage::CreateMultsigAsProposer(p) => {
                let mut r = r?;
                let result = r.multisig_create_loop(&all_pks, threshold, peer_broadcast).await?;
                secret_json = Some(result.json_or());
                Some(result.address)
            }
        }
        Ok(())
    }
}
