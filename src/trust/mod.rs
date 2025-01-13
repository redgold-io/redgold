use crate::core::relay::Relay;
use crate::schema::structs::TrustRatingLabel;
use std::collections::HashMap;
use async_trait::async_trait;
use redgold_schema::RgResult;
use redgold_common_no_wasm::stream_handlers::IntervalFold;

pub mod eigentrust;
pub mod moon;
pub mod rewards;
pub mod embed;
pub mod features;

#[allow(dead_code)]
struct Trust {
    relay: Relay
}

// Honestly is this even needed? Just update the internal database and use a model
// every 5 minutes.
#[allow(dead_code)]
#[async_trait]
impl IntervalFold for Trust {
    async fn interval_fold(&mut self) -> RgResult<()> {

        Ok(())
    }
    // async fn run(&mut self) {
    //     loop {
    //         let update = self.relay.trust.receiver.recv().unwrap();
    //         match update.remove_peer {
    //             None => {}
    //             Some(remove) => {
    //                 self.labels.remove(&remove);
    //                 continue;
    //             }
    //         }
    //         let peer_data = update.update;
    //         if peer_data.labels.is_empty() {
    //             continue;
    //         }
    //
    //         let id = peer_data.peer_id.unwrap().peer_id.unwrap().value;
    //         let latest = self.labels.get(&id).unwrap();
    //
    //         let mut label_updates: HashMap<Vec<u8>, f64> = HashMap::new();
    //
    //         for label in peer_data.labels {
    //             let l = label.trust_data.get(0).expect("data").label;
    //             label_updates.insert(label.peer_id, l);
    //         }
    //
    //         if label_updates != *latest {
    //             // self.labels.insert(self.pe, label_updates);
    //             // self.recalculate_trust()
    //         }
    //     }
    // }
    // #[allow(dead_code)]
    // pub fn new(relay: Relay) {
    //     let mut b = Self {
    //         relay,
    //         labels: HashMap::new(),
    //     };
    //     tokio::spawn(async move { b.run().await });
    // }
}

// https://texample.net/tikz/examples/neural-network/
// https://docs.rs/ndarray/0.15.3/ndarray/doc/ndarray_for_numpy_users/index.html
// fn convert_trust_map(
//     peer_metadata: &Vec<PeerMetadata>,
//     self_labels: &Vec<TrustRatingLabel>,
//     self_peer_id: &Vec<u8>
// ) -> HashMap<&Vec<u8>, Vec<TrustRatingLabel>> {
//     let mut map: HashMap<&Vec<u8>, Vec<TrustRatingLabel>> = HashMap::new();
//     map.insert(self_peer_id, self_labels.clone());
//     for x in peer_metadata {
//         map.insert(&x.peer_id.clone(), &x.labels);
//     }
//     return map;
// }

// fn calculate_trust(
//     peer_metadata: &Vec<PeerMetadata>,
//     self_labels: &Vec<TrustRatingLabel>
// ) {
//     let mut set: HashSet<Vec<u8>> = HashSet::new();
//     for l in peer_metadata {
//         set.insert(l.peer_id.clone());
//     }
//     for l in self_labels {
//         set.insert(l.peer_id.clone());
//     }
//     let mut peers = set.iter().collect::<Vec<Vec<u8>>>();
//     peers.sort();
//     let mut idx_to_peer : HashMap<usize, &Vec<u8>> = HashMap::default();
//     let mut peer_to_idx : HashMap<&Vec<u8>, usize> = HashMap::default();
//     for (idx, p) in peers.iter().enumerate(){
//         idx_to_peer.insert(idx, p);
//         peer_to_idx.insert(p, idx);
//     }
//
// }

#[test]
fn test_bed() {
    println!("wo")
}
