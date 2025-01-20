use std::str::FromStr;
use ed25519_dalek::{SigningKey, VerifyingKey};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::system_instruction;
use solana_sdk::transaction::Transaction;
use redgold_schema::{structs, ErrorInfoContext, RgResult, SafeOption};
use redgold_schema::structs::{CurrencyAmount, NetworkEnvironment, SupportedCurrency};
use redgold_schema::util::lang_util::AnyPrinter;
use crate::solana::derive_solana::{get_solana_address, get_solana_address_from_verifying, get_solana_public_key, SolanaWordPassExt, ToSolanaAddress};
use crate::TestConstants;
use redgold_schema::keys::words_pass::WordsPass;

#[derive(Clone)]
pub struct SolanaNetwork {
    pub net: NetworkEnvironment,
    pub words: Option<WordsPass>
}

impl SolanaNetwork {

    pub fn network_rpc_url(&self) -> String {
        match self.net {
            NetworkEnvironment::Main => "https://api.mainnet.solana.com".to_string(),
            _ => "https://api.devnet.solana.com".to_string(),
        }
    }

    pub async fn rpc_confirmed(&self) -> RpcClient {
        let rpc_url = self.network_rpc_url();
        RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed(),
        )
    }

    pub async fn send(
        &self,
        destination: structs::Address,
        amount: CurrencyAmount,
        from: Option<structs::Address>,
        kp: Option<(SigningKey, VerifyingKey)>,
    ) -> RgResult<Transaction> {
        let (signing, verifying) = match kp {
            Some((s, v)) => (s, v),
            None => self.words.clone().ok_msg("Missing wallet keys")?.derive_solana_keys()?
        };
        let destination = destination.render_string()?;
        let from = match from {
            Some(f) => f.render_string()?,
            None => {
                get_solana_address_from_verifying(&verifying)
            }
        };


        let rpc_url = self.network_rpc_url();
        let client = RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed(),
        );

        // Convert destination string to Pubkey
        let destination_pubkey = Pubkey::from_str(&destination)
            .error_info(format!("Invalid destination address: {}", destination))?;

        let from_pubkey = Pubkey::from_str(&*from)
            .error_info("Failed to convert public key")?;
        // Create transfer instruction (amount in lamports)

        let instruction = system_instruction::transfer(
            &from_pubkey,
            &destination_pubkey,
            amount.amount as u64
        );

        let message = Message::new(
            &[instruction],
            Some(&from_pubkey),
        );

        let blockhash = client.get_latest_blockhash()
            .await.error_info("Failed to get latest blockhash")?;


        // Then modify the send function where we create the transaction:
        let solana_kp = Self::convert_to_solana_keypair(&signing)?;

        let transaction = Transaction::new(
            &[&solana_kp],
            message,
            blockhash,
        );


        // Send and confirm transaction
        let signature = client.send_and_confirm_transaction(&transaction)
            .await.error_info("Failed to send and confirm transaction")?;
        // return txid

        Ok(transaction)
    }

    fn convert_to_solana_keypair(signing: &SigningKey) -> RgResult<Keypair> {
        let secret = signing.to_bytes();
        let verifying = signing.verifying_key();
        let public = verifying.to_bytes();

        // Combine private and public key bytes
        let mut keypair_bytes = [0u8; 64];
        keypair_bytes[..32].copy_from_slice(&secret);
        keypair_bytes[32..].copy_from_slice(&public);

        Keypair::from_bytes(&keypair_bytes)
            .error_info("Failed to convert to Solana keypair")
    }

    pub fn get_pubkey(address: structs::Address) -> RgResult<Pubkey> {
        let address = address.render_string()?;
        let pubkey = Pubkey::from_str(&address)
            .error_info("Failed to convert public key")?;
        Ok(pubkey)
    }

    pub fn keys(&self) -> RgResult<(SigningKey, VerifyingKey)> {
        self.words.clone().ok_msg("Missing wallet keys")?.derive_solana_keys()
    }

    pub fn self_pubkey(&self) -> RgResult<Pubkey> {
        let (_, verifying) = self.words.clone().ok_msg("Missing wallet keys")?.derive_solana_keys()?;
        let from = get_solana_address_from_verifying(&verifying);
        let from_pubkey = Pubkey::from_str(&*from)
            .error_info("Failed to convert public key")?;
        Ok(from_pubkey)
    }

    pub fn self_address(&self) -> RgResult<structs::Address> {
        let string = get_solana_address(self.self_pubkey()?.to_bytes().to_vec());
        let a = structs::Address::from_monero_external(&string);
        Ok(a)
    }

    pub async fn get_self_balance(&self) -> RgResult<CurrencyAmount> {
        let from_pubkey = self.self_pubkey()?;
        let client = self.rpc_confirmed().await;
        let b = client.get_balance(&from_pubkey)
            .await.error_info("Failed to get balance")?;
        Ok(CurrencyAmount::from_currency(b as i64, SupportedCurrency::Solana))
    }
    pub async fn get_balance(&self, addr: structs::Address) -> RgResult<CurrencyAmount> {
        // let addr = addr.render_string()?;
        let client = self.rpc_confirmed().await;

        let b = client.get_balance(&Self::get_pubkey(addr)?)
            .await.error_info("Failed to get balance")?;
        Ok(CurrencyAmount::from_currency(b as i64, SupportedCurrency::Solana))
    }

    // Add constructor
    pub fn new(net: NetworkEnvironment, words: Option<WordsPass>) -> Self {
        Self { net, words }
    }
}


#[ignore]
#[tokio::test]
async fn debug_kg() {
    let tc = TestConstants::new();
    let wp = tc.words_pass;
    let ci = TestConstants::test_words_pass().unwrap();
    let amount = 1_000_000; // 0.001 SOL
    let amount = CurrencyAmount::from_currency(amount, SupportedCurrency::Solana);
    let amount = CurrencyAmount::from_fractional_cur(0.95, SupportedCurrency::Solana).unwrap();

    let w = SolanaNetwork::new(NetworkEnvironment::Dev, Some(wp));
    let w2 = SolanaNetwork::new(NetworkEnvironment::Dev, Some(ci));
    println!("Wallet 1 address: {}", w.self_address().unwrap().render_string().unwrap());
    println!("Wallet 1 balance: {}", w.get_self_balance().await.unwrap().to_fractional());
    println!("Wallet 2 address: {}", w2.self_address().unwrap().render_string().unwrap());
    println!("Wallet 2 balance: {}", w2.get_self_balance().await.unwrap().to_fractional());

    // w.send(w2.self_address().unwrap(), amount, None, None).await.unwrap();

}