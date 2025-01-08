use itertools::Itertools;
use crate::party::address_event::AddressEvent;
use crate::party::party_events::{OrderFulfillment, PartyEvents};
use crate::structs::{Address, PublicKey};

pub trait PartyEventSearch {
    fn find_swaps_for_addresses(&self, addrs: &Vec<Address>) -> Vec<(OrderFulfillment, AddressEvent, AddressEvent)>;
}

impl PartyEventSearch for PartyEvents {
    fn find_swaps_for_addresses(&self, addrs: &Vec<Address>) -> Vec<(OrderFulfillment, AddressEvent, AddressEvent)> {
        let addr_str = addrs.iter().map(|a| a.render_string().unwrap()).collect_vec();
        self.fulfillment_history
            .iter()
            .filter_map(|(of, ae1, ae2)| {
                if of.is_stake_withdrawal {
                    None
                }  else if ae1.other_swap_address().map(|a| addr_str.contains(&a)).unwrap_or(false)
                    || ae2.other_swap_address().map(|a| addr_str.contains(&a)).unwrap_or(false) {
                    Some((of.clone(), ae1.clone(), ae2.clone()))
                } else {
                    None
                }
            })
            .collect_vec()
    }
}