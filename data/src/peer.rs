use redgold_schema::structs::{Address, ErrorInfo, Hash, PeerData, Transaction};
use redgold_schema::{ProtoHashable, SafeBytesAccess, TestConstants, WithMetadataHashable};
use crate::DataStoreContext;
use crate::schema::SafeOption;

#[derive(Clone)]
pub struct PeerStore {
    pub ctx: DataStoreContext
}

impl PeerStore {

}


impl PeerStore {

    pub async fn add_peer(&self, tx: &Transaction, trust: f64) -> Result<(), ErrorInfo> {
        // return Err(ErrorInfo::error_info("debug error return"));
        let pd = tx.peer_data()?;
        let tx_blob = tx.proto_serialize();
        let pd_blob = pd.proto_serialize();
        let tx_hash = tx.hash().vec();
        let mut pool = self.ctx.pool().await?;
        let pid = pd.peer_id.safe_get()?.clone();

        let rows = sqlx::query!(
            r#"INSERT OR REPLACE INTO peers (id, peer_data, tx, trust, tx_hash) VALUES (?1, ?2, ?3, ?4, ?5)"#,
            pid,
            pd_blob,
            tx_blob,
            trust,
            tx_hash
        )
            .fetch_all(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;

        for nmd in pd.node_metadata {

            let rows = sqlx::query!(
            r#"INSERT OR REPLACE INTO peer_key (public_key, id, multi_hash, address, status) VALUES (?1, ?2, ?3, ?4, ?5)"#,
            nmd.public_key,
            pid,
            nmd.multi_hash,
            nmd.external_address,
            "",
            )
                .fetch_all(&mut pool)
                .await;
            let rows_m = DataStoreContext::map_err_sqlx(rows)?;

        }
        Ok(())
    }
    pub async fn multihash_lookup(&self, p0: Vec<u8>) {
        todo!()
    }

    pub async fn all_peers_tx(
        &self
    ) -> Result<Vec<Transaction>, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"SELECT tx FROM peers"#
        )
            .fetch_all(&mut pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mut res = vec![];
        for row in rows_m {
            let deser = Transaction::proto_deserialize(row.tx)?;
            res.push(deser);
        }
        Ok(res)
    }

    pub async fn all_peers(
        &self
    ) -> Result<Vec<PeerData>, ErrorInfo> {
        self.all_peers_tx().await?.iter().map(|tx| tx.peer_data()).collect()
    }

    pub async fn peers_by_utxo_distance(
        &self,
        hash: Hash,
    ) -> Result<usize, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;
        // let rows = sqlx::query!(
        //     r#"SELECT UNIQUE(peers.peer_data) FROM peer_key INNER JOIN on peer_key.id = peers.id WHERE peer_key.id"#,
        //     height
        // )
        //     .fetch_all(&mut pool)
        //     .await;
        // let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        // let mut res = vec![];
        // for row in rows_m {
        //     res.push(AddressHistoricalBalance{
        //         address: Address::from_bytes(row.address.safe_get_msg("Row missing address")?.clone())?,
        //         balance: row.balance.safe_get_msg("Row missing balance")?.clone(),
        //         height: height.clone()
        //     });
        // }
        // Ok(res)
        Ok(0)
    }

}

#[test]
fn distance_check(){
    let tc = TestConstants::new();
    let a = tc.rhash_1.safe_bytes().unwrap();
    let b = tc.rhash_2.safe_bytes().unwrap();

    let c: Vec<u8> = a.iter().zip(b).map(|(x, y)| x ^ y).collect();


}