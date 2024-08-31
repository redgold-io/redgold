use flume::Sender;
use redgold_schema::structs::Transaction;
use redgold_schema::util::lang_util::JsonCombineResult;
use crate::core::internal_message::SendErrorInfo;
use crate::gui::app_loop::LocalState;
use crate::gui::tabs::transact::wallet_tab::StateUpdate;
use crate::node_config::NodeConfig;
use redgold_schema::observability::errors::Loggable;

// TODO: Abstract over spawn/send updates
pub fn broadcast_transaction(nc: NodeConfig, tx: Transaction, send: Sender<StateUpdate>) {
    tokio::spawn(async move {
        let nc = nc.clone();
        let res = nc.clone().api_client().send_transaction(&tx.clone(), true).await;

        let st = Some(res.clone());
        let st_msg = Some(res.clone().json_or_combine());
        let ss = Some(res
            .map(|_x| "Transaction Accepted".to_string())
            .unwrap_or("Rejected Transaction".to_string()));

        let fun = move |ls: &mut LocalState| {
            ls.wallet_state.broadcast_transaction_response = st.clone();
            ls.wallet_state.signing_flow_transaction_box_msg = st_msg.clone();
            ls.wallet_state.signing_flow_status = ss.clone();
        };
        let up = StateUpdate {
            update: Box::new(fun),
        };
        send.send_rg_err(up).log_error().ok();
    });
}
