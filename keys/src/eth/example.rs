use std::str::FromStr;

use ethers::{core::{types::TransactionRequest},
             middleware::SignerMiddleware, providers::{Http, Middleware, Provider}, providers, signers::{LocalWallet, Signer}};


use crate::{KeyPair};

use crate::util::mnemonic_support::WordsPass;

use alloy_chains::Chain;


use ethers::prelude::{maybe, to_eip155_v, U256};
use ethers::types::{Address, Bytes, Signature};
use ethers::types::transaction::eip2718::TypedTransaction;
use foundry_block_explorers::account::GenesisOption;
use foundry_block_explorers::Client;
use num_bigint::BigInt;
use num_traits::{FromPrimitive, ToPrimitive};
use redgold_schema::{EasyJson, error_info, ErrorInfoContext, from_hex, RgResult, SafeOption, structs};
use redgold_schema::structs::{NetworkEnvironment, SupportedCurrency};
use crate::util::btc_wallet::ExternalTimedTransaction;


//  Has faucet bitcoin test funds
pub fn dev_ci_kp() -> Option<(String, KeyPair)> {
    if let Some(w) = std::env::var("REDGOLD_TEST_WORDS").ok() {
        let w = WordsPass::new(w, None);
        // This is wrong for ethereum, but we just need the secret key to match other
        // faucet funds for 'same' key.
        let path = "m/84'/0'/0'/0/0";
        let privk = w.private_at(path.to_string()).expect("private key");
        let keypair = w.keypair_at(path.to_string()).expect("private key");
        Some((privk, keypair))
    } else {
        None
    }
}

fn eth_addr() -> String {
    "0xA729F9430fc31Cda6173A0e81B55bBC92426f759".to_string()
}


pub struct EthHistoricalClient {
    client: Client,
}

impl EthHistoricalClient {
    pub fn new(network_environment: &NetworkEnvironment) -> Option<RgResult<EthHistoricalClient>> {
        let key = std::env::var("ETHERSCAN_API_KEY").ok();
        let chain = Self::chain_id(network_environment);
        key.map(|k| {
            Client::new(chain, k)
                .error_info("Client initialization failure")
                .map(|c| EthHistoricalClient { client: c })
        })
    }

    fn chain_id(network_environment: &NetworkEnvironment) -> Chain {
        let chain = if network_environment.is_main() {
            Chain::mainnet()
        } else {
            Chain::sepolia()
        };
        chain
    }

    pub async fn get_balance(&self, address: &String) -> RgResult<BigInt> {
        let addr = address.parse().error_info("address parse failure")?;
        let metadata = self.client
            .get_ether_balance_single(&addr, None).await.error_info("balance fetch failure")?;
        let bal = metadata.balance;
        BigInt::from_str(&bal).error_info("balance parse failure")
    }
    pub fn translate_value(value: &String) -> RgResult<i64> {
        BigInt::from_str(value).error_info("value parse failure")
            .map(|v| v / Self::bigint_offset())
            .and_then(|v| v.to_i64().ok_or(error_info("BigInt translation to i64 failure")))
    }

    // Workaround for dealing with u64's etc, drop from e18 precision to e8 precision
    fn bigint_offset() -> BigInt {
        BigInt::from(10_u64.pow(10))
    }

    pub fn translate_float_value(value: &String) -> RgResult<i64> {
        let f64_val = value.parse::<f64>().error_info(format!("failed to parse string value {} as f64", value))?;
        let f64_offset = f64_val * 10_f64.powi(18);
        let bi = BigInt::from_f64(f64_offset).ok_msg("f64 to BigInt failure")?;
        let offset_bi = bi / BigInt::from(10_u64.pow(10));
        offset_bi.to_i64().ok_or(error_info("BigInt translation to i64 failure"))
    }

    pub fn translate_float_value_bigint(value: &String) -> RgResult<BigInt> {
        let f64_val = value.parse::<f64>().error_info(format!("failed to parse string value {} as f64", value))?;
        let f64_offset = f64_val * 10_f64.powi(18);
        let bi = BigInt::from_f64(f64_offset).ok_msg("f64 to BigInt failure")?;
        Ok(bi)
    }

    pub fn translate_big_int_u256(value: BigInt) -> U256 {
        let u256 = U256::from_big_endian(&*value.to_bytes_be().1);
        u256
    }

    pub fn translate_float_value_u256(value: &String) -> RgResult<U256> {
        let bi = Self::translate_float_value_bigint(&value)?;
        let u256 = Self::translate_big_int_u256(bi);
        Ok(u256)
    }


    pub fn translate_value_bigint(value: i64) -> RgResult<BigInt> {
        BigInt::from_i64(value).ok_or(error_info("BigInt int64 value parse failure"))
            .map(|v| v * BigInt::from(10_u64.pow(10)))
    }

    pub async fn get_all_tx(
        &self,
        address: &String
    ) -> RgResult<Vec<ExternalTimedTransaction>> {
        let addr = address.parse().error_info("address parse failure")?;
        let txs = self.client.get_transactions(&addr, None).await.error_info("txs fetch failure")?;
        let mut res = vec![];
        for t in txs {
            let tx_id = match t.hash {
                GenesisOption::Some(h) => {Some(hex::encode(h.0))}
                _ => {None}
            };
            let from_opt = match t.from {
                GenesisOption::Some(h) => {Some(h.to_string())}
                _ => {None}
            };
            let to_opt = t.to.map(|h| h.to_string());
            let timestamp = t.time_stamp.parse::<u64>().ok();

            let value_str = t.value.to_string();
            let amount = Self::translate_value(&value_str)?;

            if let (Some(tx_id), Some(from), Some(to)) = (tx_id, from_opt, to_opt) {
                let incoming = &to == address;
                let other_address = if incoming {
                    from
                } else {
                    to
                };
                res.push(ExternalTimedTransaction {
                    tx_id,
                    timestamp,
                    other_address,
                    other_output_addresses: vec![],
                    amount: amount as u64,
                    incoming,
                    currency: SupportedCurrency::Ethereum,
                });
            }
        }
        Ok(res)
    }

}


async fn foo() -> Result<(), Box<dyn std::error::Error>> {

    let key = std::env::var("ETHERSCAN_API_KEY").expect("api key");
    let client = Client::new(Chain::sepolia(), key).expect("works");
    // Or using environment variables
    // let client = Client::new_from_env(Chain::mainnet())?;

    let address = "0xA729F9430fc31Cda6173A0e81B55bBC92426f759".parse().expect("valid address");
    let metadata = client.get_ether_balance_single(&address, None).await.expect("works");
    // assert_eq!(metadata.items[0].contract_name, "DAO");
    println!("balance: {}", metadata.balance);

    let _txs = client.get_transactions(&address, None).await.expect("works");

    let environment = NetworkEnvironment::Dev;
    let h = EthHistoricalClient::new(&environment).expect("works").expect("works");

    let string_addr = "0xA729F9430fc31Cda6173A0e81B55bBC92426f759".to_string();
    let txs = h.get_all_tx(&string_addr).await.expect("works");

    println!("txs: {}", txs.json_or());

    let tx_head = txs.get(0).expect("tx");
    let _from = tx_head.other_address.clone();

    let (dev_secret, _dev_kp) = dev_ci_kp().expect("works");

    let _eth = EthWalletWrapper::new(&dev_secret, &environment).expect("works");

    let fee = "0.000108594791676".to_string();
    let fee_value = EthHistoricalClient::translate_value(&fee.to_string()).expect("works") as u64;
    let _amount = fee_value * 2;
    // eth.create_transaction(&from, amount).await.expect("works");

    Ok(())
}

pub struct EthWalletWrapper {
    pub wallet: LocalWallet,
    pub client: SignerMiddleware<Provider<Http>, LocalWallet>,
    pub provider: Provider<Http>,
}

impl EthWalletWrapper {

    pub fn new(secret_key: &String, network: &NetworkEnvironment) -> RgResult<EthWalletWrapper> {

        let bytes = from_hex(secret_key.clone())?;
        let w = LocalWallet::from_bytes(&bytes).error_info("wallet creation failure")?;

        let provider = if network.is_main() {
            &providers::MAINNET
        } else {
            &providers::SEPOLIA
        }.provider();
        let chain = EthHistoricalClient::chain_id(network).id();

        let wallet1 = w.with_chain_id(chain);

        let client = SignerMiddleware::new(
            provider.clone(), wallet1.clone()
        );

        Ok(Self {
            wallet: wallet1,
            client,
            provider,
        })

    }

    pub async fn send_tx(&self, to: &String, value: u64) -> RgResult<()> {
        let to_address: Address = to.parse().error_info("to address parse failure")?;
        let value = EthHistoricalClient::translate_value_bigint(value as i64)?;
        let value = EthHistoricalClient::translate_big_int_u256(value);
        let tx = TransactionRequest::new().to(to_address).value(value);

        // send it!
        let pending_tx = self.client.send_transaction(tx, None).await.expect("works");

        // get the mined tx
        let receipt = pending_tx.await.expect("mined").expect("no error");
        let tx = self.client.get_transaction(receipt.transaction_hash).await.expect("works");

        println!("Sent tx: {}\n", serde_json::to_string(&tx).expect("works"));
        println!("Tx receipt: {}", serde_json::to_string(&receipt).expect("works"));
        Ok(())

    }

    pub async fn create_transaction(&self, from: &String, to: &String, value: u64) -> RgResult<TypedTransaction> {
        let big_value = EthHistoricalClient::translate_value_bigint(value as i64)?;
        let u256 = U256::from_big_endian(&*big_value.to_bytes_be().1);
        let to_address: Address = to.parse().error_info("to address parse failure")?;
        let from_address: Address = from.parse().error_info("from address parse failure")?;
        let tr = TransactionRequest::new().to(to_address).value(u256);
        let mut tx: TypedTransaction = tr.into();

        tx.set_chain_id(self.wallet.chain_id());
        tx.set_from(from_address);

        let nonce = maybe(tx.nonce().cloned(), self.client.get_transaction_count(from_address, None)).await
            .error_info("nonce get failure")?;
        tx.set_nonce(nonce);

        self.provider.fill_transaction(&mut tx, None).await
            .error_info("tx fill failure")?;

        Ok(tx)
    }
    pub fn signing_data(tx: &TypedTransaction) -> RgResult<Vec<u8>> {
        let sig_hash = tx.sighash().0.to_vec();
        Ok(sig_hash)
    }

    pub fn process_signature(signature: structs::Signature, tx: &mut TypedTransaction) -> RgResult<Bytes> {
        let rsv = signature.rsv.ok_msg("rsv missing")?;
        let r = rsv.r.ok_msg("r missing")?.value;
        let s = rsv.s.ok_msg("s missing")?.value;
        let v = rsv.v.ok_msg("v missing")?;
        // let r_bytes = FieldBytes::from_slice(&*r);
        // let r_bytes: FieldBytes = r.into();
        // let s_bytes = FieldBytes::from_slice(&*s);
        // let s_bytes: FieldBytes = s.into();
        // let r = U256::from_big_endian(r_bytes.as_slice());
        // let r = U256::from_big_endian(r_bytes.as_slice());
        let r = U256::from_big_endian(&*r);
        let s = U256::from_big_endian(&*s);
        let v_offset = (v as u64) + 27;

        let mut sig = Signature {
            r,
            s,
            v: v_offset,
        };

        let chain_id = tx.chain_id().ok_msg("chain id missing")?.as_u64();

        // sign_hash sets `v` to recid + 27, so we need to subtract 27 before normalizing
        sig.v = to_eip155_v(sig.v as u8 - 27, chain_id);

        // ensure correct v given the chain - first extract recid
        let recid = (sig.v - 35) % 2;
        // eip155 check
        assert_eq!(sig.v, chain_id * 2 + 35 + recid);

        // since we initialize with None we need to re-set the chain_id for the sighash to be
        // correct
        let tx = tx;
        tx.set_chain_id(chain_id);
        let sighash = tx.sighash();

        let origin = tx.from().ok_msg("origin missing")?;
        sig.verify(sighash, *origin).error_info("signature verification failure")?;

        Ok(tx.rlp_signed(&sig))

    }

    pub async fn broadcast_tx(&self, tx: Bytes) -> RgResult<()> {
        let result = self.client.send_raw_transaction(tx).await;
        match result {
            Ok(_o) => {
                Ok(())
            }
            Err(_e) => {
                Err(error_info(format!("tx send failure {}", "error")))
            }
        }
    }

        /*

        let sighash = tx.sighash();
        let mut sig = self.sign_hash(sighash)?;

        // sign_hash sets `v` to recid + 27, so we need to subtract 27 before normalizing
        sig.v = to_eip155_v(sig.v as u8 - 27, chain_id);
        Ok(sig)

    /// Signs the provided hash.
    pub fn sign_hash(&self, hash: H256) -> Result<Signature, WalletError> {
        let (recoverable_sig, recovery_id) = self.signer.sign_prehash(hash.as_ref())?;

        let v = u8::from(recovery_id) as u64 + 27;

        let r_bytes: FieldBytes<Secp256k1> = recoverable_sig.r().into();
        let s_bytes: FieldBytes<Secp256k1> = recoverable_sig.s().into();
        let r = U256::from_big_endian(r_bytes.as_slice());
        let s = U256::from_big_endian(s_bytes.as_slice());

        Ok(Signature { r, s, v })
    }

         */

}

// 0xA729F9430fc31Cda6173A0e81B55bBC92426f759
// #[ignore]
#[tokio::test]
async fn main() {
    foo().await.expect("works");

    let _api_key = std::env::var("ETHERSCAN_API_KEY").expect("");

    let testc = TestConstants::new();
    let _test_skhex = testc.key_pair().secret_key.to_hex();

    let (dev_secret, dev_kp) = dev_ci_kp().expect("works");

    let bytes = hex::decode(dev_secret).expect("wtf");

    let _eth = dev_kp.public_key().to_ethereum_address().expect("works").print();

    let w = LocalWallet::from_bytes(&bytes).expect("works");
    println!("Wallet btc: {}", w.address().to_string());

    let anvil = Anvil::new().spawn();

    let wallet: LocalWallet = anvil.keys()[0].clone().into();
    let wallet2: LocalWallet = anvil.keys()[1].clone().into();

    // connect to the network
    let provider = Provider::<Http>::try_from(anvil.endpoint()).expect("works");


    // connect the wallet to the provider
    let wallet1 = wallet.with_chain_id(anvil.chain_id());
    let client = SignerMiddleware::new(
        provider, wallet1.clone()
    );


    let addr = wallet1.address();
    let hexs = hex::encode(addr.0);
    println!("Wallet 1 address: {}", hexs);

    let balance = client.get_balance(wallet1.address(), None).await.expect("works");

    println!("Wallet 1 balance: {}", balance);

    // craft the transaction
    let tx = TransactionRequest::new().to(wallet2.address()).value(10000);

    // send it!
    let pending_tx = client.send_transaction(tx, None).await.expect("works");

    // get the mined tx
    let receipt = pending_tx.await.expect("mined").expect("no error");
    let tx = client.get_transaction(receipt.transaction_hash).await.expect("works");

    println!("Sent tx: {}\n", serde_json::to_string(&tx).expect("works"));
    println!("Tx receipt: {}", serde_json::to_string(&receipt).expect("works"));


}