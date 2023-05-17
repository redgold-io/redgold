use bitcoin::secp256k1::rand;
use bitcoin::secp256k1::rand::Rng;
use crate::{Address, constants, ErrorInfo, KeyPair, NetworkEnvironment, PeerData, struct_metadata_new, StructMetadata, Transaction, util};
use crate::constants::{DECIMAL_MULTIPLIER, MAX_COIN_SUPPLY};
use crate::structs::{NodeMetadata, Output, Proof, StandardData, TransactionAmount, TransactionOptions, UtxoEntry};
use crate::transaction::amount_data;

pub struct TransactionBuilder {
    pub transaction: Transaction,
    pub balance: i64
}

impl TransactionBuilder {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        Self {
            transaction: Transaction{
                inputs: vec![],
                outputs: vec![],
                struct_metadata: struct_metadata_new(),
                options: Some(TransactionOptions{
                    salt: Some(rng.gen::<i64>()),
                    network_type: Some(NetworkEnvironment::Debug as i32),
                    key_value_options: vec![],
                    data: None,
                    contract: None
                }),
                hash: None
            },
            balance: 0
        }
    }

    pub fn with_input(&mut self, utxo: UtxoEntry, key_pair: KeyPair) -> &mut Self {
        let mut input = utxo.to_input();

        let data = utxo.output.expect("a").data.expect("b");
        if let Some(a) = data.amount {
            self.balance += a;
        }
        let proof = Proof::new(
            &input.transaction_hash.as_ref().expect("hash"),
            &key_pair.secret_key,
            &key_pair.public_key,
        );
        input.proof.push(proof);
        self.transaction.inputs.push(input);
        self
    }

    pub fn with_output(&mut self, destination: &Address, amount: TransactionAmount) -> &mut Self {
        self.balance -= amount.amount;
        let output = Output {
            address: Some(destination.clone()),
            data: amount_data(amount.amount as u64),
            product_id: None,
            counter_party_proofs: vec![],
            contract: None,
        };
        self.transaction.outputs.push(output);
        self
    }

    pub fn with_output_peer_data(&mut self, destination: &Address, pd: PeerData) -> &mut Self {
        let output = Output {
            address: Some(destination.clone()),
            data: StandardData::peer_data(pd),
            product_id: None,
            counter_party_proofs: vec![],
            contract: None,
        };
        self.transaction.outputs.push(output);
        self
    }

    pub fn with_output_node_metadata(&mut self, destination: &Address, pd: NodeMetadata) -> &mut Self {
        let mut data = StandardData::default();
        data.node_metadata = Some(pd);
        let data = Some(data);
        let output = Output {
            address: Some(destination.clone()),
            data,
            product_id: None,
            counter_party_proofs: vec![],
            contract: None,
        };
        self.transaction.outputs.push(output);
        self
    }

    pub fn with_remainder(&mut self) -> &mut Self {
        let address = self
            .transaction.inputs.get(0)
            .expect("missing head")
            .output
            .as_ref()
            .expect("o")
            .address
            .as_ref()
            .expect("address")
            .clone();

        let output = Output {
            address: Some(address),
            data: amount_data(self.balance as u64),
            product_id: None,
            counter_party_proofs: vec![],
            contract: None,
        };
        self.transaction.outputs.push(output);
        self
    }

}