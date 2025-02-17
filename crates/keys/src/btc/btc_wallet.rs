use std::io::Read;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use crate::btc::threshold_multiparty;
use crate::btc::threshold_multiparty::MultipartySigner;
use crate::proof_support::ProofSupport;
use crate::util::mnemonic_support::MnemonicSupport;
use bdk::bitcoin::hashes::Hash;
use bdk::bitcoin::util::psbt::PartiallySignedTransaction;
use bdk::bitcoin::{Address, EcdsaSighashType, Network, PrivateKey};
use bdk::blockchain::{Blockchain, ElectrumBlockchain, GetBlockHash, GetTx};
use bdk::database::{BatchDatabase, MemoryDatabase};
use bdk::electrum_client::{Client, Config, ElectrumApi};
use bdk::signer::{SignerContext, SignerOrdering, SignerWrapper};
use bdk::sled::Tree;
use bdk::{sled, BlockTime, FeeRate, KeychainKind, SignOptions, SyncOptions, TransactionDetails, Wallet};
use itertools::Itertools;
// use crate::util::cli::commands::send;
use redgold_schema::helpers::easy_json::EasyJsonDeser;
use redgold_schema::keys::words_pass::WordsPass;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{ErrorInfo, NetworkEnvironment, Proof, PublicKey};
// use crate::util::cli::commands::send;
use redgold_schema::{error_info, structs, ErrorInfoContext, RgResult, SafeOption};
use hex;
use redgold_schema::tx::external_tx::ExternalTimedTransaction;

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

pub struct SingleKeyBitcoinWallet<D: BatchDatabase> {
    pub(crate) wallet: Wallet<D>,
    pub public_key: structs::PublicKey,
    pub(crate) network: Network,
    pub network_environment: NetworkEnvironment,
    pub psbt: Option<PartiallySignedTransaction>,
    pub transaction_details: Option<TransactionDetails>,
    pub(crate) client: ElectrumBlockchain,
    custom_signer: Arc<MultipartySigner>,
    pub(crate) sat_per_vbyte: f32,
    pub doing_multisig: bool,
    pub address: String
}


pub fn electrum_mainnet_backends() -> Vec<String> {
    vec![
        "ssl://fulcrum.sethforprivacy.com:50002",
        "ssl://electrum.blockstream.info:50002"
    ].iter().map(|x| x.to_string()).collect_vec()
}

pub fn electrum_testnet_backends() -> Vec<String> {
    vec![
        "ssl://electrum.blockstream.info:60002"
    ].iter().map(|x| x.to_string()).collect_vec()
}


pub fn network_to_bdk_network(network: &NetworkEnvironment) -> Network {
    if *network == NetworkEnvironment::Main {
        Network::Bitcoin
    } else {
        Network::Testnet
    }
}

pub fn network_to_backends(network: &NetworkEnvironment) -> Vec<String> {
    if *network == NetworkEnvironment::Main {
        electrum_mainnet_backends()
    } else {
        electrum_testnet_backends()
    }
}

impl SingleKeyBitcoinWallet<MemoryDatabase> {

    pub fn new_wallet(
        public_key: PublicKey,
        network_environment: NetworkEnvironment,
        do_sync: bool
    ) -> Result<Self, ErrorInfo> {

        let backend = network_to_backends(&network_environment).get(0).unwrap().clone();
        let network = network_to_bdk_network(&network_environment);
        let client = Client::new(&*backend)
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
            network_environment,
            psbt: None,
            transaction_details: None,
            client,
            custom_signer: custom_signer.clone(),
            sat_per_vbyte: 4.0,
            doing_multisig: false,
            address: "".to_string(),
        };
        bitcoin_wallet.address = bitcoin_wallet.address().unwrap();
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

pub fn get_all_tx_electrum(network: &NetworkEnvironment, address: &structs::Address) -> RgResult<Vec<ExternalTimedTransaction>> {
    let addr = address.render_string()?;
    let btc_addr = Address::from_str(&*addr).error_info("Unable to parse address")?;
    let backend = network_to_backends(network).get(0).unwrap().clone();
    let client = Client::new(&*backend)
        .error_info("Error building bdk client")?;
    let client = ElectrumBlockchain::from(client);
    // Get the script_pubkey directly from the address
    let script = btc_addr.script_pubkey();
    // Call script_get_history with the script
    let mut res = vec![];
    for entry in client.script_get_history(&script).error_info("Error getting history")? {
        let hash = entry.tx_hash;
        let tx = client.get_tx(&hash).error_info("Error getting tx")?;
        let tx = tx.ok_msg("No tx found").with_detail("txid", hash.to_string())?;
        let fee = entry.fee;
        let height = entry.height;
        let header = client.block_header(height as usize).error_info("Error getting block header")?;
        let timestamp = header.time;
        let det = TransactionDetails {
            transaction: Some(tx.clone()),
            txid: tx.txid(),
            received: 0,
            sent: 0,
            fee,
            confirmation_time: Some(BlockTime{
                height: height as u32,
                timestamp: timestamp as u64,
            }),
        };

        let ret = SingleKeyBitcoinWallet::<MemoryDatabase>::extract_ett_static(
            &det,
            address,
            &client,
            network
        )?.ok_msg("No transaction found")?;
        res.push(ret);
    }
    Ok(res)
}

impl SingleKeyBitcoinWallet<Tree> {

    pub fn new_wallet_db_backed(
        public_key: PublicKey,
        network_environment: NetworkEnvironment,
        do_sync: bool,
        database_path: PathBuf,
        electrum_mn_backend: Option<String>,
        opt_private_key_hex: Option<String>,
        peer_multisig_pks: Option<Vec<structs::PublicKey>>,
        threshold: Option<i64>,
        address_only_descriptor: Option<structs::Address>
    ) -> Result<Self, ErrorInfo> {
        let backend = electrum_mn_backend.unwrap_or_else(|| network_to_backends(&network_environment).get(0).unwrap().clone());
        let network = network_to_bdk_network(&network_environment);
        let mut config = Config::builder().validate_domain(false).build();
        let client = Client::from_config(&*backend, config)
            .error_info("Error building bdk client")?;
        let client = ElectrumBlockchain::from(client);

        // KeyValueDatabase
        // Create a database (using default sled type) to store wallet data
        let mut database = sled::open(database_path.clone()).error_info("Sled database open error");
        if database.is_err() {
            if database_path.exists() {
                std::fs::remove_dir_all(&database_path)
                    .error_info("Failed to remove old sled directory")
                    .with_detail("database_path", database_path.to_str().unwrap().to_string())?;
            }
            std::fs::create_dir_all(&database_path)
                .error_info("Failed to create new sled directory")
                .with_detail("database_path", database_path.to_str().unwrap().to_string())?;

            database = sled::open(database_path.clone())
                .error_info("Sled database open error")
                .with_detail("database_path", database_path.to_str().unwrap().to_string());
        }
        let wallet_name = public_key.hex();
        let database = database?.open_tree(wallet_name.clone()).error_info("Database open tree error")?;
        // let database = MemoryDatabase::default();
        let hex = public_key.to_hex_direct_ecdsa()?;

        let doing_multisig = opt_private_key_hex.is_some() && peer_multisig_pks.is_some() && threshold.is_some();
        let descr = if doing_multisig {
            Self::multisig_descriptor_create(
                &public_key,
                &peer_multisig_pks.unwrap(),
                threshold.unwrap(),
            )?
        } else if let Some(addr) = address_only_descriptor.as_ref() {
            let addr = addr.render_string()?;
            format!("addr({})", addr)
        } else {
            format!("wpkh({})", hex)
        };

        let change_descriptor = if doing_multisig {
            Some(&*descr)
        } else {
            None
        };
        let wallet = Wallet::new(
            &*descr,
            change_descriptor,
            network,
            database
        ).error_info("Error creating BDK wallet")?;
        let custom_signer = Arc::new(MultipartySigner::new(public_key.clone()));
        let mut bitcoin_wallet = Self {
            wallet,
            public_key,
            network,
            network_environment,
            psbt: None,
            transaction_details: None,
            client,
            custom_signer: custom_signer.clone(),
            sat_per_vbyte: 4.0,
            doing_multisig,
            address: "".to_string(),
        };

        if let Some(a) = address_only_descriptor.as_ref() {
            bitcoin_wallet.address = a.render_string()?;
        } else {
            bitcoin_wallet.address = bitcoin_wallet.get_descriptor_address().unwrap();
        }

        if !doing_multisig {
            // Adding the multiparty signer to the BDK wallet
            bitcoin_wallet.wallet.add_signer(
                KeychainKind::External,
                SignerOrdering(200),
                custom_signer.clone(),
            );
        } else {
            // Convert hex to private key bytes first
            let priv_key_bytes = hex::decode(opt_private_key_hex.unwrap())
                .error_info("Invalid private key hex")?;
            let priv_key = PrivateKey::from_slice(&priv_key_bytes, network)
                .error_info("Invalid private key bytes")?;
            
            bitcoin_wallet.wallet.add_signer(
                KeychainKind::External,
                SignerOrdering(100),
                Arc::new(SignerWrapper::new(priv_key, SignerContext::Segwitv0))
            );
        }

        if do_sync {
            bitcoin_wallet.sync()?;
        }
        Ok(bitcoin_wallet)
    }
}
impl<D: BatchDatabase> SingleKeyBitcoinWallet<D> {


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
            .fee_rate(FeeRate::from_sat_per_vb(self.sat_per_vbyte));

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
            .fee_rate(FeeRate::from_sat_per_vb(self.sat_per_vbyte));

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
            let (hash, sighash) = threshold_multiparty::segwit_sighash(&psbt, input_index, ())
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
    pub fn broadcast_tx_static(psbt: String, network: &NetworkEnvironment) -> RgResult<String> {
        let psbt = psbt.json_from::<PartiallySignedTransaction>()?;
        let key = WordsPass::test_words().default_public_key()?;
        let mut w = SingleKeyBitcoinWallet::new_wallet(key, network.clone(), false)?;
        w.psbt = Some(psbt.clone());
        w.broadcast_tx()?;
        Ok(psbt.extract_tx().txid().to_string())
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



    pub fn convert_psbt_outputs(&self) -> Vec<(String, u64)> {
        let tx = self.psbt.clone().expect("psbt").extract_tx();
        let outputs = self.outputs_convert(&tx.output);
        outputs
    }

}
