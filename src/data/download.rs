use crate::core::internal_message::new_channel;
use crate::core::internal_message::PeerMessage;
use crate::core::relay::Relay;
use crate::data::data_store::DataStore;
use crate::genesis::create_genesis_transaction;
use crate::schema::structs::{
    DownloadDataType, DownloadRequest, DownloadResponse, NodeState, Request, Response,
};
use crate::util;
use bitcoin::secp256k1::PublicKey;
use log::{error, info};
use redgold_schema::constants::EARLIEST_TIME;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct DownloadMaxTimes {
    pub utxo: i64,
    pub transaction: i64,
    pub observation: i64,
    pub observation_edge: i64,
}

pub fn download_msg(
    relay: &Relay,
    start_time: i64,
    end_time: i64,
    data_type: DownloadDataType,
    key: PublicKey,
) -> Result<Response, String> {

    let key_hex = hex::encode(key.serialize().to_vec());
    // info!("Sending download message: start {:?} end: {:?}  type {:?} key_hex: {}", start_time, end_time, data_type, key_hex);
    let c = new_channel::<Response>();
    let r = c.sender.clone();
    let mut request = Request::empty();
    request.download_request = Some(DownloadRequest {
            start_time: start_time as u64,
            end_time: end_time as u64,
            data_type: data_type as i32,
            offset: None,
    });
    let mut message = PeerMessage::empty();
    message.response = Some(r);
    message.public_key = Some(key);
    message.request = request;
    let _err = relay
        .peer_message_tx
        .sender
        .send(message)
        .map_err(|e| e.to_string())?;
    // info!("Sent peer message for download, awaiting response with timeout 120");
    let response = c.receiver
        .recv_timeout(Duration::from_secs(120))
        .map_err(|e| e.to_string())?;
    use redgold_schema::json_or;
    // info!("Download response: {}", json_or(&response.clone()));
    Ok(response)
}

pub async fn download_all(
    relay: &Relay,
    start_time: i64,
    end_time: i64,
    key: PublicKey,
    clean_up_utxo: bool,
) {
    let rr = download_msg(
        &relay,
        start_time,
        end_time,
        DownloadDataType::UtxoEntry,
        key,
    );

    let vec = rr.unwrap().download_response.unwrap().utxo_entries;
    info!("Downloaded: {} utxo entries", vec.len());
    for x in vec.clone() {
        let err = relay.ds.transaction_store.insert_utxo(&x).await;
        if err.is_err() {
            error!("{:?}", err);
        }
    }

    let r = download_msg(
        &relay,
        start_time,
        end_time,
        DownloadDataType::TransactionEntry,
        key,
    );

    for x in r.unwrap().download_response.unwrap().transactions {
        relay
            .ds
            .transaction_store
            .insert_transaction_raw(&x.transaction.as_ref().unwrap(), x.time as i64, true, None)
            .await;
        // TODO return this error
        // .expect("fix");
        for (i, j) in x.transaction.as_ref().unwrap().iter_utxo_inputs() {
            // todo probably distinguish between empty or not ?
            if clean_up_utxo {
                relay.ds.delete_utxo(&i, j as u32).expect("fix");
            }
        }
    }

    let result = download_msg(
        &relay,
        start_time,
        end_time,
        DownloadDataType::ObservationEntry,
        key,
    );
    let option = result.unwrap().download_response;

    for x in option.unwrap().observations {
        relay.ds.insert_observation(x.observation.unwrap(), x.time);
        // .expect("fix");
    }

    let result1 = download_msg(
        &relay,
        start_time,
        end_time,
        DownloadDataType::ObservationEdgeEntry,
        key,
    );
    let option1 = result1.unwrap().download_response;
    for x in option1.unwrap().observation_edges {
        relay.ds.insert_observation_edge(&x);
        // .expect("fix");
    }
}

/**
Actual download process should start with getting the current parquet part file
snapshot through IPFS for all the different data types. The compacted format.
*/
pub async fn download(relay: Relay, key: PublicKey) {
    // remove genesis entry if it exists.
    // for (x, y) in create_genesis_transaction().iter_utxo_outputs() {
    //     // let err = DataStore::map_err(relay.ds.clone().delete_utxo(&x, y as u32));
    //     // if err.is_err() {
    //     //     error!("{:?}", err);
    //     // }
    // }

    // Query last time for each database, use that on the download functionality as offset
    //let last_time = relay.ds.query_download_times();

    // relay.node_state.store(NodeState::Downloading);

    let start_dl_time = util::current_time_millis();

    download_all(&relay, EARLIEST_TIME, start_dl_time as i64, key, false).await;

    // relay.node_state.store(NodeState::Synchronizing);
    //
    // download_all(
    //     &relay,
    //     start_dl_time,
    //     util::current_time_millis(),
    //     key,
    //     true,
    // );

    // Verify that we've cleaned up an old output.

    // relay.node_state.store(NodeState::Ready);

    info!(
        "Number of transactions after download {}",
        relay
            .ds
            .query_time_transaction(0, util::current_time_millis())
            .unwrap()
            .len()
    );
}

//
// pub struct DownloadHandler {
//     relay: Relay
// }
//
// impl DownloadHandler {
//     async fn run(&mut self) {
//         // let mut interval = tokio::time::interval(self.node_config.observation_formation_millis);
//         // TODO use a select! between these two.
//         loop {
//             match self.relay.observation_metadata.receiver.try_recv() {
//                 Ok(o) => {
//                     info!(
//                         "Pushing observation metadata to buffer {}",
//                         serde_json::to_string(&o.clone()).unwrap()
//                     );
//                     self.data.push(o);
//                 }
//                 Err(_) => {}
//             }
//             if SystemTime::now().duration_since(self.last_flush).unwrap()
//                 > Duration::from_millis(self.relay.node_config.observation_formation_millis)
//             {
//                 self.form_observation();
//             }
//         }
//     }
//
//     // https://stackoverflow.com/questions/63347498/tokiospawn-borrowed-value-does-not-live-long-enough-argument-requires-tha
//     pub fn new(relay: Relay, arc: Arc<Runtime>) {
//         let mut o = Self {
//             data: vec![],
//             relay,
//             last_flush: SystemTime::now(),
//         };
//         arc.spawn(async move { o.run().await });
//     }
//

pub fn process_download_request(
    relay: &Relay,
    download_request: DownloadRequest,
) -> Result<DownloadResponse, rusqlite::Error> {
    Ok(DownloadResponse {
        utxo_entries: {
            if download_request.data_type != DownloadDataType::UtxoEntry as i32 {
                vec![]
            } else {
                relay
                    .ds
                    .query_time_utxo(download_request.start_time, download_request.end_time)?
            }
        },
        transactions: {
            if download_request.data_type != DownloadDataType::TransactionEntry as i32 {
                vec![]
            } else {
                relay.ds.query_time_transaction(
                    download_request.start_time,
                    download_request.end_time,
                )?
            }
        },
        observations: {
            if download_request.data_type != DownloadDataType::ObservationEntry as i32 {
                vec![]
            } else {
                relay.ds.query_time_observation(
                    download_request.start_time,
                    download_request.end_time,
                )?
            }
        },
        observation_edges: {
            if download_request.data_type != DownloadDataType::ObservationEdgeEntry as i32 {
                vec![]
            } else {
                relay.ds.query_time_observation_edge(
                    download_request.start_time,
                    download_request.end_time,
                )?
            }
        },
        // TODO: not this
        complete_response: true,
    })
}
