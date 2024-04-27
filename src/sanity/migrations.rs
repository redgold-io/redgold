use log::{error, info};
use redgold_schema::RgResult;
use redgold_schema::structs::{Hash, UtxoId};
use rocket::serde::{Deserialize, Serialize};
use itertools::Itertools;
use rocket::form::validate::Contains;
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use crate::core::relay::Relay;
use crate::util;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManualMigration {
    id: i64,
    name: String,
    time: i64
}

impl ManualMigration {
    pub fn new(id: i64, name: String) -> ManualMigration {
        ManualMigration {
            id,
            name,
            time: util::current_time_millis_i64()
        }
    }
}

pub async fn apply_migrations(relay: &Relay) -> RgResult<()> {
    let mut migrations =
        relay.ds.config_store.get_json::<Vec<ManualMigration>>("migrations").await?
            .unwrap_or(vec![]);

    apply_dev_amm_utxo_migration_0(relay, &migrations).await?
        .iter()
        .for_each(|m| {
            migrations.push(m.clone())
        });

    relay.ds.config_store.insert_update_json("migrations", migrations).await?;

    Ok(())
}

async fn safe_remove_transaction_utxos(relay: &Relay, hash: &Hash) -> RgResult<()> {
    let ids = find_all_transaction_and_children_utxo_ids(relay, hash).await?;
    for utxo_id in ids {
        let num_rows = relay.ds.utxo.delete_utxo(&utxo_id, None).await?;
        info!("Migration removed {} rows for utxo_id: {}", num_rows, utxo_id.json_or());
        let valid = relay.ds.utxo.utxo_id_valid(&utxo_id).await?;
        if !valid {
            error!("UTXO still 'valid' safe_remove_transaction_utxos - Migration failed to remove utxo_id: {}", utxo_id.json_or());
        }
        let utxo_resolved = relay.ds.utxo.utxo_for_id(&utxo_id).await?;
        if !utxo_resolved.is_empty() {
            for u in utxo_resolved {
                if let Ok(a) = u.address() {
                    let entries_for_address = relay.ds.utxo.utxo_for_address(a).await?
                        .iter().flat_map(|u| u.utxo_id.as_ref()).cloned().collect_vec();
                    let address_entry_deletion_failed = entries_for_address.contains(&utxo_id);
                    if address_entry_deletion_failed {
                        error!("Migration failed to remove utxo_id: {} from address: {}", utxo_id.json_or(), a.json_or());
                    }
                }
            }
        }
    };
    Ok(())
}

async fn find_all_transaction_and_children_utxo_ids(r: &Relay, hash: &Hash) -> RgResult<Vec<UtxoId>> {
    let mut remaining_hashes_to_explore = vec![];
    remaining_hashes_to_explore.push(hash.clone());
    let mut all_utxo_ids = vec![];
    while remaining_hashes_to_explore.len() > 0 {
        let entry = remaining_hashes_to_explore.pop();
        if let Some(c) = entry {
            if let Some((tx, None)) = r.ds.transaction_store.query_maybe_transaction(&c).await? {
                for u in tx.output_utxo_ids() {
                    all_utxo_ids.push(u.clone());
                    let option = r.ds.utxo.utxo_child(&u).await.unwrap();
                    if let Some(child) = &option {
                        if !remaining_hashes_to_explore.contains(&child.0) {
                            remaining_hashes_to_explore.push(child.0.clone());
                        }
                    }
                }
            }
        }
    }
    Ok(all_utxo_ids)
}


async fn apply_dev_amm_utxo_migration_0(relay: &Relay, finished: &Vec<ManualMigration>) -> RgResult<Option<ManualMigration>> {

    let migration_id = 2;

    if finished.iter().filter(|m| m.id == migration_id).count() > 0 || !relay.node_config.network.is_dev()  {
        return Ok(None);
    }
    let raw = include_str!("../resources/migrations/0/remove_tx_hashes.json");
    let hash = raw.to_string().json_from::<Vec<Hash>>()?;
    for h in hash {
        safe_remove_transaction_utxos(relay, &h).await?;
    }

    let raw_utxos = include_str!("../resources/migrations/0/remove_utxos.json");
    remove_utxos(relay, raw_utxos, "first_migration".to_string()).await?;

    let raw_utxos = include_str!("../resources/migrations/0/is_valid_but_has_kids.json");
    remove_utxos(relay, raw_utxos, "second_migration".to_string()).await?;

    Ok(Some(ManualMigration::new(migration_id, "dev_amm_remove_utxo_hashes2".to_string())))
}

async fn remove_utxos(relay: &Relay, raw_utxos: &str, name: String) -> RgResult<()> {
    let utxos = raw_utxos.to_string().json_from::<Vec<UtxoId>>()?;

    for utxo_id in utxos {
        let num_rows = relay.ds.utxo.delete_utxo(&utxo_id, None).await?;
        info!("{name} remove_utxos.json direct Migration removed {} rows for utxo_id: {}", num_rows, utxo_id.json_or());
        let valid = relay.ds.utxo.utxo_id_valid(&utxo_id).await?;
        if !valid {
            error!("{name} remove_utxos.json direct Migration failed to remove utxo_id: {}", utxo_id.json_or());
        }
    }
    Ok(())
}
