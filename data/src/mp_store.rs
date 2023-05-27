use redgold_schema::structs::{Address, ErrorInfo, Hash, InitiateMultipartyKeygenRequest, InitiateMultipartySigningRequest, PeerData, Proof, Transaction};
use redgold_schema::{from_hex, ProtoHashable, ProtoSerde, SafeBytesAccess, TestConstants, WithMetadataHashable};
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
            .fetch_all(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        // TODO: Check for collissions
        let mut res = None;

        for row in rows_m {
            if let Some(s) = row.local_share {
                if let Some(k) = row.initiate_keygen {
                    let r = InitiateMultipartyKeygenRequest::proto_deserialize(k)?;
                    res = Some((s, r))
                }
            }
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
                            initiate_keygen: InitiateMultipartyKeygenRequest
    ) -> Result<i64, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;

        let time = util::current_time_millis();
        let init = initiate_keygen.proto_serialize();
        let rows = sqlx::query!(
            r#"INSERT OR REPLACE INTO multiparty (room_id, local_share, keygen_time, initiate_keygen) VALUES (?1, ?2, ?3, ?4)"#,
            room_id,
            local_share,
            time,
            init
        )
            .execute(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }

    pub async fn add_signing_proof(&self, keygen_room_id: String, room_id: String, proof: Proof, initiate_signing: InitiateMultipartySigningRequest) -> Result<(), ErrorInfo> {
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
            .fetch_all(&mut pool)
            .await;
        let _ = DataStoreContext::map_err_sqlx(rows)?;
        Ok(())
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
    //         .fetch_all(&mut pool)
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