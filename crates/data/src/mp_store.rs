use redgold_schema::parties::PartyInfo;
use crate::schema::SafeOption;
use crate::DataStoreContext;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{ErrorInfo, InitiateMultipartySigningRequest, PartyData, Proof, PublicKey, RoomId};
use redgold_schema::util::times;
use redgold_schema::RgResult;

#[derive(Clone)]
pub struct MultipartyStore {
    pub ctx: DataStoreContext
}

impl MultipartyStore {

    pub async fn all_party_info_with_key(&self) -> RgResult<Vec<PartyInfo>> {
        DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT party_info FROM multiparty WHERE keygen_public_key IS NOT NULL"#,
        )
            .fetch_all(&mut *self.ctx.pool().await?)
            .await)?.into_iter().map(|r| PartyInfo::proto_deserialize(r.party_info)).collect()
    }

    pub async fn party_info(&self, room_id: &RoomId) -> RgResult<Option<PartyInfo>> {
        let mut pool = self.ctx.pool().await?;
        let room_id = room_id.vec();

        let rows = sqlx::query!(
            r#"SELECT party_info FROM multiparty WHERE room_id = ?1"#,
            room_id
        )
            .fetch_optional(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        rows_m.map(|r| PartyInfo::proto_deserialize(r.party_info)).transpose()
    }

    pub async fn party_data(&self, keygen_public_key: &PublicKey) -> RgResult<Option<PartyData>> {
        let mut pool = self.ctx.pool().await?;
        let qry = keygen_public_key.vec();

        let rows = sqlx::query!(
            r#"SELECT party_data FROM multiparty WHERE keygen_public_key = ?1"#,
            qry
        )
            .fetch_optional(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        let mapped = rows_m.and_then(|r| r.party_data);
        let deser = mapped.map(|r| PartyData::proto_deserialize(r)).transpose()?;
        Ok(deser)
    }
    pub async fn add_keygen(&self, party_info: &PartyInfo) -> Result<i64, ErrorInfo> {
        let mut pool = self.ctx.pool().await?;


        let initiate_keygen = party_info.initiate.safe_get_msg("init missing in keygen sql add")?;
        let ident = initiate_keygen.identifier.safe_get_msg("ident missing in keygen sql add")?;

        let room_id = ident.room_id.safe_get_msg("missing room id")?.vec();
        let time = initiate_keygen.time;
        let pi = party_info.proto_serialize();
        let self_initiated = party_info.self_initiated.safe_get()?.clone();
        let host_public_key = ident.party_keys.get(0).safe_get_msg("missing host pk")?.vec();

        let party_data: Vec<u8> = vec![];

        let rows = sqlx::query!(
            r#"INSERT OR REPLACE INTO multiparty
            (room_id, keygen_time, party_info, self_initiated, host_public_key, party_data)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
            room_id,
            time,
            pi,
            self_initiated,
            host_public_key,
            party_data
        )
            .execute(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid())
    }


    pub async fn count_multiparty_total(
        &self
    ) -> Result<i64, ErrorInfo> {

        Ok(DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT COUNT(*) as count FROM multiparty"#
        )
            .fetch_one(&mut *self.ctx.pool().await?)
            .await)?.count as i64)
    }

    pub async fn count_multiparty_pk(
        &self, initiator: &PublicKey
    ) -> Result<i64, ErrorInfo> {

        let pk = initiator.vec();
        Ok(DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT COUNT(*) as count FROM multiparty WHERE host_public_key = ?1"#,
            pk
        )
            .fetch_one(&mut *self.ctx.pool().await?)
            .await)?.count as i64)
    }

    pub async fn count_self_multiparty(
        &self
    ) -> Result<i64, ErrorInfo> {
        Ok(DataStoreContext::map_err_sqlx(sqlx::query!(
            r#"SELECT COUNT(*) as count FROM multiparty WHERE self_initiated = 1"#,
        )
            .fetch_one(&mut *self.ctx.pool().await?)
            .await)?.count as i64)
    }

    pub async fn update_room_id_party_key(&self, keygen_room_id: &RoomId, party_pk: &PublicKey) -> RgResult<i64> {
        let mut pool = self.ctx.pool().await?;

        let pi = party_pk.vec();
        let room_id = keygen_room_id.vec();
        let rows = sqlx::query!(
            r#"UPDATE multiparty SET keygen_public_key = ?1 WHERE room_id = ?2"#,
            pi,
            room_id,
        )
            .execute(&mut *pool)
            .await;
        let _rows_m = DataStoreContext::map_err_sqlx(rows)?.rows_affected();
        Ok(_rows_m as i64)
    }

    pub async fn update_room_id_party_info(&self, room_id: &RoomId, party_info: PartyInfo) -> RgResult<i64> {
        let mut pool = self.ctx.pool().await?;

        let pi = party_info.proto_serialize();
        let room_id = room_id.vec();
        let rows = sqlx::query!(
            r#"UPDATE multiparty SET party_info = ?1 WHERE room_id = ?2"#,
            pi,
            room_id,
        )
            .execute(&mut *pool)
            .await;
        let _rows_m = DataStoreContext::map_err_sqlx(rows)?.rows_affected();
        Ok(_rows_m as i64)
    }

    pub async fn update_party_data(&self, keygen_public_key: &PublicKey, party_data: PartyData) -> RgResult<i64> {
        let mut pool = self.ctx.pool().await?;

        let pi = party_data.proto_serialize();
        let qry = keygen_public_key.vec();
        let rows = sqlx::query!(
            r#"UPDATE multiparty SET party_data = ?1 WHERE keygen_public_key = ?2"#,
            pi,
            qry,
        )
            .execute(&mut *pool)
            .await;
        let _rows_m = DataStoreContext::map_err_sqlx(rows)?.rows_affected();
        Ok(_rows_m as i64)
    }

    pub async fn add_signing_proof(
        &self, keygen_room_id: &RoomId, room_id: &RoomId, proof: Proof, initiate_signing: InitiateMultipartySigningRequest
    ) -> Result<(), ErrorInfo> {

        let mut pi = self.party_info(keygen_room_id).await?.ok_or(ErrorInfo::new("No party info for keygen room"))?;
        if pi.party_key.is_none() {
            // info!("Adding party key to party info and table signing proof with existing party info {}", pi.json_or());
            let key = proof.public_key.safe_get_msg("Missing pk")?.clone();
            pi.party_key = Some(key.clone());
            self.update_room_id_party_info(keygen_room_id, pi).await?;
            self.update_room_id_party_key(keygen_room_id, &key).await?;
            let all_pi = self.all_party_info_with_key().await?;
            // info!("All party info with key after update {}", all_pi.json_or());
            if !all_pi.iter().any(|p| p.party_key == Some(key.clone())) {
                return Err(ErrorInfo::new("Party key not found in all party info after update"));
            }
        }
        self.add_signing_proof_signatures(keygen_room_id, room_id, proof, initiate_signing).await
    }

    pub async fn add_signing_proof_signatures(
        &self,
        keygen_room_id: &RoomId,
        room_id: &RoomId,
        proof: Proof,
        initiate_signing: InitiateMultipartySigningRequest
    ) -> Result<(), ErrorInfo> {
        let mut pool = self.ctx.pool().await?;

        let keygen_room_id = keygen_room_id.vec();
        let room_id = room_id.vec();

        let time = times::current_time_millis();
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

}

#[test]
fn debug() {

}