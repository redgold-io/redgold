use redgold_keys::TestConstants;
use redgold_schema::structs::{Address, ErrorInfo, SupportedCurrency, InitiateMultipartyKeygenRequest, InitiateMultipartySigningRequest, Proof, PublicKey};
use redgold_schema::{ProtoHashable, ProtoSerde, SafeBytesAccess};
use crate::DataStoreContext;
use crate::schema::SafeOption;
use redgold_schema::util;

#[derive(Clone)]
pub struct MultipartyStore {
    pub ctx: DataStoreContext
}

impl MultipartyStore {
    pub async fn local_share_and_initiate(&self, room_id: String) -> Result<Option<(String, InitiateMultipartyKeygenRequest)>, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;

        let rows = sqlx::query!(
            r#"SELECT local_share, initiate_keygen FROM multiparty WHERE room_id = ?1"#,
            room_id
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        // TODO: Check for collissions
        let mut res = None;

        for row in rows_m {
            let s: String = row.local_share;
            let k = row.initiate_keygen;
            let r = InitiateMultipartyKeygenRequest::proto_deserialize(k)?;
            res = Some((s, r));
        }
        Ok(res)
    }
    /*
    room_id TEXT PRIMARY KEY,
                                    local_share TEXT,
                                    proof BLOB,
                                    keygen_time INTEGER,
                                    proof_time INTEGER
     */
    pub async fn add_keygen(&self, local_share: String, room_id: String,
                            initiate_keygen: InitiateMultipartyKeygenRequest,
                            self_initiated: bool,
        time: Option<i64>
    ) -> Result<i64, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;

        let time = time.unwrap_or(util::current_time_millis());
        let init = initiate_keygen.proto_serialize();
        let id = initiate_keygen.identifier.safe_get_msg("ident missing in keygen sql add")?;
        let option = id.party_keys.get(0);
        let pk =
            option.safe_get_msg("Missing public key head in identifier multiparty keygen")?;
        let pkb = pk.bytes.safe_bytes()?;
        let rows = sqlx::query!(
            r#"INSERT OR REPLACE INTO multiparty (room_id, local_share, keygen_time, initiate_keygen, self_initiated, host_public_key) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
            room_id,
            local_share,
            time,
            init,
            self_initiated,
            pkb
        )
            .execute(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }
    pub async fn update_room_id_key(&self, room_id: String, key: PublicKey,
    ) -> Result<(), ErrorInfo> {
        let mut pool = self.ctx.pool().await?;

        let _time = util::current_time_millis();
        let pkb = key.bytes.safe_bytes()?;
        let rows = sqlx::query!(
            r#"UPDATE multiparty SET keygen_public_key = ?1 WHERE room_id = ?2"#,
            pkb,
            room_id,
        )
            .execute(&mut *pool)
            .await;
        let _rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(())
    }

    pub async fn add_signing_proof(
        &self, keygen_room_id: String, room_id: String, proof: Proof, initiate_signing: InitiateMultipartySigningRequest
    ) -> Result<(), ErrorInfo> {

        self.update_room_id_key(keygen_room_id.clone(), proof.public_key.safe_get_msg("Missing pk")?.clone()).await?;

        self.add_signing_proof_signatures(keygen_room_id, room_id, proof, initiate_signing).await
    }

    pub async fn add_signing_proof_signatures(&self, keygen_room_id: String, room_id: String, proof: Proof, initiate_signing: InitiateMultipartySigningRequest) -> Result<(), ErrorInfo> {
        let mut pool = self.ctx.pool().await?;

        let time = util::current_time_millis();
        let proof_vec = proof.proto_serialize();
        let init = initiate_signing.proto_serialize();
        let rows = sqlx::query!(
            r#"INSERT OR REPLACE INTO multiparty_signatures (room_id, keygen_room_id, proof, proof_time, initiate_signing) VALUES (?1, ?2, ?3, ?4, ?5)"#,
            room_id,
            keygen_room_id,
            proof_vec,
            time,
            init
        )
            .fetch_all(&mut *pool)
            .await;
        let _ = DataStoreContext::map_err_sqlx(rows)?;
        Ok(())
    }

    pub async fn check_bridge_txid_used(&self, txid: &Vec<u8>) -> Result<bool, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT COUNT(txid) as count FROM multiparty_bridge WHERE txid = ?1"#,
            txid
        )
            .fetch_one(&mut *pool)
            .await;
        let r = DataStoreContext::map_err_sqlx(rows)?;
        Ok(r.count > 0)
    }

    pub async fn insert_bridge_tx(
        &self,
        txid: &Vec<u8>,
        secondary_txid: &Vec<u8>,
        outgoing: bool,
        network: SupportedCurrency,
        source_address: &Address,
        destination_address: &Address,
        timestamp: i64,
        amount: i64
    ) -> Result<i64, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let network = network as i32;
        let source_address = source_address.address.safe_bytes()?;
        let destination_address = destination_address.address.safe_bytes()?;
        let rows = sqlx::query!(
            r#"INSERT INTO multiparty_bridge (txid, secondary_txid, outgoing,
            network, source_address, destination_address, timestamp, amount)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
            txid,
            secondary_txid,
            outgoing,
            network,
            source_address,
            destination_address,
            timestamp,
            amount
        )
            .execute(&mut *pool)
            .await;
        let r = DataStoreContext::map_err_sqlx(rows)?;
        Ok(r.last_insert_rowid())
    }

    //
    // pub async fn query_transaction_hex(
    //     &self,
    //     hex: String,
    // ) -> Result<Option<Transaction>, ErrorInfo> {
    //     let vec = from_hex(hex)?;
    //     self.query_transaction(&vec).await
    // }
    //
    // pub async fn query_transaction(
    //     &self,
    //     transaction_hash: &Vec<u8>,
    // ) -> Result<Option<Transaction>, ErrorInfo> {
    //
    //     let mut pool = self.ctx.pool().await?;
    //     let rows = sqlx::query!(
    //         r#"SELECT raw_transaction FROM transactions WHERE hash = ?1"#,
    //         transaction_hash
    //     )
    //         .fetch_all(&mut *pool)
    //         .await;
    //     let rows_m = DataStoreContext::map_err_sqlx(rows)?;
    //     let mut res = vec![];
    //     for row in rows_m {
    //         let option1 = row.raw_transaction;
    //         if let Some(o) = option1 {
    //             let deser = Transaction::proto_deserialize(o)?;
    //             res.push(deser);
    //         }
    //     }
    //     let option = res.get(0).map(|x| x.clone());
    //     Ok(option)
    // }

}

#[test]
fn debug() {

}