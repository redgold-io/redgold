use flume::Sender;
use redgold_schema::structs::{PublicKey, Transaction};
use redgold_schema::util::lang_util::JsonCombineResult;
use crate::core::internal_message::SendErrorInfo;
use crate::gui::app_loop::LocalState;
use crate::gui::tabs::transact::wallet_tab::StateUpdate;
use crate::hardware::trezor;
use redgold_schema::observability::errors::Loggable;

pub fn initiate_hardware_signing(t: Transaction, send: Sender<StateUpdate>, public: PublicKey) {
    tokio::spawn(async move {
        let t = &mut t.clone();
        let res = trezor::sign_transaction(
            t, public, trezor::default_pubkey_path())
            .await
            .log_error()
            .map(|x| x.clone())
            .map_err(|e| e.clone());

        let st = Some(res.clone());
        let st_msg = Some(res.clone().json_or_combine());
        let ss = Some(res
            .map(|_x| "Signed Successfully".to_string())
            .unwrap_or("Signing error".to_string()));

        let fun = move |ls: &mut LocalState| {
            ls.wallet_state.update_signed_tx(st.clone());
            ls.wallet_state.signing_flow_transaction_box_msg = st_msg.clone();
            ls.wallet_state.signing_flow_status = ss.clone();
        };
        let up = StateUpdate {
            update: Box::new(fun),
        };
        send.send_rg_err(up).log_error().ok();
    });
}
