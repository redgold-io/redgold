use bdk::bitcoin::blockdata::opcodes::all::*;
use bdk::bitcoin::psbt::PartiallySignedTransaction;
use bdk::bitcoin::secp256k1::{PublicKey, Secp256k1};
use bdk::bitcoin::{Address, Network, Script, Transaction};
use bdk::descriptor::Descriptor;
use bdk::descriptor::DescriptorPublicKey;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, RwLock};

// Structure to hold information about a participant in the multisig
#[derive(Clone, Debug)]
pub struct MultisigParticipant {
    pub public_key: PublicKey,
    pub weight: u32,  // For weighted multisig scenarios
}

// Multisig configuration
#[derive(Clone, Debug)]
pub struct MultisigConfig {
    pub threshold: u32,
    pub participants: Vec<MultisigParticipant>,
    pub total_weight: u32,
}

impl MultisigConfig {
    pub fn new(threshold: u32, participants: Vec<MultisigParticipant>) -> Self {
        let total_weight = participants.iter().map(|p| p.weight).sum();
        Self {
            threshold,
            participants,
            total_weight,
        }
    }

    // Validate that the configuration is valid
    pub fn validate(&self) -> Result<(), String> {
        if self.threshold > self.total_weight {
            return Err("Threshold cannot be greater than total weight".to_string());
        }
        if self.participants.is_empty() {
            return Err("Must have at least one participant".to_string());
        }
        Ok(())
    }
}

// Structure for managing multisig spending paths
#[derive(Debug)]
pub struct MultisigSpendInfo {
    pub script: Script,
    pub spend_paths: HashMap<Script, (u32, Vec<MultisigParticipant>)>, // Script -> (required_weight, participants)
}

pub struct MultisigWallet {
    pub config: MultisigConfig,
    pub network: Network,
    pub spend_info: MultisigSpendInfo,
    signatures: Arc<RwLock<HashMap<PublicKey, Vec<u8>>>>,
}

impl MultisigWallet {

    // Get the multisig address
    pub fn get_address(&self) -> String {
        // Create redeem script for multisig
        let redeem_script = self.spend_info.script.clone();

        // For P2WSH
        let address = Address::p2wsh(&redeem_script, self.network).to_string();

        address
    }

    pub fn new(config: MultisigConfig, network: Network) -> Result<Self, String> {
        config.validate()?;

        let secp = Secp256k1::new();
        let mut spend_paths = HashMap::new();

        // Create the base multisig script
        let script = Self::create_multisig_script(&config);

        // Store the script and its requirements
        spend_paths.insert(
            script.clone(),
            (config.threshold, config.participants.clone())
        );

        let spend_info = MultisigSpendInfo {
            script,
            spend_paths,
        };

        Ok(Self {
            config,
            network,
            spend_info,
            signatures: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    // Create the base multisig script
    fn create_multisig_script(config: &MultisigConfig) -> Script {
        let mut script_content: Vec<u8> = Vec::new();

        // Push OP_M (threshold)
        script_content.push(config.threshold as u8 + 0x50); // Convert to OP_N

        // Add each participant's public key
        for participant in &config.participants {
            let key_bytes = participant.public_key.serialize();
            script_content.push(key_bytes.len() as u8);
            script_content.extend_from_slice(&key_bytes);
        }

        // Push OP_N (total number of participants)
        script_content.push(config.participants.len() as u8 + 0x50); // Convert to OP_N

        // Add OP_CHECKMULTISIG
        script_content.push(OP_CHECKMULTISIG.into_u8());

        Script::new().into()
    }

    // Add a signature from a participant
    pub fn add_signature(&self, pubkey: PublicKey, signature: Vec<u8>) -> Result<(), String> {
        // Verify the signature belongs to a participant
        if !self.config.participants.iter().any(|p| p.public_key == pubkey) {
            return Err("Signature from unknown participant".to_string());
        }

        let mut sigs = self.signatures.write().map_err(|_| "Lock error")?;
        sigs.insert(pubkey, signature);
        Ok(())
    }

    // Check if we have enough signatures to complete the transaction
    pub fn has_enough_signatures(&self) -> bool {
        let sigs = self.signatures.read().unwrap();
        let current_weight: u32 = self.config.participants
            .iter()
            .filter(|p| sigs.contains_key(&p.public_key))
            .map(|p| p.weight)
            .sum();

        current_weight >= self.config.threshold
    }

    // Create a descriptor for the wallet
    pub fn create_descriptor(&self) -> Result<Descriptor<DescriptorPublicKey>, String> {
        let threshold = self.config.threshold as usize;

        // Convert regular public keys to descriptor public keys
        let keys: Result<Vec<DescriptorPublicKey>, _> = self.config.participants
            .iter()
            .map(|p| {
                let key_str = format!("[{}]{}", "00000000/0'/0'/0']", p.public_key.to_string());
                DescriptorPublicKey::from_str(&key_str)
                    .map_err(|e| format!("Error converting key: {}", e))
            })
            .collect();

        let keys = keys?;

        let desc = format!("wsh(multi({},{}))",
                           threshold,
                           keys.iter()
                               .map(|k| k.to_string())
                               .collect::<Vec<_>>()
                               .join(",")
        );

        Descriptor::from_str(&desc)
            .map_err(|e| format!("Error creating descriptor: {}", e))
    }

    // Sign a transaction input
    pub fn sign_input(
        &self,
        tx: &Transaction,
        input_index: usize,
        pubkey: PublicKey,
        signature: Vec<u8>
    ) -> Result<(), String> {
        // Verify the signature belongs to a valid participant
        self.add_signature(pubkey, signature)?;
        Ok(())
    }

    // Finalize the transaction if we have enough signatures
    pub fn finalize_transaction(&self, tx: Transaction) -> Result<Transaction, String> {
        if !self.has_enough_signatures() {
            return Err("Not enough signatures to finalize".to_string());
        }
        Ok(tx)
    }
    //
    // // Create a new transaction with multiple outputs
    // pub fn create_transaction(
    //     &self,
    //     utxos: Vec<(Transaction, u32)>, // Previous transactions and output indices to spend
    //     destinations: Vec<(String, u64)>, // Destination addresses and amounts
    //     fee_rate: f32, // Sats per vbyte
    // ) -> Result<PartiallySignedTransaction, String> {
    //
    //     // Create transaction inputs from UTXOs
    //     let inputs: Vec<TxIn> = utxos.iter()
    //         .map(|(tx, vout)| TxIn {
    //             previous_output: OutPoint {
    //                 txid: tx.txid(),
    //                 vout: *vout,
    //             },
    //             sequence: 0xFFFFFFFF,
    //             witness: Vec::new(),
    //             script_sig: Script::new(),
    //         })
    //         .collect();
    //
    //     // Create transaction outputs
    //     let outputs: Result<Vec<TxOut>, String> = destinations.iter()
    //         .map(|(addr_str, amount)| {
    //             let addr = Address::from_str(addr_str)
    //                 .map_err(|e| format!("Invalid address {}: {}", addr_str, e))?;
    //             Ok(TxOut {
    //                 value: *amount,
    //                 script_pubkey: addr.script_pubkey(),
    //             })
    //         })
    //         .collect();
    //     let outputs = outputs?;
    //
    //     // Create unsigned transaction
    //     let unsigned_tx = Transaction {
    //         version: 2,
    //         lock_time: 0,
    //         input: inputs,
    //         output: outputs,
    //     };
    //
    //     // Create PSBT
    //     let mut psbt = PartiallySignedTransaction::from_unsigned_tx(unsigned_tx)
    //         .map_err(|e| format!("Error creating PSBT: {}", e))?;
    //
    //     // Add information about our inputs to the PSBT
    //     for ((prev_tx, vout), psbt_input) in utxos.iter().zip(psbt.inputs.iter_mut()) {
    //         // Add the full previous transaction
    //         psbt_input.non_witness_utxo = Some(prev_tx.clone());
    //
    //         // Add the redeem script
    //         psbt_input.witness_script = Some(self.spend_info.script.clone());
    //     }
    //
    //     Ok(psbt)
    // }
    //
    // // Add a signature to the PSBT
    // pub fn add_signature_to_psbt(
    //     &self,
    //     mut psbt: PartiallySignedTransaction,
    //     input_index: usize,
    //     pubkey: PublicKey,
    //     signature: Vec<u8>
    // ) -> Result<PartiallySignedTransaction, String> {
    //     if input_index >= psbt.inputs.len() {
    //         return Err("Invalid input index".to_string());
    //     }
    //
    //     let psbt_input = &mut psbt.inputs[input_index];
    //
    //     // Convert the raw signature to bitcoin ECDSA signature format
    //     let sig = Signature::from_compact(&signature)
    //         .map_err(|e| format!("Invalid signature: {}", e))?;
    //
    //     // Add to partial sigs
    //     psbt_input.partial_sigs.insert(pubkey, sig);
    //
    //     Ok(psbt)
    // }

    // Finalize the PSBT into a transaction
    pub fn finalize_psbt(
        &self,
        psbt: PartiallySignedTransaction
    ) -> Result<Transaction, String> {
        // Verify we have enough signatures
        if !self.has_enough_signatures() {
            return Err("Not enough signatures to finalize".to_string());
        }

        // Extract the final transaction
        let tx = psbt.extract_tx();
        Ok(tx)
    }
}

// Example usage and tests
#[cfg(test)]
mod tests {
    use super::*;
    use bdk::bitcoin::secp256k1::rand::rngs::OsRng;

    #[test]
    fn test_multisig_creation() {
        let secp = Secp256k1::new();
        let mut rng = OsRng::default();

        // Create some test participants
        let participants = (0..3).map(|_| {
            let (secret_key, public_key) = secp.generate_keypair(&mut rng);
            MultisigParticipant {
                public_key,
                weight: 1,
            }
        }).collect();

        let config = MultisigConfig::new(2, participants); // 2-of-3 multisig

        let wallet = MultisigWallet::new(
            config,
            Network::Testnet
        ).expect("Failed to create wallet");

        assert!(!wallet.has_enough_signatures());
    }

    #[test]
    fn test_descriptor_creation() {
        let secp = Secp256k1::new();
        let mut rng = OsRng::default();

        // Create some test participants
        let participants = (0..3).map(|_| {
            let (secret_key, public_key) = secp.generate_keypair(&mut rng);
            MultisigParticipant {
                public_key,
                weight: 1,
            }
        }).collect();

        let config = MultisigConfig::new(2, participants); // 2-of-3 multisig
        let wallet = MultisigWallet::new(config, Network::Testnet).expect("Failed to create wallet");

        let descriptor = wallet.create_descriptor().expect("Failed to create descriptor");
        println!("Created descriptor: {}", descriptor);
    }
}