use std::collections::HashMap;
use warp::Rejection;
use crate::api::rosetta::models::*;
use crate::api::rosetta::spec::Rosetta;

use redgold_schema::{constants, ProtoSerde, SafeOption, struct_metadata_new};

use crate::{schema, util};
use crate::core::relay::Relay;
use redgold_data::data_store::DataStore;
use crate::schema::{bytes_data, error_message};
use crate::schema::{
    from_hex, i64_from_string, ProtoHashable, SafeBytesAccess, WithMetadataHashable,
};
use crate::schema::structs;
use crate::schema::structs::{
    Address, Error as RGError, Input, Output, Proof, State, StructMetadata,
    SubmitTransactionRequest, UtxoEntry,
};
use crate::schema::structs::{ErrorInfo, Hash};
use crate::schema::transaction::amount_data;
use redgold_schema::util::lang_util::SameResult;


// TODO: What is a better way to handle this?
pub async fn account_balance_wrap(r: Rosetta, request: AccountBalanceRequest,) -> Result<Result<AccountBalanceResponse, ErrorInfo>, Rejection> {
    Ok(account_balance(r, request).await)
}

pub async fn account_balance(r: Rosetta, request: AccountBalanceRequest,) -> Result<AccountBalanceResponse, ErrorInfo> {
    r.validate_network(request.network_identifier).await?;
    r.validate_currencies(&request.currencies).await?;
    // TODO: validate currencies

    let mut maybe_block = None;
    if let Some(partial_block_identifier) = request.block_identifier {
        maybe_block = r
            .query_block(
                partial_block_identifier.hash,
                partial_block_identifier.index,
            )
            .await?;
    };
    let address = Address::parse(request.account_identifier.address)?;

    let block: structs::Block = match maybe_block {
        None => r.latest_block().await?,
        Some(h) => h,
    };

    let latest_balance = r
        .relay
        .ds
        .address_block_store
        .query_address_balance_by_height(address, block.height)
        .await?;
    let bal = match latest_balance {
        None => Err(error_message(
            RGError::AddressNotFound,
            "No data found for address",
        )),
        Some(b) => Ok(b),
    }?;
    // Ok(5 as i64)
    Ok(AccountBalanceResponse {
        block_identifier: Rosetta::block_identifier(&block),
        balances: vec![Rosetta::amount(bal)],
        metadata: None,
    })
}

pub async fn account_coins_wrap(r: Rosetta, request: AccountCoinsRequest,) -> Result<Result<AccountCoinsResponse, ErrorInfo>, Rejection> {
    Ok(account_coins(r, request).await)
}

pub async fn account_coins(
    r: Rosetta,
    request: AccountCoinsRequest,
) -> Result<AccountCoinsResponse, ErrorInfo> {
    r.validate_network(request.network_identifier).await?;
    r.validate_currencies(&request.currencies).await?;
    let address = Address::parse(request.account_identifier.address)?;
    let block: structs::Block = r.latest_block().await?;
    let entries = r.relay.ds.transaction_store.query_utxo_address(&address).await?;
    let mut map: HashMap<Vec<u8>, UtxoEntry> = HashMap::new();
    for e in entries {
        map.insert(e.to_utxo_id().coin_id(), e);
    }
    // TODO: Collect all transactions which have been accepted but which are not yet in a block.
    // Then, for those, filter on the address being queried, roll back any changes here
    // It should be -- remove any UTXOs generated in the current from those newer transactions
    // Add any UTXOs on the address from the current block.
    // In fact just does this by when finalizing a transaction -- send a message to the block formation shit.
    // Or just do a DS time based query
    // for tx in block.transactions {
    //     for e in tx.to_utxo_entries(0) {
    //         let addr1 = e.output.expect("o").address.expect("a").address.safe_bytes()?;
    //         if addr1 == address.safe_bytes()? {
    //             coins.insert(e.to_utxo_id().coin_id(), e);
    //         }
    //     }
    // }

    let mut coins = vec![];
    for (k, entry) in map {
        coins.push(Coin {
            coin_identifier: CoinIdentifier {
                identifier: hex::encode(k),
            },
            amount: Rosetta::amount(entry.output.expect("o").amount() as i64),
        });
    }

    Ok(AccountCoinsResponse {
        block_identifier: Rosetta::block_identifier(&block),
        coins,
        metadata: None,
    })
}

pub async fn block(r: Rosetta, request: BlockRequest) -> Result<BlockResponse, ErrorInfo> {
    r.validate_network(request.network_identifier).await?;
    let mut maybe_hash = None;
    let partial_block_identifier = request.block_identifier;
    if let Some(h) = partial_block_identifier.hash {
        maybe_hash = Some(schema::decode_hex(h)?);
    };
    // TODO: Shouldn't this whole thing really check to verify the active height / active chain thing works?
    // revisit this on block removed events.
    let maybe_block = r
        .relay
        .ds
        .address_block_store
        .query_block_hash_height(maybe_hash, partial_block_identifier.index)
        .await?;
    Ok(BlockResponse {
        block: match maybe_block {
            Some(b) => r.translate_block(b, State::Finalized).await?.into(),
            None => None,
        },
        other_transactions: None,
    })
}

pub async fn block_transaction(
    r: Rosetta,
    request: BlockTransactionRequest,
) -> Result<BlockTransactionResponse, ErrorInfo> {
    r.validate_network(request.network_identifier).await?;
    let block = r
        .query_block(
            Some(request.block_identifier.hash),
            Some(request.block_identifier.index),
        )
        .await?
        .ok_or(error_message(
            RGError::UnknownBlock,
            "Block not found in data store",
        ))?;

    for t in block.transactions {
        if t.hash_hex()? == request.transaction_identifier.hash {
            let transaction = r.translate_transaction(t.clone(), State::Finalized).await?;
            return Ok(BlockTransactionResponse { transaction });
        }
    }
    Err(error_message(
        RGError::UnknownTransaction,
        "Transaction not found in block",
    ))
}

pub async fn call(r: Rosetta, request: CallRequest) -> Result<CallResponse, ErrorInfo> {
    r.validate_network(request.network_identifier).await?;
    return Ok(CallResponse {
        result: GenericMetadata {},
        idempotent: true,
    });
}


pub async fn construction_combine(
    r: Rosetta,
    request: ConstructionCombineRequest,
) -> Result<ConstructionCombineResponse, ErrorInfo> {
    r.validate_network(request.network_identifier).await?;
    let mut tx = structs::Transaction::from_hex(request.unsigned_transaction)?;

    for signature in request.signatures {
        let proof = Rosetta::signature_to_proof(signature.clone())?;
        let pay = signature.signing_payload;
        let pay_b = from_hex(pay.hex_bytes)?;
        for i in tx.inputs.iter_mut() {
            if i.utxo_id.safe_get()?.transaction_hash.safe_bytes()? == pay_b {
                i.proof.push(proof.clone());
            }
        }
    }
    use std::cmp::Ordering;
    for i in tx.inputs.iter_mut() {
        i.proof.sort_by(|x, y| {
            let vec = x.clone().proto_serialize();
            let vec1 = y.clone().proto_serialize();
            vec.cmp(&vec1)
        });
    }
    // Necessary to sort proof for weights? Not actually used so...
    Ok(ConstructionCombineResponse {
        signed_transaction: hex::encode(tx.proto_serialize()),
    })
}

pub async fn construction_derive(
    r: Rosetta,
    request: ConstructionDeriveRequest,
) -> Result<ConstructionDeriveResponse, ErrorInfo> {
    r.validate_network(request.network_identifier).await?;
    // TODO: Translate to internal type, add address adapter
    let address: Address = Rosetta::translate_public_key(request.public_key)?.into();
    Ok(ConstructionDeriveResponse {
        address: Some(address.render_string()?),
        account_identifier: Some(Rosetta::account_identifier(address)?),
        // TODO: Request metadata for attaching other keys and multisig weights
        metadata: None,
    })
}
pub async fn construction_hash(
    r: Rosetta,
    request: ConstructionHashRequest,
) -> Result<TransactionIdentifier, ErrorInfo> {
    r.validate_network(request.network_identifier).await?;
    Ok(TransactionIdentifier {
        hash: structs::Transaction::from_hex(request.signed_transaction)?.hash_hex()?,
    })
}

pub async fn construction_metadata(
    r: Rosetta,
    request: ConstructionMetadataRequest,
) -> Result<ConstructionMetadataResponse, ErrorInfo> {
    r.validate_network(request.network_identifier).await?;
    let keys = request
        .public_keys
        .ok_or(error_message(RGError::MissingField, "Missing public keys"))?;
    let mut utxos = vec![];
    for k in keys {
        let address: Address = Rosetta::translate_public_key(k)?.into();
        let utxo = r.relay.ds.transaction_store.query_utxo_address(&address).await?;
        // let utxo =
        //     DataStore::map_err_sqlx(r.relay.ds.query_utxo_address(vec![address]).await)?;
        utxos.extend(utxo);
    }
    Ok(ConstructionMetadataResponse {
        metadata: ConstructionMetadataResponseMetadata { utxos },
        suggested_fee: None,
    })
}

pub async fn construction_parse(
    r: Rosetta,
    request: ConstructionParseRequest,
) -> Result<ConstructionParseResponse, ErrorInfo> {
    r.validate_network(request.network_identifier).await?;
    let tx = structs::Transaction::from_hex(request.transaction)?;
    let tx_r = r.translate_transaction(tx.clone(), State::Pending).await?;
    let operations = tx_r.operations.clone();
    let mut signers = None;
    let mut account_identifier_signers = None;
    if request.signed {
        let mut signers_v = vec![];
        let mut ai_signers_v = vec![];
        for i in tx.inputs {
            if !i.proof.is_empty() {
                let output = r.input_output(i.clone()).await?;
                let address = output.address.expect("a");
                signers_v.push(address.render_string()?);
                ai_signers_v.push(Rosetta::account_identifier(address)?);
            }
        }
        signers = Some(signers_v);
        account_identifier_signers = Some(ai_signers_v);
    }
    Ok(ConstructionParseResponse {
        operations,
        signers,
        account_identifier_signers,
        metadata: None,
    })
}

pub async fn construction_payloads(
    r: Rosetta,
    request: ConstructionPayloadsRequest,
) -> Result<ConstructionPayloadsResponse, ErrorInfo> {
    r.validate_network(request.network_identifier).await?;
    let mut tx = structs::Transaction {
        inputs: vec![],
        outputs: vec![],
        struct_metadata: struct_metadata_new(), // TODO: Edit time here??
        options: None,
    };
    let mut payloads = vec![];
    let meta = request
        .metadata
        .ok_or(error_message(RGError::MissingField, "Missing metadata"))?;
    let mut input_address_utxo_map: HashMap<Address, Vec<UtxoEntry>> = HashMap::new();
    for x in meta.utxos {
        let entry = input_address_utxo_map.get_mut(&x.address.safe_get()?.clone());
        match entry {
            Some(e) => {
                e.push(x.clone());
            }
            None => {
                input_address_utxo_map.insert(x.address.clone().expect("a"), vec![x.clone()]);
            }
        }
    }

    for operation in &request.operations {
        for a in operation.amount.clone() {
            for acc in operation.account.clone() {
                let addr_bytes = from_hex(acc.address.clone())?;
                let addr: Address = addr_bytes.clone().into();
                let ai64 = i64_from_string(a.clone().value)?;
                if ai64 >= 0 {
                    tx.outputs.push(Output::new(&addr.into(), ai64));
                } else {
                    let utxos =
                        input_address_utxo_map
                            .get(&addr)
                            .ok_or(error_message(
                                RGError::MissingField,
                                "Missing metadata for input",
                            ))?;
                    for utxo in utxos {
                        let inp = utxo.to_input();
                        tx.inputs.push(inp);
                        payloads.push(SigningPayload {
                            address: Some(acc.address.clone()),
                            account_identifier: Some(acc.clone()),
                            hex_bytes: utxo.transaction_hash.safe_get()?.hex(),
                            signature_type: Some(SignatureType::ECDSA),
                        });
                    }
                }
            }
        }
    }

    // let keys = request.public_keys.ok_or(error_message(RGError::MissingField, "Missing public keys"))?;

    Ok(ConstructionPayloadsResponse {
        unsigned_transaction: hex::encode(tx.proto_serialize()),
        payloads,
    })
}

pub async fn construction_preprocess(
    r: Rosetta,
    request: ConstructionPreprocessRequest,
) -> Result<ConstructionPreprocessResponse, ErrorInfo> {
    r.validate_network(request.network_identifier).await?;
    let mut required_public_keys = vec![];
    for o in request.operations {
        for a in o.amount {
            let ai64 = i64_from_string(a.clone().value)?;
            if ai64 < 0 {
                match o.account.clone() {
                    None => {}
                    Some(ai) => required_public_keys.push(ai.clone()),
                }
            }
        }
    }
    Ok(ConstructionPreprocessResponse {
        options: None,
        required_public_keys: Some(required_public_keys),
    })
}

pub async fn construction_submit(
    r: Rosetta,
    request: ConstructionSubmitRequest,
) -> Result<TransactionIdentifier, ErrorInfo> {
    r.validate_network(request.network_identifier).await?;
    let tx = structs::Transaction::from_hex(request.signed_transaction)?;
    r.relay
        .submit_transaction(SubmitTransactionRequest {
            transaction: tx.clone().into(),
            sync_query_response: false,
        })
        .await?;
    Rosetta::transaction_identifier(&tx)
}

// TODO: How to deal with this?
// Build a new table called block event or something?
// update block index table to include both old and new ones?? how to to handle gc?
pub async fn events_blocks(
    r: Rosetta,
    request: EventsBlocksRequest,
) -> Result<EventsBlocksResponse, ErrorInfo> {
    r.validate_network(request.network_identifier).await?;
    Ok(EventsBlocksResponse{
        max_sequence: 0,
        events: vec![]
    })
}

pub async fn mempool(
    r: Rosetta,
    request: NetworkRequest,
) -> Result<MempoolResponse, ErrorInfo> {
    r.validate_network(request.network_identifier).await?;
    // TODO: make separate mempool that removes stuff.
    let mut transaction_identifiers = vec![];
    for x in r.relay.transaction_channels.iter() {
        transaction_identifiers.push(TransactionIdentifier{
            hash: x.key().hex()
        });
    }
    Ok(
        MempoolResponse{
            transaction_identifiers
        }
    )
}

pub async fn mempool_transaction(
    r: Rosetta,
    request: MempoolTransactionRequest,
) -> Result<MempoolTransactionResponse, ErrorInfo> {
    r.validate_network(request.network_identifier).await?;
    let hash = Hash::from_hex(request.transaction_identifier.hash)?;
    // let tx: Result<structs::Transaction, ErrorInfo> = ;
    let result = r.relay.transaction_channels.get(&hash);
    if let Some(rr) = result {
        let tx = rr.value().transaction.clone();
        let transaction = r.translate_transaction(tx, State::Pending).await?;
        Ok({MempoolTransactionResponse{
            transaction,
            metadata: None
        }})
    } else {
        Err(error_message(RGError::UnknownTransaction, ""))
    }
}

pub async fn network_list(
    r: Rosetta,
    _request: MetadataRequest,
) -> Result<NetworkListResponse, ErrorInfo> {
    let current_network = r.relay.node_config.network.to_std_string();
    Ok(NetworkListResponse{
        network_identifiers: vec![NetworkIdentifier{
            blockchain: Rosetta::redgold_blockchain(),
            network: current_network,
            sub_network_identifier: None
        }]
    })
}

pub async fn network_options(
    r: Rosetta,
    request: NetworkRequest,
) -> Result<NetworkOptionsResponse, ErrorInfo> {
    r.validate_network(request.network_identifier).await?;
    Ok(NetworkOptionsResponse{
        version: Version {
            // TODO: grab from schema file
            rosetta_version: "".to_string(),
            // Relay node version?
            node_version: "".to_string(),
            middleware_version: None,
            metadata: None
        },
        allow: Allow {
            operation_statuses: vec![
                OperationStatus{
                    status: format!("{:?}", State::Pending),
                    successful: false
                },
                OperationStatus{
                    status: format!("{:?}", State::Finalized),
                    successful: true
                }],
            operation_types: vec![Rosetta::operation_type()],
            // TODO: Errors list
            errors: vec![],
            historical_balance_lookup: false,
            timestamp_start_index: None,
            call_methods: vec![],
            balance_exemptions: vec![],
            mempool_coins: false,
            block_hash_case: None,
            transaction_hash_case: None
        }
    })
}


pub async fn network_status(
    r: Rosetta,
    request: NetworkRequest,
) -> Result<NetworkStatusResponse, ErrorInfo> {
    r.validate_network(request.network_identifier).await?;
    Ok(NetworkStatusResponse{
        current_block_identifier: BlockIdentifier { index: 0, hash: "".to_string() },
        current_block_timestamp: 0,
        genesis_block_identifier: BlockIdentifier { index: 0, hash: "".to_string() },
        oldest_block_identifier: None,
        sync_status: None,
        peers: vec![]
    })
}

#[allow(unused_variables, unused_assignments, unused_mut, dead_code)]
pub async fn search_transactions(
    r: Rosetta,
    request: SearchTransactionsRequest,
) -> Result<SearchTransactionsResponse, ErrorInfo> {
    r.validate_network(request.network_identifier).await?;
    if let Some(c) = request.currency {
        r.validate_currency(&c).await?;
    }
    if let Some(s) = request._type {
        r.validate_type(s).await?;
    }
    // if let Some(s) = request.coin_identifier {
    //     // todo validate
    //     //r.validate_type(s)?;
    // }
    // if let Some(s) = request.status {
    //     // todo validate
    //     //r.validate_type(s)?;
    // }

    let mut _transaction_hash: Option<Hash> = None;
    let mut _addr: Option<Address> = None;
    let mut _success: bool = false;
    let mut _is_and: bool = false;
    let mut _is_or: bool = false;
    // let limit = request.limit;
    // let offset = request.offset;
    // let max_block = request.max_block;

    for ti in request.transaction_identifier {
        let b = from_hex(ti.hash)?;
        _transaction_hash = Some(Hash::new(b));
    }

    for a in request.address {
        _addr = Some(Address::parse(a)?);
    }
    if let Some(s) = request.account_identifier {
        // TODO: Abstract this, validate no subaccount present or throw error.
        _addr = Some(Address::parse(s.address)?);
    }

    if let Some(s) = request.success {
        _success = s;
    }

    if let Some(s) = request.operator {
        match s {
            Operator::OR => {
                _is_or = true;
            }
            Operator::AND => {
                _is_and = true;
            }
        }
    }

    Ok(SearchTransactionsResponse{
        transactions: vec![],
        total_count: 0,
        next_offset: None
    })
}
