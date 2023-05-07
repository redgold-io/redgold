use crate::core::relay::Relay;
use crate::schema::structs::TrustLabel;
use std::collections::HashMap;

pub mod eigentrust;
pub mod moon;
pub mod rewards;

#[allow(dead_code)]
struct Trust {
    relay: Relay,
    labels: HashMap<Vec<u8>, HashMap<Vec<u8>, f64>>,
}

// Honestly is this even needed? Just update the internal database and use a model
// every 5 minutes.
#[allow(dead_code)]
impl Trust {
    async fn run(&mut self) {
        loop {
            let update = self.relay.trust.receiver.recv().unwrap();
            match update.remove_peer {
                None => {}
                Some(remove) => {
                    self.labels.remove(&remove);
                    continue;
                }
            }
            let peer_data = update.update;
            if peer_data.labels.is_empty() {
                continue;
            }

            let id = peer_data.peer_id.unwrap().peer_id.unwrap().value;
            let latest = self.labels.get(&id).unwrap();

            let mut label_updates: HashMap<Vec<u8>, f64> = HashMap::new();

            for label in peer_data.labels {
                let l = label.trust_data.get(0).expect("data").label;
                label_updates.insert(label.peer_id, l);
            }

            if label_updates != *latest {
                // self.labels.insert(self.pe, label_updates);
                // self.recalculate_trust()
            }
        }
    }
    #[allow(dead_code)]
    pub fn new(relay: Relay) {
        let mut b = Self {
            relay,
            labels: HashMap::new(),
        };
        tokio::spawn(async move { b.run().await });
    }
}

// https://texample.net/tikz/examples/neural-network/
// https://docs.rs/ndarray/0.15.3/ndarray/doc/ndarray_for_numpy_users/index.html
// fn convert_trust_map(
//     peer_metadata: &Vec<PeerMetadata>,
//     self_labels: &Vec<TrustLabel>,
//     self_peer_id: &Vec<u8>
// ) -> HashMap<&Vec<u8>, Vec<TrustLabel>> {
//     let mut map: HashMap<&Vec<u8>, Vec<TrustLabel>> = HashMap::new();
//     map.insert(self_peer_id, self_labels.clone());
//     for x in peer_metadata {
//         map.insert(&x.peer_id.clone(), &x.labels);
//     }
//     return map;
// }

// http://viz-js.com/
#[allow(dead_code)]
fn datt(
    labels: HashMap<&Vec<u8>, &Vec<TrustLabel>>,
    self_peer_id: &Vec<u8>,
    expansion_threshold: f64,
) {
    let self_trust = labels.get(self_peer_id).unwrap();
    let mut scores: HashMap<&Vec<u8>, f64> = HashMap::new();

    for immediate_peer_label in *self_trust {
        scores.insert(&immediate_peer_label.peer_id, immediate_peer_label.trust_data.get(0).unwrap().label);
    }

    // TODO: Weighted neighbor expansion?
    let current_neighbor_expansion = self_trust.clone().clone();

    while !current_neighbor_expansion.is_empty() {
        let mut next_neighbor_expansion: Vec<TrustLabel> = vec![];
        let mut dot_scores: HashMap<&Vec<u8>, Vec<f64>> = HashMap::new();
        for l in current_neighbor_expansion.clone() {
            if l.trust_data.get(0).unwrap().label > expansion_threshold {
                let outer_labels = labels.get(&l.peer_id).unwrap();
                for l2 in *outer_labels {
                    if !scores.contains_key(&l2.peer_id) {
                        let transitive_current_score = scores.get(&l.peer_id).unwrap();
                        let outer_transitive_score = transitive_current_score * l2.trust_data.get(0).unwrap().label;
                        match dot_scores.get_mut(&l2.peer_id) {
                            None => {
                                dot_scores.insert(&l2.peer_id, vec![outer_transitive_score]);
                            }
                            Some(v) => {
                                v.push(outer_transitive_score);
                            }
                        }
                        if !next_neighbor_expansion.contains(l2) {
                            next_neighbor_expansion.push(l2.clone());
                        }
                    }
                }
            }
        }
        for (k, v) in dot_scores.iter() {
            scores.insert(k, v.iter().sum::<f64>() / (v.len() as f64));
        }
    }
}

// fn calculate_trust(
//     peer_metadata: &Vec<PeerMetadata>,
//     self_labels: &Vec<TrustLabel>
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
