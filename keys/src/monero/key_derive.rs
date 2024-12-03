use monero::{Address, KeyPair, Network, PrivateKey, PublicKey};
use tiny_keccak::{Hasher, Keccak};
use redgold_schema::{structs, ErrorInfoContext, RgResult};
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::structs::NetworkEnvironment;
use crate::util::mnemonic_support::WordsPass;

pub trait MoneroSeedBytes {
    fn monero_seed_bytes(&self) -> RgResult<[u8; 32]>;
    fn derive_monero_keys(&self) -> RgResult<KeyPair>;
    fn derive_monero_address(&self, network_environment: &NetworkEnvironment) -> RgResult<Address>;
    fn derive_monero_public_keys(&self) -> RgResult<structs::PublicKey>;
    fn monero_external_address(&self, net: &NetworkEnvironment) -> RgResult<structs::Address>;
}

impl MoneroSeedBytes for WordsPass {
    fn monero_seed_bytes(&self) -> RgResult<[u8; 32]> {
        self.derive_seed_at_path("m/44'/501'/0'/0'")
    }
    fn derive_monero_keys(&self) -> RgResult<KeyPair> {
        let seed = self.monero_seed_bytes()?;

        // Create private spend key by hashing seed
        let mut keccak = Keccak::v256();
        let mut spend_key = [0u8; 32];
        keccak.update(&seed);
        keccak.finalize(&mut spend_key);

        // Create private view key by hashing spend key
        let mut keccak = Keccak::v256();
        let mut view_key = [0u8; 32];
        keccak.update(&spend_key);
        keccak.finalize(&mut view_key);

        // Convert to Monero private keys
        let spend_key = PrivateKey::from_slice(&spend_key)
            .error_info("Invalid spend key")?;
        let view_key = PrivateKey::from_slice(&view_key)
            .error_info("Invalid view key")?;

        let kp = KeyPair {
            view: view_key,
            spend: spend_key,
        };
        Ok(kp)
    }
    fn derive_monero_address(&self, network_environment: &NetworkEnvironment) -> RgResult<Address> {
        let keypair = self.derive_monero_keys()?;

        // Get public keys from the KeyPair
        let public_spend = PublicKey::from_private_key(&keypair.spend);
        let public_view = PublicKey::from_private_key(&keypair.view);

        let net = if network_environment == &NetworkEnvironment::Main {
            Network::Mainnet
        } else {
            Network::Testnet
        };
        // Create the address (mainnet)
        let address = Address::standard(net, public_spend, public_view);
        Ok(address)
    }

    fn derive_monero_public_keys(&self) -> RgResult<structs::PublicKey> {
        let keypair = self.derive_monero_keys()?;
        let public_spend = PublicKey::from_private_key(&keypair.spend);
        let public_view = PublicKey::from_private_key(&keypair.view);
        let pks =
            structs::PublicKey::from_bytes_direct_ed25519_aux(
                public_spend.to_bytes().to_vec(), public_view.to_bytes().to_vec()
            );
        Ok(pks)
    }
    fn monero_external_address(&self, net: &NetworkEnvironment) -> RgResult<structs::Address> {
        let result = self.derive_monero_address(net)?.to_string();
        Ok(structs::Address::from_monero(&result))
    }
}