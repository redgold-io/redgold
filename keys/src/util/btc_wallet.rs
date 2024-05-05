use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, RwLock};

use bdk::{Balance, FeeRate, KeychainKind, SignOptions, sled, SyncOptions, TransactionDetails, Wallet};
use bdk::bitcoin::{Address, ecdsa, EcdsaSighashType, Network, Script, Sighash, TxIn, TxOut};
use bdk::bitcoin::blockdata::opcodes;
use bdk::bitcoin::blockdata::script::Builder as ScriptBuilder;
use bdk::bitcoin::hashes::Hash;
use bdk::bitcoin::secp256k1::{All, Secp256k1, Signature};
use bdk::bitcoin::util::{psbt, sighash};
use bdk::bitcoin::util::psbt::PartiallySignedTransaction;
use bdk::blockchain::{Blockchain, ElectrumBlockchain, GetTx};
use bdk::database::{BatchDatabase, MemoryDatabase};
use bdk::electrum_client::Client;
use bdk::signer::{InputSigner, SignerCommon, SignerError, SignerId, SignerOrdering};
use bdk::sled::Tree;
use itertools::Itertools;
// use crate::util::cli::commands::send;
use redgold_schema::{error_info, ErrorInfoContext, RgResult, SafeOption, structs};
use redgold_schema::structs::{CurrencyAmount, ErrorInfo, NetworkEnvironment, Proof, PublicKey, SupportedCurrency};
use serde::{Deserialize, Serialize};
// use crate::util::cli::commands::send;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::proto_serde::ProtoSerde;
use crate::{KeyPair, TestConstants};
use crate::address_external::ToBitcoinAddress;
use crate::address_support::AddressSupport;
use crate::eth::example::dev_ci_kp;
use crate::proof_support::ProofSupport;
use crate::util::keys::ToPublicKeyFromLib;
use crate::util::mnemonic_support::{test_pkey_hex, test_pubk};


#[test]
fn schnorr_test() {
    let tc = TestConstants::new();
    let _kp = tc.key_pair();

}

pub fn struct_public_to_address(pk: structs::PublicKey, network: Network) -> Result<Address, ErrorInfo> {
    let pk2 = bdk::bitcoin::util::key::PublicKey::from_slice(&*pk.raw_bytes()?)
        .error_info("Unable to convert destination pk to bdk public key")?;
    let addr = Address::p2wpkh(&pk2, network)
        .error_info("Unable to convert destination pk to bdk address")?;
    Ok(addr)
}

pub fn struct_public_to_bdk_pubkey(pk: &structs::PublicKey) -> Result<bdk::bitcoin::util::key::PublicKey, ErrorInfo> {
    let pk2 = bdk::bitcoin::util::key::PublicKey::from_slice(&*pk.raw_bytes()?)
        .error_info("Unable to convert destination pk to bdk public key")?;
    Ok(pk2)
}


// use log::error;

fn p2wpkh_script_code(script: &Script) -> Script {
    ScriptBuilder::new()
        .push_opcode(opcodes::all::OP_DUP)
        .push_opcode(opcodes::all::OP_HASH160)
        .push_slice(&script[2..])
        .push_opcode(opcodes::all::OP_EQUALVERIFY)
        .push_opcode(opcodes::all::OP_CHECKSIG)
        .into_script()
}

// type Extra = ();
// type Sighash = bitcoin::Sighash;
// type SighashType = EcdsaSighashType;

fn segwit_sighash(
    psbt: &psbt::PartiallySignedTransaction,
    input_index: usize,
    _extra: (),
) -> Result<(Sighash, EcdsaSighashType), SignerError> {
    if input_index >= psbt.inputs.len() || input_index >= psbt.unsigned_tx.input.len() {
        return Err(SignerError::InputIndexOutOfRange);
    }

    let psbt_input = &psbt.inputs[input_index];
    let tx_input = &psbt.unsigned_tx.input[input_index];

    let sighash = psbt_input
        .sighash_type
        .unwrap_or_else(|| EcdsaSighashType::All.into())
        .ecdsa_hash_ty()
        .map_err(|_| SignerError::InvalidSighash)?;

    // Always try first with the non-witness utxo
    let utxo = if let Some(prev_tx) = &psbt_input.non_witness_utxo {
        // Check the provided prev-tx
        if prev_tx.txid() != tx_input.previous_output.txid {
            return Err(SignerError::InvalidNonWitnessUtxo);
        }

        // The output should be present, if it's missing the `non_witness_utxo` is invalid
        prev_tx
            .output
            .get(tx_input.previous_output.vout as usize)
            .ok_or(SignerError::InvalidNonWitnessUtxo)?
    } else if let Some(witness_utxo) = &psbt_input.witness_utxo {
        // Fallback to the witness_utxo. If we aren't allowed to use it, signing should fail
        // before we get to this point
        witness_utxo
    } else {
        // Nothing has been provided
        return Err(SignerError::MissingNonWitnessUtxo);
    };
    let value = utxo.value;

    let script = match psbt_input.witness_script {
        Some(ref witness_script) => witness_script.clone(),
        None => {
            if utxo.script_pubkey.is_v0_p2wpkh() {
                p2wpkh_script_code(&utxo.script_pubkey)
            } else if psbt_input
                .redeem_script
                .as_ref()
                .map(Script::is_v0_p2wpkh)
                .unwrap_or(false)
            {
                p2wpkh_script_code(psbt_input.redeem_script.as_ref().unwrap())
            } else {
                return Err(SignerError::MissingWitnessScript);
            }
        }
    };

    Ok((
        sighash::SighashCache::new(&psbt.unsigned_tx).segwit_signature_hash(
            input_index,
            &script,
            value,
            sighash,
        )?,
        sighash,
    ))
}


#[derive(Debug, Clone)]
struct MultipartySigner {
    public_key: structs::PublicKey,
    proofs: Arc<RwLock<HashMap<usize, Proof>>>,
    err: Arc<RwLock<Option<ErrorInfo>>>
}

impl MultipartySigner {
    pub fn new(public_key: structs::PublicKey) -> Self {
        Self {
            public_key,
            proofs: Default::default(),
            err: Arc::new(RwLock::new(None)),
        }
    }
    pub fn sign_input(&self,
                      psbt: &mut PartiallySignedTransaction,
                      input_index: usize,
                      hash_ty: EcdsaSighashType,
                      _sign_options: &SignOptions
    ) -> Result<(), ErrorInfo> {
        let arc = self.proofs.clone();
        let guard = arc.read().unwrap();
        let proof = guard.get(&input_index).ok_or(error_info("No proof found"))?;
        let signature = proof.signature.safe_get_msg("Missing signature in proof")?;
        let sig = Signature::from_compact(&*signature.raw_bytes()?).error_msg(
            structs::ErrorCode::IncorrectSignature,
            "Decoded signature construction failure",
        )?;

        let final_signature = ecdsa::EcdsaSig { sig, hash_ty };

        let public_key = proof.public_key.safe_get_msg("Missing public key")?.raw_bytes()?;
        let public_key = bdk::bitcoin::util::key::PublicKey::from_slice(&*public_key)
            .error_info("Public key failure")?;

        let input = psbt.inputs.get_mut(input_index).ok_or(error_info("No psbt found"))?;
        input
            .partial_sigs
            .insert(public_key, final_signature);

        Ok(())
    }
}

impl SignerCommon for MultipartySigner {
    fn id(&self, _secp: &Secp256k1<All>) -> SignerId {
        let pk = struct_public_to_bdk_pubkey(&self.public_key).unwrap();
        SignerId::PkHash(pk.pubkey_hash().as_hash())
    }
}

impl InputSigner for MultipartySigner {
    fn sign_input(&self,
                  psbt: &mut PartiallySignedTransaction,
                  input_index: usize,
                  sign_options: &SignOptions, _secp: &Secp256k1<All>
    ) -> Result<(), SignerError> {
        let (_, sighash_type) = segwit_sighash(psbt, input_index, ())?;
        match self.sign_input(psbt, input_index, sighash_type, sign_options) {
            Ok(_) => {
                Ok(())
            }
            Err(e) => {
                *self.err.write().unwrap() = Some(e);
                Err(SignerError::UserCanceled)
            }
        }
    }
}


pub struct SingleKeyBitcoinWallet<D: BatchDatabase> {
    wallet: Wallet<D>,
    pub public_key: structs::PublicKey,
    network: Network,
    pub psbt: Option<PartiallySignedTransaction>,
    pub transaction_details: Option<TransactionDetails>,
    client: ElectrumBlockchain,
    custom_signer: Arc<MultipartySigner>
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RawTransaction {
    pub psbt: Option<PartiallySignedTransaction>,
    pub transaction_details: Option<TransactionDetails>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalTimedTransaction {
    pub tx_id: String,
    pub timestamp: Option<u64>,
    pub other_address: String,
    pub other_output_addresses: Vec<String>,
    pub amount: u64,
    pub bigint_amount: Option<String>,
    pub incoming: bool,
    pub currency: SupportedCurrency,
    pub block_number: Option<u64>,
    pub price_usd: Option<f64>,
}

impl ExternalTimedTransaction {

    pub fn other_address_typed(&self) -> RgResult<structs::Address> {
        self.other_address.parse_address()
    }

    pub fn currency_amount(&self) -> CurrencyAmount {
        let mut ca = if let Some(ba) = self.bigint_amount.as_ref() {
            CurrencyAmount::from_eth_bigint_string(ba.clone())
        } else {
            CurrencyAmount::from(self.amount as i64)
        };
        ca.currency = Some(self.currency as i32);
        ca
    }
    pub fn confirmed(&self) -> bool {
        self.timestamp.is_some()
    }

}

impl SingleKeyBitcoinWallet<MemoryDatabase> {

    pub fn new_wallet(
        public_key: PublicKey,
        network: NetworkEnvironment,
        do_sync: bool
    ) -> Result<Self, ErrorInfo> {
        let network = if network == NetworkEnvironment::Main {
            Network::Bitcoin
        } else {
            Network::Testnet
        };
        let client = Client::new("ssl://electrum.blockstream.info:60002")
            .error_info("Error building bdk client")?;
        let client = ElectrumBlockchain::from(client);
        let database = MemoryDatabase::default();
        let hex = public_key.to_hex_direct_ecdsa()?;
        let descr = format!("wpkh({})", hex);
        let wallet = Wallet::new(
            &*descr,
            Some(&*descr),
            network,
            database
        ).error_info("Error creating BDK wallet")?;
        let custom_signer = Arc::new(MultipartySigner::new(public_key.clone()));
        let mut bitcoin_wallet = Self {
            wallet,
            public_key,
            network,
            psbt: None,
            transaction_details: None,
            client,
            custom_signer: custom_signer.clone(),
        };
        // Adding the multiparty signer to the BDK wallet
        bitcoin_wallet.wallet.add_signer(
            KeychainKind::External,
            SignerOrdering(200),
            custom_signer.clone(),
        );

        if do_sync {
            bitcoin_wallet.sync()?;
        }
        Ok(bitcoin_wallet)
    }
}
impl SingleKeyBitcoinWallet<Tree> {
    pub fn new_wallet_db_backed(
        public_key: PublicKey,
        network: NetworkEnvironment,
        do_sync: bool,
        database_path: PathBuf
    ) -> Result<Self, ErrorInfo> {
        let network = if network == NetworkEnvironment::Main {
            Network::Bitcoin
        } else {
            Network::Testnet
        };
        let client = Client::new("ssl://electrum.blockstream.info:60002")
            .error_info("Error building bdk client")?;
        let client = ElectrumBlockchain::from(client);
        // KeyValueDatabase
        // Create a database (using default sled type) to store wallet data
        let database = sled::open(database_path).error_info("Sled database open error")?;
        let wallet_name = public_key.hex();
        let database = database.open_tree(wallet_name.clone()).error_info("Database open tree error")?;
        // let database = MemoryDatabase::default();
        let hex = public_key.to_hex_direct_ecdsa()?;
        let descr = format!("wpkh({})", hex);
        let wallet = Wallet::new(
            &*descr,
            Some(&*descr),
            network,
            database
        ).error_info("Error creating BDK wallet")?;
        let custom_signer = Arc::new(MultipartySigner::new(public_key.clone()));
        let mut bitcoin_wallet = Self {
            wallet,
            public_key,
            network,
            psbt: None,
            transaction_details: None,
            client,
            custom_signer: custom_signer.clone(),
        };
        // Adding the multiparty signer to the BDK wallet
        bitcoin_wallet.wallet.add_signer(
            KeychainKind::External,
            SignerOrdering(200),
            custom_signer.clone(),
        );

        if do_sync {
            bitcoin_wallet.sync()?;
        }
        Ok(bitcoin_wallet)
    }
}
impl<D: BatchDatabase> SingleKeyBitcoinWallet<D> {

    //
    // pub fn new_hardware_wallet(
    //     public_key: structs::PublicKey,
    //     network: NetworkEnvironment,
    //     do_sync: bool
    // ) -> Result<Self, ErrorInfo> {
    //     let network = if network == NetworkEnvironment::Main {
    //         Network::Bitcoin
    //     } else {
    //         Network::Testnet
    //     };
    //     let client = Client::new("ssl://electrum.blockstream.info:60002")
    //         .error_info("Error building bdk client")?;
    //     let client = ElectrumBlockchain::from(client);
    //     let database = MemoryDatabase::default();
    //     let hex = public_key.hex_or();
    //     let descr = format!("wpkh({})", hex);
    //     let wallet = Wallet::new(
    //         &*descr,
    //         Some(&*descr),
    //         network,
    //         database
    //     ).error_info("Error creating BDK wallet")?;
    //     // let custom_signer = Arc::new(MultipartySigner::new(public_key.clone()));
    //     let mut devices = HWIClient::enumerate()?;
    //     if devices.is_empty() {
    //         panic!("No devices found!");
    //     }
    //     let first_device = devices.remove(0)?;
    //     let custom_signer = HWISigner::from_device(&first_device, HWIChain::Test)?;
    //
    //
    //     let mut bitcoin_wallet = Self {
    //         wallet,
    //         public_key,
    //         network,
    //         psbt: None,
    //         transaction_details: None,
    //         client,
    //         custom_signer: custom_signer.clone(),
    //     };
    //     // Adding the multiparty signer to the BDK wallet
    //     bitcoin_wallet.wallet.add_signer(
    //         KeychainKind::External,
    //         SignerOrdering(200),
    //         custom_signer.clone(),
    //     );
    //
    //     if do_sync {
    //         bitcoin_wallet.sync()?;
    //     }
    //     Ok(bitcoin_wallet)
    // }

    pub fn sync(&self) -> Result<(), ErrorInfo> {
        self.wallet.sync(&self.client, SyncOptions::default()).error_info("Error syncing BDK wallet")?;
        Ok(())
    }

    pub fn address(&self) -> Result<String, ErrorInfo> {
        let pk2 = bdk::bitcoin::util::key::PublicKey::from_slice(&*self.public_key.raw_bytes()?)
            .error_info("Unable to convert destination pk to bdk public key")?;
        let addr = bdk::bitcoin::util::address::Address::p2wpkh(&pk2, self.network)
            .error_info("Unable to convert destination pk to bdk address")?;
        Ok(addr.to_string())
    }

    pub fn parse_address(addr: &String) -> RgResult<Address> {
        Address::from_str(&addr).error_info("Unable to convert destination pk to bdk address")
    }

    pub fn get_sourced_tx(&self) -> Result<Vec<ExternalTimedTransaction>, ErrorInfo> {
        let self_addr = self.address()?;
        let mut res = vec![];
        let result = self.wallet.list_transactions(true)
            .error_info("Error listing transactions")?;
        for x in result.iter() {
            let tx = x.transaction.safe_get_msg("Error getting transaction")?;
            let mut to_self_output_amount: Option<u64> = None;
            let mut non_self_addrs_output = vec![];
            for o in &tx.output {
                if let Some(a) = Address::from_script(&o.script_pubkey, self.network).ok() {
                    if a.to_string() == self_addr {
                        // sum value here instead?
                        to_self_output_amount = Some(o.value)
                    } else {
                        non_self_addrs_output.push(a.to_string())
                    }
                }
            }
            // This is probably fine for now, but we should really keep track of all inputs
            // in the event of use of multiple addresses?
            let mut non_self_input_addr: Option<String> = None;
            for i in &tx.input {
                let txid = i.previous_output.txid;
                let vout = i.previous_output.vout;
                let prev_tx = self.client.get_tx(&txid).error_info("Error getting tx")?;
                let prev_tx = prev_tx.safe_get_msg("No tx found")?;
                let prev_output = prev_tx.output.get(vout as usize);
                let prev_output = prev_output.safe_get_msg("Error getting output")?;
                let a = Address::from_script(&prev_output.script_pubkey, self.network).ok();
                // println!("{}", format!("TxIn address: {:?}", a));
                if let Some(a) = a {
                    let a = a.to_string();
                    if a != self_addr {
                        non_self_input_addr = Some(a)
                    }
                }
            }

            // println!("{}", format!("Transaction: {} received: {} sent: {} non_self_input_addr {} \
            // nonself_output_addr {}",
            //                        x.txid, x.received, x.sent,
            //                        non_self_input_addr.unwrap_or("None".to_string()),
            //                        to_self_output_amount.unwrap_or(0)
            // ));
            if let (Some(c), Some(a), Some(value)) =
                (x.confirmation_time.clone(), non_self_input_addr, to_self_output_amount) {

                let ett = ExternalTimedTransaction {
                    tx_id: x.txid.to_string(),
                    timestamp: Some(c.timestamp),
                    other_address: a,
                    other_output_addresses: non_self_addrs_output,
                    amount: value,
                    bigint_amount: None,
                    incoming: true,
                    currency: SupportedCurrency::Bitcoin,
                    block_number: None,
                    price_usd: None,
                };
                res.push(ett)
            }
        }
        Ok(res)
    }

    pub fn outputs_convert(&self, outs: &Vec<TxOut>) -> Vec<(String, u64)> {
        let mut res = vec![];
        for o in outs {
            let a = Address::from_script(&o.script_pubkey, self.network).ok();
            if let Some(a) = a {
                res.push((a.to_string(), o.value))
            }
        }
        res
    }

    pub fn convert_tx_inputs_address(&self, tx_ins: &Vec<TxIn>) -> RgResult<Vec<(String, u64)>> {
        let mut res = vec![];
        for i in tx_ins {
            let txid = i.previous_output.txid;
            let vout = i.previous_output.vout;
            let prev_tx = self.client.get_tx(&txid).error_info("Error getting tx")?;
            let prev_tx = prev_tx.safe_get_msg("No tx found")?;
            let prev_output = prev_tx.output.get(vout as usize);
            let prev_output = prev_output.safe_get_msg("Error getting output")?;
            let amount = prev_output.value;
            let a = Address::from_script(&prev_output.script_pubkey, self.network).ok();
            // println!("{}", format!("TxIn address: {:?}", a));
            if let Some(a) = a {
                let a = a.to_string();
                res.push((a, amount));
            }
        }
        Ok(res)
    }
    pub fn get_all_tx(&self) -> Result<Vec<ExternalTimedTransaction>, ErrorInfo> {
        let self_addr = self.address()?;
        let mut res = vec![];
        let result = self.wallet.list_transactions(true)
            .error_info("Error listing transactions")?;
        for x in result.iter() {
            let tx = x.transaction.safe_get_msg("Error getting transaction")?;
            let output_amounts = self.outputs_convert(&tx.output);
            let other_output_addresses = output_amounts.iter().filter_map(|(x,_y)| {
                if x != &self_addr {
                    Some(x.clone())
                } else {
                    None
                }
            }).collect();
            let input_addrs = self.convert_tx_inputs_address(&tx.input)?;

            // Not needed?
            // let has_self_output = output_amounts.iter().filter(|(x,y)| x != &self_addr).next().is_some();
            let has_self_input = input_addrs.iter().filter(|(x,_y)| x == &self_addr).next().is_some();
            let incoming = !has_self_input;

            let other_address = if incoming {
                input_addrs.iter().filter(|(x,_y)| x != &self_addr).next().map(|(x,_y)| x.clone())
            } else {
                output_amounts.iter().filter(|(x,_y)| x != &self_addr).next().map(|(x,_y)| x.clone())
            };

            let amount = if incoming {
                output_amounts.iter().filter(|(x,_y)| x == &self_addr).next().map(|(_x,y)| y.clone())
            } else {
                output_amounts.iter().filter(|(x,_y)| x != &self_addr).next().map(|(_x,y)| y.clone())
            };

            let block_timestamp = x.confirmation_time.clone().map(|x| x.timestamp).map(|t| t * 1000);

            if let (Some(a), Some(value)) = (other_address, amount) {

                let ett = ExternalTimedTransaction {
                    tx_id: x.txid.to_string(),
                    timestamp: block_timestamp,
                    other_address: a,
                    other_output_addresses,
                    amount: value,
                    bigint_amount: None,
                    incoming,
                    currency: SupportedCurrency::Bitcoin,
                    block_number: None,
                    price_usd: None,
                };
                res.push(ett)
            }
        }
        Ok(res)
    }

    pub fn get_wallet_balance(&self
    ) -> Result<Balance, ErrorInfo> {
        self.sync()?;
        let balance = self.wallet.get_balance().error_info("Error getting BDK wallet balance")?;
        Ok(balance)
    }

    pub fn create_transaction(&mut self, destination: Option<structs::PublicKey>, destination_str: Option<String>, amount: u64) -> Result<(), ErrorInfo> {

        let addr = if let Some(destination) = destination {
            let pk2 = bdk::bitcoin::util::key::PublicKey::from_slice(&*destination.raw_bytes()?)
                .error_info("Unable to convert destination pk to bdk public key")?;
            let addr = Address::p2wpkh(&pk2, self.network)
                .error_info("Unable to convert destination pk to bdk address")?;
            addr
        } else if let Some(d) = destination_str {
            Address::from_str(&*d).error_info("Unable to parse address")?
        } else {
            return Err(error_info("No destination specified".to_string()))
        };

        println!("Source address: {}", self.address()?);
        println!("Send to address: {}", addr.to_string());
        self.sync()?;

        let mut builder = self.wallet.build_tx();
        builder
            .add_recipient(addr.script_pubkey(), amount)
            .enable_rbf()
            .fee_rate(FeeRate::from_sat_per_vb(1.0));

        let (psbt, details) = builder
            .finish()
            .error_info("Builder TX issue")?;

        self.transaction_details = Some(details);
        self.psbt = Some(psbt);
        // self.custom_signer.proofs = HashMap::new();
        Ok(())
    }

    pub fn create_transaction_output_batch(&mut self, destinations: Vec<(String, u64)>) -> Result<(), ErrorInfo> {

        self.sync()?;

        let mut builder = self.wallet.build_tx();

        builder.enable_rbf()
            .fee_rate(FeeRate::from_sat_per_vb(1.0));

        for (d, amount) in destinations {
            let addr = Address::from_str(&*d).error_info("Unable to parse address")?;
            builder
                .add_recipient(addr.script_pubkey(), amount);
        }

        let (psbt, details) = builder
            .finish()
            .error_info("Builder TX issue")?;

        self.transaction_details = Some(details);
        self.psbt = Some(psbt);
        Ok(())
    }

    // pub fn psbt_outputs(&self) -> RgResult<Vec<(structs::Address, i64)>> {
    //     let psbt = self.psbt.safe_get_msg("No psbt found")?;
    //     for o in psbt.outputs.iter() {
    //         o.redeem_script
    //     }
    // }

    pub fn txid(&self) -> Result<String, ErrorInfo> {
        let txid = self.transaction_details.safe_get_msg("No psbt found")?.txid;
        Ok(txid.to_string())
    }

    pub fn signable_hashes(&mut self) -> Result<Vec<(Vec<u8>, EcdsaSighashType)>, ErrorInfo> {
        let psbt = self.psbt.safe_get_msg("No psbt found")?.clone();
        let mut res = vec![];
        for (input_index, _input) in psbt.inputs.iter().enumerate() {
            // TODO: Port SignerContext if necessary
            // let (hash, sighash) = match input.witness_utxo {
            //     Some(_) => segwitv0_sighash(&psbt, input_index).error_info("segwitv0_sighash extraction failure")?,
            //     None => legacy_sighash(&psbt, input_index).error_info("segwitv0_legacy signature hash extraction failure")?,
            // };
            let (hash, sighash) = segwit_sighash(&psbt, input_index, ())
                .error_info("segwitv0_sighash extraction failure")?;
            let data = hash.into_inner().to_vec();
            res.push((data, sighash));
        };
        Ok(res)
    }

    // pub fn pre_signing(&mut self) -> Result<(), ErrorInfo> {
    //     if let Some(psbt) = &self.psbt {
    //         self.wallet.update_psbt_with_descriptor(psbt).error_info();
    //     }
    //
    //     Ok(())
    // }

    pub fn sign(&mut self)
        -> Result<bool, ErrorInfo> {
        let res = if let Some(psbt) = self.psbt.as_mut() {
            self.wallet.sign(psbt, SignOptions::default())
                .map_err(|_e| self.custom_signer.err.read().unwrap().clone().unwrap().clone())
        } else {
            return Err(error_info("No psbt found"))
        };
        res
    }
    pub fn affix_input_signature(&self, input_index: usize, proof: &Proof, _sighashtype: &EcdsaSighashType) {
        self.custom_signer.proofs.write().unwrap().insert(input_index, proof.clone());
    }

    pub fn broadcast_tx(&mut self) -> Result<(), ErrorInfo> {
        let psbt = self.psbt.safe_get()?;
        let transaction = psbt.clone().extract_tx();
        self.client.broadcast(&transaction).error_info("Error broadcasting transaction")?;
        Ok(())
    }

    // TODO: How to implement this check native to BDK?
    pub fn verify(&mut self) -> Result<(), ErrorInfo> {
        let psbt = self.psbt.safe_get()?;
        let _transaction = psbt.clone().extract_tx();
        let _transaction_details = self.transaction_details.safe_get()?;
        // psbt.extract_tx()
        // psbt.clone().extract_tx().verify_with_flags()
        Ok(())
    }


    // Same as below

    // Used for rendering json for gui
    pub fn prepare_single(&mut self, dest: String, amount: f64) -> RgResult<String> {
        let amount = (amount * (1e8f64)) as u64;
        self.create_transaction_output_batch(vec![(dest, amount)])?;
        self.render_json()
    }

    pub fn render_json(&self) -> RgResult<String> {
        RawTransaction {
            psbt: self.psbt.clone(),
            transaction_details: self.transaction_details.clone(),
        }.json()
    }

    pub fn prepare_single_sign(&mut self, dest: String, amount: f64, pkey_hex: String) -> RgResult<String> {
        self.prepare_single(dest, amount)?;
        self.local_sign_single(pkey_hex)
    }

    pub fn prepare_single_sign_and_broadcast(&mut self, dest: String, amount: f64, pkey_hex: String) -> RgResult<String> {
        self.prepare_single(dest, amount)?;
        self.local_sign_single(pkey_hex)?;
        self.broadcast_tx()?;
        let txid = self.transaction_details.safe_get_msg("No psbt found")?.txid.to_string();
        Ok(txid)
    }

    pub fn local_sign_single(&mut self, pkey_hex: String) -> RgResult<String> {
        let kp = KeyPair::from_private_hex(pkey_hex)?;
        let signables = self.signable_hashes()?;
        for (i, (hash, sighashtype)) in signables.iter().enumerate() {
            // println!("signable {}: {}", i, hex::encode(hash));
            let prf = Proof::from_keypair(hash, kp);
            self.affix_input_signature(i, &prf, sighashtype);
        }
        let finalized = self.sign()?;
        if !finalized {
            return Err(error_info("Not finalized"));
        }
        self.render_json()
    }

    pub fn send_local(&mut self, dest: String, amount: u64, pkey_hex: String) -> RgResult<String> {
        self.create_transaction_output_batch(vec![(dest, amount)])?;
        let kp = KeyPair::from_private_hex(pkey_hex)?;
        // let d = w.transaction_details.clone().expect("d");
        // println!("txid: {:?}", d.txid);
        let signables = self.signable_hashes()?;
        // println!("num signable hashes: {:?}", signables.len());
        for (i, (hash, sighashtype)) in signables.iter().enumerate() {
            // println!("signable {}: {}", i, hex::encode(hash));
            let prf = Proof::from_keypair(hash, kp);
            self.affix_input_signature(i, &prf, sighashtype);
        }
        let finalized = self.sign()?;
        if !finalized {
            return Err(error_info("Not finalized"));
        }
        // println!("finalized: {:?}", finalized);

        self.broadcast_tx()?;

        let txid = self.transaction_details.safe_get_msg("No psbt found")?.txid.to_string();
        // let txid = w.broadcast_tx().expect("txid");
        // println!("txid: {:?}", txid);
        Ok(txid)
    }

    pub fn convert_psbt_outputs(&self) -> Vec<(String, u64)> {
        let tx = self.psbt.clone().expect("psbt").extract_tx();
        let outputs = self.outputs_convert(&tx.output);
        outputs
    }

}

/*
balance: Balance { immature: 0, trusted_pending: 0, untrusted_pending: 0, confirmed: 6817 }
Source address: tb1q0287j37tntffkndch8fj38s2f994xk06rlr4w4
Send to address: tb1q68rhft47r5jwq5832k9urtypggpvzyh5z9c9gn
txid: 8080a06c0671d1492a24ef60fc1771cbba44cc5387dd754e434de3df4f8e9e5c
num signable hashes: 1
signable 0: a19abb028d61f5add0fbb033bbbe22677f9ab658e648b95ec84eb93edf5d81c4
finalized: true
test integrations::bitcoin::bdk_example::balance_test ... ok

 */

#[ignore]
#[tokio::test]
async fn tx_debug() {
    // MnemonicWords::from_mnemonic_words()
    let _pkey = test_pkey_hex().expect("");
    let public = test_pubk().expect("");
    println!("Public key rg address {}", public.address().expect("").render_string().expect(""));
    let w = SingleKeyBitcoinWallet
    ::new_wallet(public, NetworkEnvironment::Test, true).expect("worx");
    let balance = w.get_wallet_balance().expect("");
    println!("balance: {:?}", balance);
    println!("address: {:?}", w.address().expect(""));
    // w.send_local("tb1qaq8de62av8xkcnwfrgjmvatsl56hmpc4q6m3uz".to_string(), 2500, pkey).expect("");
    // w.send_local("tb1q0287j37tntffkndch8fj38s2f994xk06rlr4w4".to_string(), 3500, pkey).expect("");
    // let txid = w.transaction_details.expect("d").txid.to_string();
    // println!("txid: {}", txid);
    // 2485227b319650fcd689009ca8b5fb2a02e556098f7c568e832ae72ac07ab8e8
}


// #[ignore]
#[tokio::test]
async fn balance_test2() {
    let mut w = SingleKeyBitcoinWallet
    ::new_wallet(PublicKey::from_hex_direct("028215a7bdab82791763e79148b4784cc7474f0969f23e44fea65d066602dea585").expect(""), NetworkEnvironment::Test, true).expect("worx");
    let balance = w.get_wallet_balance().expect("");



    println!("balance: {:?}", balance);
    println!("address: {:?}", w.address().expect(""));
    let txs = w.get_sourced_tx().expect("");
    for t in txs {
        println!("tx: {}", t.json_or());
    }
    let (_, kp) = dev_ci_kp().expect("");
    let dest = kp.public_key().to_bitcoin_address(&NetworkEnvironment::Dev).expect("");
    let tx = w.create_transaction(Some(kp.public_key()), None, 2200).expect("");
    let psbt = w.psbt.expect("psbt");
    let txb = psbt.clone().extract_tx();
    println!("txb: {:?}", txb);
    for o in txb.output {
        println!("o: {:?}", o);

    }


}

#[ignore]
#[tokio::test]
async fn balance_test() {
    let tc = TestConstants::new();
    let _kp = tc.key_pair();
    // let pk = kp.public_key.to_struct_public_key();
    // let balance = get_balance(pk).expect("");
    // Source address: tb1q0287j37tntffkndch8fj38s2f994xk06rlr4w4
    // Send to address: tb1q68rhft47r5jwq5832k9urtypggpvzyh5z9c9gn
    let w = SingleKeyBitcoinWallet
    ::new_wallet(tc.public.to_struct_public_key(), NetworkEnvironment::Test, true).expect("worx");
    let balance = w.get_wallet_balance().expect("");
    println!("balance: {:?}", balance);
    println!("address: {:?}", w.address().expect(""));
    // w.get_source_addresses();
    let w2 = SingleKeyBitcoinWallet
    ::new_wallet(tc.public2.to_struct_public_key(), NetworkEnvironment::Test, true).expect("worx");
    let balance = w2.get_wallet_balance().expect("");
    println!("balance2: {:?}", balance);
    println!("address2: {:?}", w2.address().expect(""));
    println!("{:?}", w2.get_sourced_tx().expect(""));


    // w.create_transaction(tc.public2.to_struct_public_key(), 3500).expect("");
    // let d = w.transaction_details.clone().expect("d");
    // println!("txid: {:?}", d.txid);
    // let signables = w.signable_hashes().expect("");
    // println!("num signable hashes: {:?}", signables.len());
    // for (i, (hash, sighashtype)) in signables.iter().enumerate() {
    //     println!("signable {}: {}", i, hex::encode(hash));
    //     let prf = Proof::from_keypair(hash, tc.key_pair());
    //     w.affix_input_signature(i, &prf, sighashtype);
    // }
    // let finalized = w.sign().expect("sign");
    // println!("finalized: {:?}", finalized);

    // w.broadcast_tx().expect("broadcast");
    // let txid = w.broadcast_tx().expect("txid");
    // println!("txid: {:?}", txid);
}

// // https://bitcoindevkit.org/blog/2021/12/first-bdk-taproot-tx-look-at-the-code-part-2/
// // https://github.com/bitcoin/bitcoin/blob/master/doc/descriptors.md
