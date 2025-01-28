use redgold_schema::structs::{CurrencyAmount, InitiateMultipartyKeygenRequest, PartyId, PublicKey};
use redgold_schema::{RgResult, SafeOption};
use rocket::serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct DepositKeyAllocation {
    pub key: PublicKey,
    pub allocation: f64,
    pub initiate: InitiateMultipartyKeygenRequest,
    pub balance_btc: u64,
    pub balance_rdg: u64,
}

impl DepositKeyAllocation {
    pub fn is_self_initiated(&self, self_key: &PublicKey) -> RgResult<bool> {
        let id = self.initiate.identifier.safe_get_msg("Missing identifier")?;
        let head = id.party_keys.get(0);
        let head_pk = head.safe_get_msg("Missing party keys")?;
        Ok(head_pk == &self_key)
    }

    pub fn party_id(&self) -> RgResult<PartyId> {
        let id = self.initiate.identifier.safe_get_msg("Missing identifier")?;
        let head = id.party_keys.get(0);
        let mut pid = PartyId::default();
        pid.public_key = Some(self.key.clone());
        pid.owner = head.cloned();
        Ok(pid)
    }

    pub fn balances(&self) -> Vec<CurrencyAmount> {
        vec![
            CurrencyAmount::from_btc(self.balance_btc as i64),
            CurrencyAmount::from_rdg(self.balance_btc as i64),
            ]
    }


    // pub fn party_info(&self) -> RgResult<PartyInfo> {
    //     let ident = self.initiate.identifier.safe_get_msg("Missing identifier")?;
    //     let id = self.party_id()?;
    //     let size = ident.party_keys.len();
    //     let mut pi = PartyInfo::default();
    //     let w = Weighting::from_float((ident.threshold as f64) / (size as f64));
    //     let mw = Weighting::from_float(1f64 / (size as f64));
    //     pi.party_id = Some(id);
    //     pi.threshold = Some(w);
    //     pi.balances = self.balances();
    //     pi.members = ident.party_keys.iter().map(|p| {
    //         let mut pm = structs::PartyMember::default();
    //         pm.public_key = Some(p.clone());
    //         pm.weight = Some(mw.clone());
    //         pm
    //     }).collect_vec();
    //     Ok(pi)
    // }


}
