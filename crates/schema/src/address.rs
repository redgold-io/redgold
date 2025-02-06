use itertools::Itertools;
use crate::structs::{Address, AddressDescriptor, AddressInfo, AddressType, ErrorInfo, Hash, OutputContract, PublicKey, SupportedCurrency, UtxoEntry, Weighting};
use crate::{bytes_data, error_info, from_hex, ErrorInfoContext, RgResult, SafeOption};
use crate::structs;
use sha3::Sha3_224;

use crate::proto_serde::ProtoSerde;
use crate::structs::SupportedCurrency::Redgold;
use sha3::Digest;

// impl fromstr for address etc. impl tostring
impl Into<Address> for structs::PublicKey {
    fn into(self) -> Address {
        Address::from_struct_public(&self).expect("some")
    }
}

impl Into<Address> for Vec<u8> {
    fn into(self) -> Address {
        Address::address_data(self).expect("some")
    }
}


impl AddressDescriptor {
    pub fn from_multisig_public_keys_and_threshold(public_keys: &Vec<PublicKey>, threshold: i64) -> AddressDescriptor {
        let mut descriptor = AddressDescriptor::default();
        descriptor.public_keys = public_keys.clone();
        let mut contract = OutputContract::default();
        contract.threshold = Some(Weighting::from_int_basis(threshold, public_keys.len() as i64));
        descriptor.contract = Some(contract);
        descriptor
    }
}


impl Address {

    pub fn from_multisig_public_keys_and_threshold(public_keys: &Vec<PublicKey>, threshold: i64) -> Address {
        let descriptor = AddressDescriptor::from_multisig_public_keys_and_threshold(public_keys, threshold);
        let bytes = descriptor.to_hashed().bytes;

        Self {
            address: bytes,
            address_type: AddressType::MultisigContract as i32,
            currency: Redgold as i32,
        }
    }

    pub fn mark_bitcoin_external(&mut self) -> &mut Self {
        self.currency = SupportedCurrency::Bitcoin as i32;
        self
    }

    pub fn mark_ethereum_external(&mut self) -> &mut Self {
        self.currency = SupportedCurrency::Ethereum as i32;
        self
    }

    pub fn mark_external(&mut self) -> &mut Self {
        match self.address_type() {
            AddressType::BitcoinExternalString => self.mark_bitcoin_external(),
            AddressType::EthereumExternalString => self.mark_ethereum_external(),
            _ => self
        }
    }

    pub fn as_external(&self) -> Address {
        let mut address = self.clone();
        address.mark_external();
        address.clone()
    }

    pub fn script_hash(input: impl AsRef<[u8]>) -> RgResult<Self> {
        let mut new = Self::from_bytes(Self::hash(input.as_ref()))?;
        new.address_type = AddressType::ScriptHash as i32;
        Ok(new)
    }

    pub fn from_bitcoin_external(address: &String) -> Address {
        let mut ret = Self::from_bitcoin(address);
        ret.currency = SupportedCurrency::Bitcoin as i32;
        ret
    }

        // Maybe consider checking here to see if the address is valid?
    // Or before this actually, we should potentially not do 'into bytes'
    // but instead change this to a vec and decode it.
    pub fn from_bitcoin(address: &String) -> Address {
        Self {
            address: bytes_data(address.clone().into_bytes()),
            address_type: AddressType::BitcoinExternalString as i32,
            // currency: SupportedCurrency::Bitcoin as i32,
            currency: Redgold as i32,
        }
    }
    pub fn from_eth_direct(address: impl Into<String>) -> Address {
        let address = address.into();
        Self {
            address: bytes_data(address.clone().into_bytes()),
            address_type: AddressType::EthereumExternalString as i32,
            currency: Redgold as i32,
        }
    }

    pub fn from_monero(address: &String) -> Address {
        Self {
            address: bytes_data(address.clone().into_bytes()),
            address_type: AddressType::MoneroExternalString as i32,
            currency: Redgold as i32,
        }
    }
    pub fn from_solana(address: &String) -> Address {
        Self {
            address: bytes_data(address.clone().into_bytes()),
            address_type: AddressType::SolanaExternalString as i32,
            currency: Redgold as i32,
        }
    }

    pub fn from_type(address: &String, t: AddressType) -> Address {
        Self {
            address: bytes_data(address.clone().into_bytes()),
            address_type: t as i32,
            currency: Redgold as i32,
        }
    }

    pub fn from_monero_external(address: &String) -> Address {
        let mut ret = Self::from_monero(address);
        ret.currency = SupportedCurrency::Monero as i32;
        ret
    }

    pub fn from_solana_external(address: &String) -> Address {
        let mut ret = Self::from_monero(address);
        ret.currency = SupportedCurrency::Solana as i32;
        ret
    }

    pub fn from_eth_external_exact(address: impl Into<String>) -> Address {
        let address = address.into();
        let mut ret = Self::from_eth_direct(address);
        ret.currency = SupportedCurrency::Ethereum as i32;
        ret
    }

    pub fn is_bitcoin(&self) -> bool {
        self.address_type == AddressType::BitcoinExternalString as i32
    }

    pub fn address_typed(&self) -> RgResult<AddressType> {
        AddressType::from_i32(self.address_type).ok_msg("Invalid address type")
    }

    pub fn raw_bytes(&self) -> RgResult<Vec<u8>> {
        Ok(self.address.safe_get()?.value.clone())
    }

    pub fn external_string_address(&self) -> RgResult<String> {
        String::from_utf8(self.raw_bytes()?)
            .error_info("Unable to convert bitcoin address bytes to utf8 string")
    }

    pub fn render_string(&self) -> Result<String, ErrorInfo> {

        let address_string = match self.address_typed()? {
            AddressType::BitcoinExternalString => {
                self.external_string_address()?
            }
            AddressType::EthereumExternalString => {
                self.external_string_address()?
            }
            AddressType::MoneroExternalString => {
                self.external_string_address()?
            }
            AddressType::SolanaExternalString => {
                self.external_string_address()?
            }
            _ => {
                self.hex()
            }
        };
        Ok(address_string)
    }
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Address, ErrorInfo> {
        let addr = Self::new_raw(bytes);
        addr.verify_checksum()?;
        Ok(addr)
    }

    pub fn from_byte_calculate(vec: &Vec<u8>) -> Result<Address, ErrorInfo> {
        Self::from_bytes(Self::hash(&vec))
    }


    pub fn from_struct_public(pk: &structs::PublicKey) -> Result<Address, ErrorInfo> {
        Self::from_byte_calculate(&pk.vec())
    }

    pub fn with_checksum(bytes: Vec<u8>) -> Vec<u8> {
        let checksum_bytes = Hash::digest(bytes.clone()).raw_bytes().unwrap();
        let mut res: Vec<u8> = Vec::new();
        res.extend_from_slice(&bytes);
        res.extend_from_slice(&checksum_bytes[0..4]);
        res
    }

    fn hash(buf: &[u8]) -> Vec<u8> {
        let bytes = Sha3_224::digest(buf).to_vec();
        Self::with_checksum(bytes)
    }

    pub fn verify_length(&self) -> Result<(), ErrorInfo> {
        // let i = self.address.safe_bytes()?.len();
        // if i != 32 {
        //     Err(error_info(format!("Invalid address length: {:?}", i)))?;
        // }
        Ok(())
    }

    pub fn verify_checksum(&self) -> Result<(), ErrorInfo> {
        self.verify_length()?;
        let bytes = self.raw_bytes()?;
        if Self::with_checksum(bytes[0..28].to_vec()) != bytes {
            Err(error_info("Invalid address checksum bytes"))?;
        }
        Ok(())
    }

    pub fn validated(self) -> RgResult<Self> {
        self.verify_length()?;
        self.verify_checksum()?;
        Ok(self)
    }

    pub fn str_to_address(s: String) -> Vec<u8> {
        hex::decode(s).expect("hex")
        // return base58::from_check(&s[3..]).unwrap();
    }

    pub fn address_to_str(a: &Vec<u8>) -> String {
        // let mut b = base58::check_encode_slice(&*a);
        // b.insert_str(0, "rg1");
        // return b;
        hex::encode(a)
    }

    pub fn address_data(address: Vec<u8>) -> Option<Address> {
        Some(Self::new_raw(address))
    }

    pub fn new_raw(address: Vec<u8>) -> Address {
        Address {
            address: bytes_data(address),
            address_type: AddressType::Sha3224ChecksumPublic as i32,
            currency: SupportedCurrency::Redgold as i32,
        }
    }

    pub fn raw_from_hex(hex: impl Into<String>) -> RgResult<Address> {
        Self::new_raw(from_hex(hex.into())?).validated()
    }

    pub fn currency_or(&self) -> SupportedCurrency {
        self.currency()
    }
}



impl AddressInfo {
    pub fn from_utxo_entries(address: Address, entries: Vec<UtxoEntry>) -> Self {
        let mut bal: i64 = 0;
        for r in &entries {
            if let Some(o) = &r.output {
                if let Some(d) = &o.data {
                    if let Some(a) = &d.amount {
                        bal += a.amount;
                    }
                }
            }
        }
        AddressInfo {
            address: Some(address.clone()),
            utxo_entries: entries,
            balance: bal,
            recent_transactions: vec![],
            balances: vec![]
        }
    }
}