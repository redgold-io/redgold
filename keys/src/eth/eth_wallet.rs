use bdk::bitcoin::hashes::hex::ToHex;
use ethers::addressbook::Address;
use ethers::middleware::{Middleware, SignerMiddleware};
use ethers::prelude::{Bytes, LocalWallet, maybe, Provider, Signature, Signer, to_eip155_v, TransactionRequest, U256};
use ethers::prelude::transaction::eip2718::TypedTransaction;
use ethers::providers;
use ethers::providers::Http;
use log::kv::Key;
use redgold_schema::{error_info, ErrorInfoContext, from_hex, RgResult, SafeOption, structs};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::structs::{CurrencyAmount, NetworkEnvironment, SupportedCurrency};

use crate::address_external::ToEthereumAddress;
use crate::eth::historical_client::EthHistoricalClient;
use crate::KeyPair;
use crate::util::sign;

pub struct EthWalletWrapper {
    pub wallet: LocalWallet,
    pub client: SignerMiddleware<Provider<Http>, LocalWallet>,
    pub provider: Provider<Http>,
    pub keypair: KeyPair,
    pub network: NetworkEnvironment
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
            keypair: KeyPair::from_private_hex(secret_key.clone())?,
            network: network.clone(),
        })

    }

    pub fn parse_address(a: &String) -> RgResult<Address> {
        let addr: Address = a.parse().error_info("address parse failure")?;
        Ok(addr)
    }

    pub fn address(&self) -> RgResult<structs::Address> {
        let addr = self.keypair.public_key().to_ethereum_address_typed()?;
        Ok(addr)
    }

    pub async fn send_or_form_fake(&self, to: &structs::Address, value: &CurrencyAmount, kp: &KeyPair, do_broadcast: bool) -> RgResult<String> {
        let tx = self.create_transaction_typed(&self.address()?, to, value.clone(), None).await?;
        let from_address: Address = self.address()?.render_string()?.parse().error_info("from address parse failure")?;
        let signed = self.client.sign_transaction(&tx, from_address)
            .await.error_info("Signing error")?;
        // Return the raw rlp-encoded signed transaction
        let bytes = tx.rlp_signed(&signed);
        let tx = Self::decode_rlp_tx(bytes.to_vec())?;
        if do_broadcast {
            self.broadcast_tx(bytes).await?;
        }
        Ok(tx.hash.to_string())
    }

    pub async fn send(&self, to: &structs::Address, value: &CurrencyAmount) -> RgResult<String> {
        let tx = self.create_transaction_typed(&self.address()?, to, value.clone(), None).await?;
        // send it!
        let pending_tx = self.client.send_transaction(tx, None).await.expect("works");

        // get the mined tx
        let receipt = pending_tx.await.expect("mined").expect("no error");
        let tx = self.client.get_transaction(receipt.transaction_hash).await.expect("works");
        // println!("Sent tx: {}\n", serde_json::to_string(&tx).expect("works"));
        // println!("Tx receipt: {}", serde_json::to_string(&receipt).expect("works"));
        Ok(receipt.transaction_hash.0.to_hex())

    }


    pub async fn create_transaction_typed(
        &self,
        from: &structs::Address,
        to: &structs::Address,
        value: CurrencyAmount,
        fee_gas_price: Option<CurrencyAmount>
    ) -> RgResult<TypedTransaction> {
        self.create_transaction_typed_inner(from, to, value.clone(), fee_gas_price.clone()).await
            .with_detail("from", from.render_string()?)
            .with_detail("to", to.render_string()?)
            .with_detail("value", value.clone().json_or())
            .with_detail("fee", fee_gas_price.clone().json_or())
            .with_detail("default_fee", Self::fee_fixed_normal_by_env(&self.network).json_or())
    }
    pub async fn create_transaction_typed_inner(
        &self, from: &structs::Address, to: &structs::Address, value: CurrencyAmount,
        fee_gas_price: Option<CurrencyAmount>
    ) -> RgResult<TypedTransaction> {
        if value.currency_or() != SupportedCurrency::Ethereum {
            return Err(error_info("Currency must be Ethereum"));
        }
        let fee_gas_price = fee_gas_price.unwrap_or(Self::gas_price_fixed_normal_by_env(&self.network));
        let big_value = value.bigint_amount().ok_msg("CurrencyAmount bigint amount missing")?;
        // let big_value = EthHistoricalClient::translate_value_bigint(value as i64)?;
        // U256::from_dec_str()
        let bigint_str = big_value.to_string();
        let u256 = U256::from_dec_str(&*bigint_str).error_info("U256 parse failure")?;
        let to_address: Address = to.render_string()?.parse().error_info("to address parse failure")?;
        let from_address: Address = from.render_string()?.parse().error_info("from address parse failure")?;
        let tr = TransactionRequest::new().to(to_address).value(u256);
        let mut tx: TypedTransaction = tr.into();
        let gas_price_str = fee_gas_price.string_amount.ok_msg("fee gas price missing")?;
        let gas_price_u256 = U256::from_dec_str(&*gas_price_str).error_info("U256 parse failure")?;
        tx.set_gas_price(gas_price_u256);
        // tx.gas_price()
        // tx.set_gas()

        tx.set_chain_id(self.wallet.chain_id());
        tx.set_from(from_address);

        let nonce = maybe(tx.nonce().cloned(), self.client.get_transaction_count(from_address, None)).await
            .error_info("nonce get failure")?;
        tx.set_nonce(nonce);

        self.provider.fill_transaction(&mut tx, None).await
            .error_info("tx fill failure")
            .with_detail("tx", tx.json_or())
            ?;


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

    pub async fn broadcast_tx_vec(&self, tx: Vec<u8>) -> RgResult<()> {
        self.broadcast_tx(Bytes::from(tx)).await
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

}
