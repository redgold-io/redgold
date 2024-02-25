use redgold_schema::{bytes_data, constants, error_message, from_hex, SafeBytesAccess, SafeOption, structs, WithMetadataHashable};
use redgold_schema::structs::{Address, Error as RGError, ErrorInfo, Proof, State};
use crate::api::rosetta::models::{AccountIdentifier, Amount, Block, BlockIdentifier, CoinAction, CoinChange, CoinIdentifier, Currency, NetworkIdentifier, Operation, OperationIdentifier, PublicKey, Signature, Transaction, TransactionIdentifier};
use crate::core::relay::Relay;
use redgold_data::data_store::DataStore;
use crate::schema;

#[derive(Clone)]
pub struct Rosetta {
    pub(crate) relay: Relay,
}


impl Rosetta {

    pub fn redgold_currency() -> Currency {
        Currency {
            symbol: constants::SYMBOL.to_string(),
            decimals: constants::DECIMALS as u32,
            metadata: None,
        }
    }

    pub fn rosetta_version() -> String {
        "1.4.12".into()
    }

    pub fn amount(val: i64) -> Amount {
        Amount {
            value: format!("{}", val),
            currency: Rosetta::redgold_currency(),
            metadata: None,
        }
    }

    pub fn redgold_blockchain() -> String {
        constants::REDGOLD.to_string()
    }

    pub async fn validate_network(
        &self,
        network_identifier: NetworkIdentifier,
    ) -> Result<(), ErrorInfo> {
        let current_network = self.relay.node_config.network.to_std_string();
        if network_identifier.network != current_network {
            return Err(error_message(
                RGError::InvalidNetworkEnvironment,
                format!(
                    "Expected current network environment {} but got {}",
                    current_network, network_identifier.network
                ),
            ));
        }
        if network_identifier.blockchain != Self::redgold_blockchain() {
            return Err(error_message(
                RGError::InvalidNetworkEnvironment,
                format!(
                    "Expected blockchain {} but got {}",
                    constants::REDGOLD,
                    network_identifier.blockchain
                ),
            ));
        }
        if network_identifier.sub_network_identifier.is_some() {
            return Err(error_message(
                RGError::InvalidNetworkEnvironment,
                format!(
                    "Sub network supplied but sub networks not supported"
                ),
            ));
        }
        Ok(())
    }

    pub async fn validate_currency(&self, currency: &Currency) -> Result<(), ErrorInfo> {
        // TODO: Query datastore to find supported currencies
        if currency.symbol != Rosetta::redgold_currency().symbol {
            Err(error_message(
                RGError::UnsupportedCurrency,
                format!("Currency {} not supported", currency.symbol),
            ))?
        }
        Ok(())
    }

    pub async fn query_block(
        &self,
        hash: Option<String>,
        height: Option<i64>,
    ) -> Result<Option<structs::Block>, ErrorInfo> {
        let mut maybe_hash = None;
        if let Some(h) = hash {
            maybe_hash = Some(schema::decode_hex(h)?);
        };
        // TODO: Shouldn't this whole thing really check to verify the active height / active chain thing works?
        // revisit this on block removed events.
        let maybe_block = self
            .relay
            .ds
            .address_block_store
            .query_block_hash_height(maybe_hash, height)
            .await?;
        Ok(maybe_block)
    }

    pub fn block_identifier(block: &structs::Block) -> BlockIdentifier {
        BlockIdentifier {
            index: block.height,
            hash: block.hash_hex().expect("hash"),
        }
    }

    pub async fn validate_currencies(
        &self,
        opt_currencies: &Option<Vec<Currency>>,
    ) -> Result<(), ErrorInfo> {
        if let Some(currencies) = opt_currencies {
            for currency in currencies {
                self.validate_currency(currency).await?;
            }
        }
        Ok(())
    }

    pub async fn latest_block(&self) -> Result<crate::schema::structs::Block, ErrorInfo> {
        self.relay.ds.address_block_store.query_last_block().await?.ok_or(error_message(
            RGError::DataStoreInternalCorruption,
            "Missing latest block",
        ))
    }

    pub fn transaction_identifier(
        transaction: &structs::Transaction,
    ) -> Result<TransactionIdentifier, ErrorInfo> {
        Ok(TransactionIdentifier {
            hash: transaction.hash_hex()?,
        })
    }

    pub fn operation_type() -> String {
        "TRANSFER".to_string()
    }

    pub fn account_identifier(address: Address) -> Result<AccountIdentifier, ErrorInfo> {
        Ok(AccountIdentifier {
            address: address.render_string()?,
            sub_account: None,
            metadata: None,
        })
    }

    pub fn coin_identifier() -> CoinIdentifier {
        CoinIdentifier {
            identifier: "transaction_hash:index.".to_string(),
        }
    }

    pub fn operation(
        index: i64,
        state: State,
        address: Address,
        operation_amount: i64,
        coin_action: CoinAction,
    ) -> Result<Operation, ErrorInfo> {
        Ok(Operation {
            operation_identifier: OperationIdentifier {
                index,
                network_index: None,
            },
            related_operations: None,
            _type: Self::operation_type(),
            status: Some(format!("{:?}", state)),
            account: Some(Self::account_identifier(address)?),
            amount: Some(Self::amount(operation_amount)),
            coin_change: Some(CoinChange {
                coin_identifier: Self::coin_identifier(),
                coin_action,
            }),
            metadata: None,
        })
    }

    pub async fn input_output(&self, input: structs::Input) -> Result<structs::Output, ErrorInfo> {
        let output = match input.output {
            None => {
                let utxo = input.utxo_id.safe_get()?;
                let hash = utxo.transaction_hash.safe_get()?;
                let result = self.relay.ds.transaction_store
                    .query_maybe_transaction(&hash).await?;
                // TODO: Translate result to error here
                let out = result
                    .as_ref()
                    .expect("a")
                    .0
                    .outputs
                    .get(utxo.output_index as usize)
                    .expect("a");
                out.clone()
            }
            Some(o) => o,
        };
        Ok(output)
    }

    pub async fn translate_operation(
        &self,
        transaction: &structs::Transaction,
        state: State,
    ) -> Result<Vec<Operation>, ErrorInfo> {
        let mut operations = vec![];
        let mut index = 0 as i64;
        for output in &transaction.outputs {
            operations.push(Self::operation(
                index,
                state,
                output.address.as_ref().expect("address").clone(),
                output.amount() as i64,
                CoinAction::CREATED,
            )?);
            index += 1;
        }
        for input in &transaction.inputs {
            // TODO: Enrich input's output if missing
            let output = self.input_output(input.clone()).await?;
            operations.push(Self::operation(
                index,
                state,
                output.address.as_ref().expect("address").clone(),
                -1 * (output.amount() as i64),
                CoinAction::SPENT,
            )?);
            index += 1;
        }
        Ok(operations)
    }

    pub async fn translate_transaction(
        &self,
        transaction: structs::Transaction,
        state: State,
    ) -> Result<Transaction, ErrorInfo> {
        Ok(Transaction {
            transaction_identifier: Self::transaction_identifier(&transaction)?,
            operations: self.translate_operation(&transaction, state).await?,
            related_transactions: None,
            metadata: None,
        })
    }

    pub async fn translate_block(&self, block: structs::Block, state: State) -> Result<Block, ErrorInfo> {
        let parent_height = std::cmp::max(block.height - 1, block.height);
        let parent_hash = block
            .clone()
            .previous_block_hash
            .unwrap_or(block.hash_or()) // for genesis only
            .hex();

        let mut translated = vec![];
        for t in &block.transactions {
            let transaction = self.translate_transaction(t.clone(), state).await?;
            translated.push(transaction);
        }

        Ok(Block {
            block_identifier: Self::block_identifier(&block),
            parent_block_identifier: BlockIdentifier {
                index: parent_height,
                hash: parent_hash,
            },
            timestamp: block.struct_metadata.expect("struct").time.expect("time"),
            transactions: translated,
            metadata: None,
        })
    }

    fn translate_signature(signature: Signature) -> Result<structs::Signature, ErrorInfo> {
        let sig_bytes = from_hex(signature.hex_bytes)?;
        Ok(structs::Signature {
            bytes: bytes_data(sig_bytes.clone()),
            signature_type: structs::SignatureType::Ecdsa as i32,
            rsv: None,
        })
    }

    pub(crate) fn translate_public_key(public_key: PublicKey) -> Result<structs::PublicKey, ErrorInfo> {
        let pub_bytes = from_hex(public_key.hex_bytes)?;
        Ok(structs::PublicKey {
            bytes: bytes_data(pub_bytes),
            key_type: structs::PublicKeyType::Secp256k1 as i32,
        })
    }

    pub fn signature_to_proof(signature: Signature) -> Result<Proof, ErrorInfo> {
        Ok(Proof {
            signature: Some(Self::translate_signature(signature.clone())?),
            public_key: Some(Self::translate_public_key(signature.public_key)?),
        })
    }

    // todo
    pub async fn validate_type(&self, _type: String) -> Result<(), ErrorInfo> {
        Ok(())
    }

}
