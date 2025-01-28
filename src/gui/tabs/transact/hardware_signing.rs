use crate::hardware::trezor;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::{EnhanceErrorInfo, Loggable};
use redgold_schema::structs::{ErrorInfo, PublicKey, Transaction};


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
