use std::collections::HashMap;
use redgold_schema::structs::{PeerId, TrustRatingLabel};
// http://viz-js.com/
// #[allow(dead_code)]
// fn transitive_trust_feature(
//     labels: HashMap<&PeerId, &Vec<TrustRatingLabel>>,
//     self_peer_id: &PeerId,
//     expansion_threshold: f64,
// ) {
//     let self_trust = labels.get(self_peer_id).unwrap();
//     let mut scores: HashMap<PeerId, f64> = HashMap::new();
//
//     for immediate_peer_label in *self_trust {
//         scores.insert(immediate_peer_label.peer_id.expect("pid"), immediate_peer_label.trust_data.get(0).unwrap().label());
//     }
//
//     // TODO: Weighted neighbor expansion?
//     let current_neighbor_expansion = self_trust.clone().clone();
//
//     while !current_neighbor_expansion.is_empty() {
//         let mut next_neighbor_expansion: Vec<TrustRatingLabel> = vec![];
//         let mut dot_scores: HashMap<&Vec<u8>, Vec<f64>> = HashMap::new();
//         for l in current_neighbor_expansion.clone() {
//             if l.trust_data.get(0).unwrap().label() > expansion_threshold {
//                 let outer_labels = labels.get(&l.peer_id).unwrap();
//                 for l2 in *outer_labels {
//                     if !scores.contains_key(&l2.peer_id) {
//                         let transitive_current_score = scores.get(&l.peer_id).unwrap();
//                         let outer_transitive_score = transitive_current_score * l2.trust_data.get(0).unwrap().label();
//                         match dot_scores.get_mut(&l2.peer_id) {
//                             None => {
//                                 dot_scores.insert(&l2.peer_id, vec![outer_transitive_score]);
//                             }
//                             Some(v) => {
//                                 v.push(outer_transitive_score);
//                             }
//                         }
//                         if !next_neighbor_expansion.contains(l2) {
//                             next_neighbor_expansion.push(l2.clone());
//                         }
//                     }
//                 }
//             }
//         }
//         for (k, v) in dot_scores.iter() {
//             scores.insert(k, v.iter().sum::<f64>() / (v.len() as f64));
//         }
//     }
// }
