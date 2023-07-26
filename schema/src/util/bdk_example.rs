use std::ops::Mul;
use std::str::FromStr;
use std::sync::{Arc, Mutex, RwLock};
use bdk::blockchain::{noop_progress, ElectrumBlockchain, Blockchain, GetTx};
use bdk::database::MemoryDatabase;
use bdk::{Balance, FeeRate, KeychainKind, SignOptions, SyncOptions, TransactionDetails, TxBuilder, Wallet};
use bdk::bitcoin::{Address, ecdsa, EcdsaSighashType, Network, Script, Sighash, Txid};
use bdk::bitcoin::blockdata::opcodes;
use bdk::bitcoin::hashes::Hash;
use bdk::bitcoin::secp256k1::{All, Secp256k1, Signature};
use bdk::bitcoin::util::{bip143, psbt, sighash};
use bdk::bitcoin::util::psbt::PartiallySignedTransaction;

use bdk::electrum_client::Client;
use bdk::signer::{InputSigner, SignerCommon, SignerContext, SignerError, SignerId, SignerOrdering, TransactionSigner};
use bdk::wallet::{AddressIndex, AddressInfo};
use bdk::wallet::coin_selection::DefaultCoinSelectionAlgorithm;
use bdk::wallet::tx_builder::CreateTx;
use bdk::wallet::AddressIndex::New;
use bitcoin::AddressType::P2wpkh;

use bitcoin::consensus::serialize;
use miniscript::{Descriptor, Legacy, Segwitv0};
// use crate::util::cli::commands::send;
use crate::{error_info, ErrorInfoContext, KeyPair, RgResult, SafeBytesAccess, SafeOption, structs, TestConstants};
use crate::public_key::ToPublicKey;
use crate::structs::{ErrorInfo, NetworkEnvironment, Proof};
use crate::util::keys::ToPublicKeyFromLib;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Read;

#[test]
fn schnorr_test() {
    let tc = TestConstants::new();
    let kp = tc.key_pair();

}

pub fn struct_public_to_address(pk: structs::PublicKey, network: Network) -> Result<Address, ErrorInfo> {
    let pk2 = bdk::bitcoin::util::key::PublicKey::from_slice(&*pk.bytes.safe_bytes()?)
        .error_info("Unable to convert destination pk to bdk public key")?;
    let addr = Address::p2wpkh(&pk2, network)
        .error_info("Unable to convert destination pk to bdk address")?;
    Ok(addr)
}

pub fn struct_public_to_bdk_pubkey(pk: &structs::PublicKey) -> Result<bdk::bitcoin::util::key::PublicKey, ErrorInfo> {
    let pk2 = bdk::bitcoin::util::key::PublicKey::from_slice(&*pk.bytes.safe_bytes()?)
        .error_info("Unable to convert destination pk to bdk public key")?;
    Ok(pk2)
}


use bdk::bitcoin::blockdata::script::Builder as ScriptBuilder;
use bdk::signer::SignerContext::{Segwitv0 as Segwitv0Context};
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
        let sig = Signature::from_compact(&*signature.bytes.safe_bytes()?).error_msg(
            structs::Error::IncorrectSignature,
            "Decoded signature construction failure",
        )?;

        let final_signature = ecdsa::EcdsaSig { sig, hash_ty };

        let public_key = proof.public_key.safe_get_msg("Missing public key")?.bytes.safe_bytes()?;
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


pub struct SingleKeyBitcoinWallet {
    wallet: Wallet<MemoryDatabase>,
    public_key: structs::PublicKey,
    network: Network,
    pub psbt: Option<PartiallySignedTransaction>,
    pub transaction_details: Option<TransactionDetails>,
    client: ElectrumBlockchain,
    custom_signer: Arc<MultipartySigner>
}

#[derive(Clone, Debug)]
pub struct ExternalTimedTransaction {
    pub tx_id: String,
    pub timestamp: u64,
    pub other_address: String,
    pub amount: u64,
    pub incoming: bool
}


impl SingleKeyBitcoinWallet {

    pub fn new_wallet(
        public_key: structs::PublicKey,
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
        let hex = public_key.hex_or();
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

    pub fn sync(&self) -> Result<(), ErrorInfo> {
        self.wallet.sync(&self.client, SyncOptions::default()).error_info("Error syncing BDK wallet")?;
        Ok(())
    }

    pub fn address(&self) -> Result<String, ErrorInfo> {
        let pk2 = bdk::bitcoin::util::key::PublicKey::from_slice(&*self.public_key.bytes.safe_bytes()?)
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
            for o in &tx.output {
                if let Some(a) = Address::from_script(&o.script_pubkey, self.network).ok() {
                    if a.to_string() == self_addr {
                        // sum value here instead?
                        to_self_output_amount = Some(o.value)
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
                    timestamp: c.timestamp,
                    other_address: a,
                    amount: value,
                    incoming: true,
                };
                res.push(ett)
            }
        }
        Ok(res)
    }

    pub fn get_wallet_balance(&self
    ) -> Result<Balance, ErrorInfo> {
        let balance = self.wallet.get_balance().error_info("Error getting BDK wallet balance")?;
        Ok(balance)
    }

    pub fn create_transaction(&mut self, destination: Option<structs::PublicKey>, destination_str: Option<String>, amount: u64) -> Result<(), ErrorInfo> {

        let addr = if let Some(destination) = destination {
            let pk2 = bdk::bitcoin::util::key::PublicKey::from_slice(&*destination.bytes.safe_bytes()?)
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

        let (mut psbt, details) = builder
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

    pub fn txid(&self) -> Result<String, ErrorInfo> {
        let txid = self.transaction_details.safe_get_msg("No psbt found")?.txid;
        Ok(txid.to_string())
    }

    pub fn signable_hashes(&mut self) -> Result<Vec<(Vec<u8>, EcdsaSighashType)>, ErrorInfo> {
        let psbt = self.psbt.safe_get_msg("No psbt found")?.clone();
        let mut res = vec![];
        for (input_index, input) in psbt.inputs.iter().enumerate() {
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
                .map_err(|e| self.custom_signer.err.read().unwrap().clone().unwrap().clone())
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
        let transaction = psbt.clone().extract_tx();
        let transaction_details = self.transaction_details.safe_get()?;
        // psbt.extract_tx()
        // psbt.clone().extract_tx().verify_with_flags()
        Ok(())
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

// #[ignore]
#[tokio::test]
async fn balance_test() {
    let tc = TestConstants::new();
    let kp = tc.key_pair();
    // let pk = kp.public_key.to_struct_public_key();
    // let balance = get_balance(pk).expect("");
    // Source address: tb1q0287j37tntffkndch8fj38s2f994xk06rlr4w4
    // Send to address: tb1q68rhft47r5jwq5832k9urtypggpvzyh5z9c9gn
    let mut w = SingleKeyBitcoinWallet
    ::new_wallet(tc.public.to_struct_public_key(), NetworkEnvironment::Test, true).expect("worx");
    let balance = w.get_wallet_balance().expect("");
    println!("balance: {:?}", balance);
    println!("address: {:?}", w.address().expect(""));
    // w.get_source_addresses();
    let mut w2 = SingleKeyBitcoinWallet
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
