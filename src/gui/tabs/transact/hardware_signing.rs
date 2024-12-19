use flume::Sender;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::structs::{ErrorInfo, PublicKey, Transaction};
use redgold_schema::util::lang_util::JsonCombineResult;
use redgold_common::flume_send_help::SendErrorInfo;
use crate::gui::app_loop::LocalState;
use crate::gui::tabs::transact::wallet_tab::StateUpdate;
use crate::hardware::trezor;
use redgold_schema::observability::errors::{EnhanceErrorInfo, Loggable};

pub fn initiate_hardware_signing(t: Transaction, send: Sender<StateUpdate>, public: PublicKey, derivation_path: String) {
    tokio::spawn(async move {
        let t = &mut t.clone();
        let res = gui_trezor_sign(public, derivation_path, t).await;

        let st = Some(res.clone());
        let st_msg = Some(res.clone().json_or_combine());
        let ss = Some(res
            .map(|_x| "Signed Successfully".to_string())
            .unwrap_or("Signing error".to_string()));

        let fun = move |ls: &mut LocalState| {
            ls.wallet.update_signed_tx(st.clone());
            ls.wallet.signing_flow_transaction_box_msg = st_msg.clone();
            ls.wallet.signing_flow_status = ss.clone();
        };
        let up = StateUpdate {
            update: Box::new(fun),
        };
        send.send_rg_err(up).log_error().ok();
    });
}

pub async fn gui_trezor_sign(public: PublicKey, derivation_path: String, t: &mut Transaction) -> Result<Transaction, ErrorInfo> {
    trezor::sign_transaction(
        t, public.clone(), derivation_path.clone())
        .await
        .with_detail("derivation_path", derivation_path.clone())
        .with_detail("public_key", public.json_or())
        .with_detail("transaction", t.json_or())
        .log_error()
        .map(|x| x.clone())
        .map_err(|e| e.clone())
}
