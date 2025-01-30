#![allow(unused_imports, unused_qualifications, unused_extern_crates, unexpected_cfgs)]
// extern crate chrono;


use serde::ser::Serializer;

use redgold_schema::structs::ErrorInfo;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
// use swagger;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct GenericMetadata {}

/// An AccountBalanceRequest is utilized to make a balance request on the /account/balance endpoint. If the block_identifier is populated, a historical balance query should be performed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct AccountBalanceRequest {
    #[serde(rename = "network_identifier")]
    pub network_identifier: NetworkIdentifier,

    #[serde(rename = "account_identifier")]
    pub account_identifier: AccountIdentifier,

    #[serde(rename = "block_identifier")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_identifier: Option<PartialBlockIdentifier>,

    /// In some cases, the caller may not want to retrieve all available balances for an AccountIdentifier. If the currencies field is populated, only balances for the specified currencies will be returned. If not populated, all available balances will be returned.
    #[serde(rename = "currencies")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currencies: Option<Vec<Currency>>,
}

impl AccountBalanceRequest {
    pub fn new(
        network_identifier: NetworkIdentifier,
        account_identifier: AccountIdentifier,
    ) -> AccountBalanceRequest {
        AccountBalanceRequest {
            network_identifier: network_identifier,
            account_identifier: account_identifier,
            block_identifier: None,
            currencies: None,
        }
    }
}

/// An AccountBalanceResponse is returned on the /account/balance endpoint. If an account has a balance for each AccountIdentifier describing it (ex: an ERC-20 token balance on a few smart contracts), an account balance request must be made with each AccountIdentifier. The `coins` field was removed and replaced by by `/account/coins` in `v1.4.7`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct AccountBalanceResponse {
    #[serde(rename = "block_identifier")]
    pub block_identifier: BlockIdentifier,

    /// A single account may have a balance in multiple currencies.
    #[serde(rename = "balances")]
    pub balances: Vec<Amount>,

    /// Account-based blockchains that utilize a nonce or sequence number should include that number in the metadata. This number could be unique to the identifier or global across the account address.
    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenericMetadata>,
}

impl AccountBalanceResponse {
    pub fn new(block_identifier: BlockIdentifier, balances: Vec<Amount>) -> AccountBalanceResponse {
        AccountBalanceResponse {
            block_identifier: block_identifier,
            balances: balances,
            metadata: None,
        }
    }
}

/// AccountCoinsRequest is utilized to make a request on the /account/coins endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct AccountCoinsRequest {
    #[serde(rename = "network_identifier")]
    pub network_identifier: NetworkIdentifier,

    #[serde(rename = "account_identifier")]
    pub account_identifier: AccountIdentifier,

    /// Include state from the mempool when looking up an account's unspent coins. Note, using this functionality breaks any guarantee of idempotency.
    #[serde(rename = "include_mempool")]
    pub include_mempool: bool,

    /// In some cases, the caller may not want to retrieve coins for all currencies for an AccountIdentifier. If the currencies field is populated, only coins for the specified currencies will be returned. If not populated, all unspent coins will be returned.
    #[serde(rename = "currencies")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currencies: Option<Vec<Currency>>,
}

impl AccountCoinsRequest {
    pub fn new(
        network_identifier: NetworkIdentifier,
        account_identifier: AccountIdentifier,
        include_mempool: bool,
    ) -> AccountCoinsRequest {
        AccountCoinsRequest {
            network_identifier: network_identifier,
            account_identifier: account_identifier,
            include_mempool: include_mempool,
            currencies: None,
        }
    }
}

/// AccountCoinsResponse is returned on the /account/coins endpoint and includes all unspent Coins owned by an AccountIdentifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct AccountCoinsResponse {
    #[serde(rename = "block_identifier")]
    pub block_identifier: BlockIdentifier,

    /// If a blockchain is UTXO-based, all unspent Coins owned by an account_identifier should be returned alongside the balance. It is highly recommended to populate this field so that users of the Rosetta API implementation don't need to maintain their own indexer to track their UTXOs.
    #[serde(rename = "coins")]
    pub coins: Vec<Coin>,

    /// Account-based blockchains that utilize a nonce or sequence number should include that number in the metadata. This number could be unique to the identifier or global across the account address.
    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenericMetadata>,
}

impl AccountCoinsResponse {
    pub fn new(block_identifier: BlockIdentifier, coins: Vec<Coin>) -> AccountCoinsResponse {
        AccountCoinsResponse {
            block_identifier: block_identifier,
            coins: coins,
            metadata: None,
        }
    }
}

/// The account_identifier uniquely identifies an account within a network. All fields in the account_identifier are utilized to determine this uniqueness (including the metadata field, if populated).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct AccountIdentifier {
    /// The address may be a cryptographic public key (or some encoding of it) or a provided username.
    #[serde(rename = "address")]
    pub address: String,

    #[serde(rename = "sub_account")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_account: Option<SubAccountIdentifier>,

    /// Blockchains that utilize a username model (where the address is not a derivative of a cryptographic public key) should specify the public key(s) owned by the address in metadata.
    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenericMetadata>,
}

impl AccountIdentifier {
    pub fn new(address: String) -> AccountIdentifier {
        AccountIdentifier {
            address: address,
            sub_account: None,
            metadata: None,
        }
    }
}

/// Allow specifies supported Operation status, Operation types, and all possible error statuses. This Allow GenericMetadata is used by clients to validate the correctness of a Rosetta Server implementation. It is expected that these clients will error if they receive some response that contains any of the above information that is not specified here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct Allow {
    /// All Operation.Status this implementation supports. Any status that is returned during parsing that is not listed here will cause client validation to error.
    #[serde(rename = "operation_statuses")]
    pub operation_statuses: Vec<OperationStatus>,

    /// All Operation.Type this implementation supports. Any type that is returned during parsing that is not listed here will cause client validation to error.
    #[serde(rename = "operation_types")]
    pub operation_types: Vec<String>,

    /// All Errors that this implementation could return. Any error that is returned during parsing that is not listed here will cause client validation to error.
    #[serde(rename = "errors")]
    pub errors: Vec<Error>,

    /// Any Rosetta implementation that supports querying the balance of an account at any height in the past should set this to true.
    #[serde(rename = "historical_balance_lookup")]
    pub historical_balance_lookup: bool,

    /// If populated, `timestamp_start_index` indicates the first block index where block timestamps are considered valid (i.e. all blocks less than `timestamp_start_index` could have invalid timestamps). This is useful when the genesis block (or blocks) of a network have timestamp 0. If not populated, block timestamps are assumed to be valid for all available blocks.
    #[serde(rename = "timestamp_start_index")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_start_index: Option<i64>,

    /// All methods that are supported by the /call endpoint. Communicating which parameters should be provided to /call is the responsibility of the implementer (this is en lieu of defining an entire type system and requiring the implementer to define that in Allow).
    #[serde(rename = "call_methods")]
    pub call_methods: Vec<String>,

    /// BalanceExemptions is an array of BalanceExemption indicating which account balances could change without a corresponding Operation. BalanceExemptions should be used sparingly as they may introduce significant complexity for integrators that attempt to reconcile all account balance changes. If your implementation relies on any BalanceExemptions, you MUST implement historical balance lookup (the ability to query an account balance at any BlockIdentifier).
    #[serde(rename = "balance_exemptions")]
    pub balance_exemptions: Vec<BalanceExemption>,

    /// Any Rosetta implementation that can update an AccountIdentifier's unspent coins based on the contents of the mempool should populate this field as true. If false, requests to `/account/coins` that set `include_mempool` as true will be automatically rejected.
    #[serde(rename = "mempool_coins")]
    pub mempool_coins: bool,

    #[serde(rename = "block_hash_case")]
    // #[serde(deserialize_with = "swagger::nullable_format::deserialize_optional_nullable")]
    // #[serde(default = "swagger::nullable_format::default_optional_nullable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_hash_case: Option<Case>,

    #[serde(rename = "transaction_hash_case")]
    // #[serde(deserialize_with = "swagger::nullable_format::deserialize_optional_nullable")]
    // #[serde(default = "swagger::nullable_format::default_optional_nullable")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_hash_case: Option<Case>,
}

impl Allow {
    pub fn new(
        operation_statuses: Vec<OperationStatus>,
        operation_types: Vec<String>,
        errors: Vec<Error>,
        historical_balance_lookup: bool,
        call_methods: Vec<String>,
        balance_exemptions: Vec<BalanceExemption>,
        mempool_coins: bool,
    ) -> Allow {
        Allow {
            operation_statuses: operation_statuses,
            operation_types: operation_types,
            errors: errors,
            historical_balance_lookup: historical_balance_lookup,
            timestamp_start_index: None,
            call_methods: call_methods,
            balance_exemptions: balance_exemptions,
            mempool_coins: mempool_coins,
            block_hash_case: None,
            transaction_hash_case: None,
        }
    }
}

/// Amount is some Value of a Currency. It is considered invalid to specify a Value without a Currency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct Amount {
    /// Value of the transaction in atomic units represented as an arbitrary-sized signed integer. For example, 1 BTC would be represented by a value of 100000000.
    #[serde(rename = "value")]
    pub value: String,

    #[serde(rename = "currency")]
    pub currency: Currency,

    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenericMetadata>,
}

impl Amount {
    pub fn new(value: String, currency: Currency) -> Amount {
        Amount {
            value: value,
            currency: currency,
            metadata: None,
        }
    }
}

/// BalanceExemption indicates that the balance for an exempt account could change without a corresponding Operation. This typically occurs with staking rewards, vesting balances, and Currencies with a dynamic supply. Currently, it is possible to exempt an account from strict reconciliation by SubAccountIdentifier.Address or by Currency. This means that any account with SubAccountIdentifier.Address would be exempt or any balance of a particular Currency would be exempt, respectively. BalanceExemptions should be used sparingly as they may introduce significant complexity for integrators that attempt to reconcile all account balance changes. If your implementation relies on any BalanceExemptions, you MUST implement historical balance lookup (the ability to query an account balance at any BlockIdentifier).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct BalanceExemption {
    /// SubAccountAddress is the SubAccountIdentifier.Address that the BalanceExemption applies to (regardless of the value of SubAccountIdentifier.Metadata).
    #[serde(rename = "sub_account_address")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_account_address: Option<String>,

    #[serde(rename = "currency")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<Currency>,

    #[serde(rename = "exemption_type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exemption_type: Option<ExemptionType>,
}

impl BalanceExemption {
    pub fn new() -> BalanceExemption {
        BalanceExemption {
            sub_account_address: None,
            currency: None,
            exemption_type: None,
        }
    }
}

/// Blocks contain an array of Transactions that occurred at a particular BlockIdentifier. A hard requirement for blocks returned by Rosetta implementations is that they MUST be _inalterable_: once a client has requested and received a block identified by a specific BlockIndentifier, all future calls for that same BlockIdentifier must return the same block contents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct Block {
    #[serde(rename = "block_identifier")]
    pub block_identifier: BlockIdentifier,

    #[serde(rename = "parent_block_identifier")]
    pub parent_block_identifier: BlockIdentifier,

    /// The timestamp of the block in milliseconds since the Unix Epoch. The timestamp is stored in milliseconds because some blockchains produce blocks more often than once a second.
    #[serde(rename = "timestamp")]
    pub timestamp: i64,

    #[serde(rename = "transactions")]
    pub transactions: Vec<Transaction>,

    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenericMetadata>,
}

impl Block {
    pub fn new(
        block_identifier: BlockIdentifier,
        parent_block_identifier: BlockIdentifier,
        timestamp: i64,
        transactions: Vec<Transaction>,
    ) -> Block {
        Block {
            block_identifier: block_identifier,
            parent_block_identifier: parent_block_identifier,
            timestamp: timestamp,
            transactions: transactions,
            metadata: None,
        }
    }
}

/// BlockEvent represents the addition or removal of a BlockIdentifier from storage. Streaming BlockEvents allows lightweight clients to update their own state without needing to implement their own syncing logic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct BlockEvent {
    /// sequence is the unique identifier of a BlockEvent within the context of a NetworkIdentifier.
    #[serde(rename = "sequence")]
    pub sequence: i64,

    #[serde(rename = "block_identifier")]
    pub block_identifier: BlockIdentifier,

    #[serde(rename = "type")]
    pub _type: BlockEventType,
}

impl BlockEvent {
    pub fn new(
        sequence: i64,
        block_identifier: BlockIdentifier,
        _type: BlockEventType,
    ) -> BlockEvent {
        BlockEvent {
            sequence: sequence,
            block_identifier: block_identifier,
            _type: _type,
        }
    }
}

/// BlockEventType determines if a BlockEvent represents the addition or removal of a block.
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGenericEnum))]
pub enum BlockEventType {
    #[serde(rename = "block_added")]
    ADDED,
    #[serde(rename = "block_removed")]
    REMOVED,
}

impl ::std::fmt::Display for BlockEventType {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            BlockEventType::ADDED => write!(f, "{}", "block_added"),
            BlockEventType::REMOVED => write!(f, "{}", "block_removed"),
        }
    }
}

impl ::std::str::FromStr for BlockEventType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "block_added" => Ok(BlockEventType::ADDED),
            "block_removed" => Ok(BlockEventType::REMOVED),
            _ => Err(()),
        }
    }
}

/// The block_identifier uniquely identifies a block in a particular network.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct BlockIdentifier {
    /// This is also known as the block height.
    #[serde(rename = "index")]
    pub index: i64,

    /// This should be normalized according to the case specified in the block_hash_case network options.
    #[serde(rename = "hash")]
    pub hash: String,
}

impl BlockIdentifier {
    pub fn new(index: i64, hash: String) -> BlockIdentifier {
        BlockIdentifier {
            index: index,
            hash: hash,
        }
    }
}

/// A BlockRequest is utilized to make a block request on the /block endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct BlockRequest {
    #[serde(rename = "network_identifier")]
    pub network_identifier: NetworkIdentifier,

    #[serde(rename = "block_identifier")]
    pub block_identifier: PartialBlockIdentifier,
}

impl BlockRequest {
    pub fn new(
        network_identifier: NetworkIdentifier,
        block_identifier: PartialBlockIdentifier,
    ) -> BlockRequest {
        BlockRequest {
            network_identifier: network_identifier,
            block_identifier: block_identifier,
        }
    }
}

/// A BlockResponse includes a fully-populated block or a partially-populated block with a list of other transactions to fetch (other_transactions). As a result of the consensus algorithm of some blockchains, blocks can be omitted (i.e. certain block indices can be skipped). If a query for one of these omitted indices is made, the response should not include a `Block` GenericMetadata. It is VERY important to note that blocks MUST still form a canonical, connected chain of blocks where each block has a unique index. In other words, the `PartialBlockIdentifier` of a block after an omitted block should reference the last non-omitted block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct BlockResponse {
    #[serde(rename = "block")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block: Option<Block>,

    /// Some blockchains may require additional transactions to be fetched that weren't returned in the block response (ex: block only returns transaction hashes). For blockchains with a lot of transactions in each block, this can be very useful as consumers can concurrently fetch all transactions returned.
    #[serde(rename = "other_transactions")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_transactions: Option<Vec<TransactionIdentifier>>,
}

impl BlockResponse {
    pub fn new() -> BlockResponse {
        BlockResponse {
            block: None,
            other_transactions: None,
        }
    }
}

/// BlockTransaction contains a populated Transaction and the BlockIdentifier that contains it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct BlockTransaction {
    #[serde(rename = "block_identifier")]
    pub block_identifier: BlockIdentifier,

    #[serde(rename = "transaction")]
    pub transaction: Transaction,
}

impl BlockTransaction {
    pub fn new(block_identifier: BlockIdentifier, transaction: Transaction) -> BlockTransaction {
        BlockTransaction {
            block_identifier: block_identifier,
            transaction: transaction,
        }
    }
}

/// A BlockTransactionRequest is used to fetch a Transaction included in a block that is not returned in a BlockResponse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct BlockTransactionRequest {
    #[serde(rename = "network_identifier")]
    pub network_identifier: NetworkIdentifier,

    #[serde(rename = "block_identifier")]
    pub block_identifier: BlockIdentifier,

    #[serde(rename = "transaction_identifier")]
    pub transaction_identifier: TransactionIdentifier,
}

impl BlockTransactionRequest {
    pub fn new(
        network_identifier: NetworkIdentifier,
        block_identifier: BlockIdentifier,
        transaction_identifier: TransactionIdentifier,
    ) -> BlockTransactionRequest {
        BlockTransactionRequest {
            network_identifier: network_identifier,
            block_identifier: block_identifier,
            transaction_identifier: transaction_identifier,
        }
    }
}

/// A BlockTransactionResponse contains information about a block transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct BlockTransactionResponse {
    #[serde(rename = "transaction")]
    pub transaction: Transaction,
}

impl BlockTransactionResponse {
    pub fn new(transaction: Transaction) -> BlockTransactionResponse {
        BlockTransactionResponse {
            transaction: transaction,
        }
    }
}

/// CallRequest is the input to the `/call` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct CallRequest {
    #[serde(rename = "network_identifier")]
    pub network_identifier: NetworkIdentifier,

    /// Method is some network-specific procedure call. This method could map to a network-specific RPC endpoint, a method in an SDK generated from a smart contract, or some hybrid of the two. The implementation must define all available methods in the Allow GenericMetadata. However, it is up to the caller to determine which parameters to provide when invoking `/call`.
    #[serde(rename = "method")]
    pub method: String,

    /// Parameters is some network-specific argument for a method. It is up to the caller to determine which parameters to provide when invoking `/call`.
    #[serde(rename = "parameters")]
    pub parameters: GenericMetadata,
}

impl CallRequest {
    pub fn new(
        network_identifier: NetworkIdentifier,
        method: String,
        parameters: GenericMetadata,
    ) -> CallRequest {
        CallRequest {
            network_identifier: network_identifier,
            method: method,
            parameters: parameters,
        }
    }
}

/// CallResponse contains the result of a `/call` invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct CallResponse {
    /// Result contains the result of the `/call` invocation. This result will not be inspected or interpreted by Rosetta tooling and is left to the caller to decode.
    #[serde(rename = "result")]
    pub result: GenericMetadata,

    /// Idempotent indicates that if `/call` is invoked with the same CallRequest again, at any point in time, it will return the same CallResponse. Integrators may cache the CallResponse if this is set to true to avoid making unnecessary calls to the Rosetta implementation. For this reason, implementers should be very conservative about returning true here or they could cause issues for the caller.
    #[serde(rename = "idempotent")]
    pub idempotent: bool,
}

impl CallResponse {
    pub fn new(result: GenericMetadata, idempotent: bool) -> CallResponse {
        CallResponse {
            result: result,
            idempotent: idempotent,
        }
    }
}

/// Case specifies the expected case for strings and hashes.
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGenericEnum))]
pub enum Case {
    #[serde(rename = "upper_case")]
    UPPER_CASE,
    #[serde(rename = "lower_case")]
    LOWER_CASE,
    #[serde(rename = "case_sensitive")]
    CASE_SENSITIVE,
    #[serde(rename = "null")]
    NULL,
}

impl ::std::fmt::Display for Case {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            Case::UPPER_CASE => write!(f, "{}", "upper_case"),
            Case::LOWER_CASE => write!(f, "{}", "lower_case"),
            Case::CASE_SENSITIVE => write!(f, "{}", "case_sensitive"),
            Case::NULL => write!(f, "{}", "null"),
        }
    }
}

impl ::std::str::FromStr for Case {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "upper_case" => Ok(Case::UPPER_CASE),
            "lower_case" => Ok(Case::LOWER_CASE),
            "case_sensitive" => Ok(Case::CASE_SENSITIVE),
            "null" => Ok(Case::NULL),
            _ => Err(()),
        }
    }
}

/// Coin contains its unique identifier and the amount it represents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct Coin {
    #[serde(rename = "coin_identifier")]
    pub coin_identifier: CoinIdentifier,

    #[serde(rename = "amount")]
    pub amount: Amount,
}

impl Coin {
    pub fn new(coin_identifier: CoinIdentifier, amount: Amount) -> Coin {
        Coin {
            coin_identifier: coin_identifier,
            amount: amount,
        }
    }
}

/// CoinActions are different state changes that a Coin can undergo. When a Coin is created, it is coin_created. When a Coin is spent, it is coin_spent. It is assumed that a single Coin cannot be created or spent more than once.
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGenericEnum))]
pub enum CoinAction {
    #[serde(rename = "coin_created")]
    CREATED,
    #[serde(rename = "coin_spent")]
    SPENT,
}

impl ::std::fmt::Display for CoinAction {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            CoinAction::CREATED => write!(f, "{}", "coin_created"),
            CoinAction::SPENT => write!(f, "{}", "coin_spent"),
        }
    }
}

impl ::std::str::FromStr for CoinAction {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "coin_created" => Ok(CoinAction::CREATED),
            "coin_spent" => Ok(CoinAction::SPENT),
            _ => Err(()),
        }
    }
}

/// CoinChange is used to represent a change in state of a some coin identified by a coin_identifier. This GenericMetadata is part of the Operation model and must be populated for UTXO-based blockchains. Coincidentally, this abstraction of UTXOs allows for supporting both account-based transfers and UTXO-based transfers on the same blockchain (when a transfer is account-based, don't populate this model).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct CoinChange {
    #[serde(rename = "coin_identifier")]
    pub coin_identifier: CoinIdentifier,

    #[serde(rename = "coin_action")]
    pub coin_action: CoinAction,
}

impl CoinChange {
    pub fn new(coin_identifier: CoinIdentifier, coin_action: CoinAction) -> CoinChange {
        CoinChange {
            coin_identifier: coin_identifier,
            coin_action: coin_action,
        }
    }
}

/// CoinIdentifier uniquely identifies a Coin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct CoinIdentifier {
    /// Identifier should be populated with a globally unique identifier of a Coin. In Bitcoin, this identifier would be transaction_hash:index.
    #[serde(rename = "identifier")]
    pub identifier: String,
}

impl CoinIdentifier {
    pub fn new(identifier: String) -> CoinIdentifier {
        CoinIdentifier {
            identifier: identifier,
        }
    }
}

/// ConstructionCombineRequest is the input to the `/construction/combine` endpoint. It contains the unsigned transaction blob returned by `/construction/payloads` and all required signatures to create a network transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct ConstructionCombineRequest {
    #[serde(rename = "network_identifier")]
    pub network_identifier: NetworkIdentifier,

    #[serde(rename = "unsigned_transaction")]
    pub unsigned_transaction: String,

    #[serde(rename = "signatures")]
    pub signatures: Vec<Signature>,
}

impl ConstructionCombineRequest {
    pub fn new(
        network_identifier: NetworkIdentifier,
        unsigned_transaction: String,
        signatures: Vec<Signature>,
    ) -> ConstructionCombineRequest {
        ConstructionCombineRequest {
            network_identifier: network_identifier,
            unsigned_transaction: unsigned_transaction,
            signatures: signatures,
        }
    }
}

/// ConstructionCombineResponse is returned by `/construction/combine`. The network payload will be sent directly to the `construction/submit` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct ConstructionCombineResponse {
    #[serde(rename = "signed_transaction")]
    pub signed_transaction: String,
}

impl ConstructionCombineResponse {
    pub fn new(signed_transaction: String) -> ConstructionCombineResponse {
        ConstructionCombineResponse {
            signed_transaction: signed_transaction,
        }
    }
}

/// ConstructionDeriveRequest is passed to the `/construction/derive` endpoint. Network is provided in the request because some blockchains have different address formats for different networks. Metadata is provided in the request because some blockchains allow for multiple address types (i.e. different address for validators vs normal accounts).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct ConstructionDeriveRequest {
    #[serde(rename = "network_identifier")]
    pub network_identifier: NetworkIdentifier,

    #[serde(rename = "public_key")]
    pub public_key: PublicKey,

    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenericMetadata>,
}

impl ConstructionDeriveRequest {
    pub fn new(
        network_identifier: NetworkIdentifier,
        public_key: PublicKey,
    ) -> ConstructionDeriveRequest {
        ConstructionDeriveRequest {
            network_identifier: network_identifier,
            public_key: public_key,
            metadata: None,
        }
    }
}

/// ConstructionDeriveResponse is returned by the `/construction/derive` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct ConstructionDeriveResponse {
    /// [DEPRECATED by `account_identifier` in `v1.4.4`] Address in network-specific format.
    #[serde(rename = "address")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,

    #[serde(rename = "account_identifier")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_identifier: Option<AccountIdentifier>,

    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenericMetadata>,
}

impl ConstructionDeriveResponse {
    pub fn new() -> ConstructionDeriveResponse {
        ConstructionDeriveResponse {
            address: None,
            account_identifier: None,
            metadata: None,
        }
    }
}

/// ConstructionHashRequest is the input to the `/construction/hash` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct ConstructionHashRequest {
    #[serde(rename = "network_identifier")]
    pub network_identifier: NetworkIdentifier,

    #[serde(rename = "signed_transaction")]
    pub signed_transaction: String,
}

impl ConstructionHashRequest {
    pub fn new(
        network_identifier: NetworkIdentifier,
        signed_transaction: String,
    ) -> ConstructionHashRequest {
        ConstructionHashRequest {
            network_identifier: network_identifier,
            signed_transaction: signed_transaction,
        }
    }
}

/// A ConstructionMetadataRequest is utilized to get information required to construct a transaction. The Options GenericMetadata used to specify which metadata to return is left purposely unstructured to allow flexibility for implementers. Options is not required in the case that there is network-wide metadata of interest. Optionally, the request can also include an array of PublicKeys associated with the AccountIdentifiers returned in ConstructionPreprocessResponse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct ConstructionMetadataRequest {
    #[serde(rename = "network_identifier")]
    pub network_identifier: NetworkIdentifier,

    /// Some blockchains require different metadata for different types of transaction construction (ex: delegation versus a transfer). Instead of requiring a blockchain node to return all possible types of metadata for construction (which may require multiple node fetches), the client can populate an options GenericMetadata to limit the metadata returned to only the subset required.
    #[serde(rename = "options")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<GenericMetadata>,

    #[serde(rename = "public_keys")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_keys: Option<Vec<PublicKey>>,
}

impl ConstructionMetadataRequest {
    pub fn new(network_identifier: NetworkIdentifier) -> ConstructionMetadataRequest {
        ConstructionMetadataRequest {
            network_identifier: network_identifier,
            options: None,
            public_keys: None,
        }
    }
}

/// The ConstructionMetadataResponse returns network-specific metadata used for transaction construction. Optionally, the implementer can return the suggested fee associated with the transaction being constructed. The caller may use this info to adjust the intent of the transaction or to create a transaction with a different account that can pay the suggested fee. Suggested fee is an array in case fee payment must occur in multiple currencies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct ConstructionMetadataResponseMetadata {
    #[serde(rename = "utxos")]
    pub utxos: Vec<crate::schema::structs::UtxoEntry>,
}
/// The ConstructionMetadataResponse returns network-specific metadata used for transaction construction. Optionally, the implementer can return the suggested fee associated with the transaction being constructed. The caller may use this info to adjust the intent of the transaction or to create a transaction with a different account that can pay the suggested fee. Suggested fee is an array in case fee payment must occur in multiple currencies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct ConstructionMetadataResponse {
    #[serde(rename = "metadata")]
    pub metadata: ConstructionMetadataResponseMetadata,

    #[serde(rename = "suggested_fee")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_fee: Option<Vec<Amount>>,
}

impl ConstructionMetadataResponse {
    pub fn new(metadata: ConstructionMetadataResponseMetadata) -> ConstructionMetadataResponse {
        ConstructionMetadataResponse {
            metadata,
            suggested_fee: None,
        }
    }
}

/// ConstructionParseRequest is the input to the `/construction/parse` endpoint. It allows the caller to parse either an unsigned or signed transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct ConstructionParseRequest {
    #[serde(rename = "network_identifier")]
    pub network_identifier: NetworkIdentifier,

    /// Signed is a boolean indicating whether the transaction is signed.
    #[serde(rename = "signed")]
    pub signed: bool,

    /// This must be either the unsigned transaction blob returned by `/construction/payloads` or the signed transaction blob returned by `/construction/combine`.
    #[serde(rename = "transaction")]
    pub transaction: String,
}

impl ConstructionParseRequest {
    pub fn new(
        network_identifier: NetworkIdentifier,
        signed: bool,
        transaction: String,
    ) -> ConstructionParseRequest {
        ConstructionParseRequest {
            network_identifier: network_identifier,
            signed: signed,
            transaction: transaction,
        }
    }
}

/// ConstructionParseResponse contains an array of operations that occur in a transaction blob. This should match the array of operations provided to `/construction/preprocess` and `/construction/payloads`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct ConstructionParseResponse {
    #[serde(rename = "operations")]
    pub operations: Vec<Operation>,

    /// [DEPRECATED by `account_identifier_signers` in `v1.4.4`] All signers (addresses) of a particular transaction. If the transaction is unsigned, it should be empty.
    #[serde(rename = "signers")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signers: Option<Vec<String>>,

    #[serde(rename = "account_identifier_signers")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_identifier_signers: Option<Vec<AccountIdentifier>>,

    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenericMetadata>,
}

impl ConstructionParseResponse {
    pub fn new(operations: Vec<Operation>) -> ConstructionParseResponse {
        ConstructionParseResponse {
            operations: operations,
            signers: None,
            account_identifier_signers: None,
            metadata: None,
        }
    }
}

/// ConstructionPayloadsRequest is the request to `/construction/payloads`. It contains the network, a slice of operations, and arbitrary metadata that was returned by the call to `/construction/metadata`. Optionally, the request can also include an array of PublicKeys associated with the AccountIdentifiers returned in ConstructionPreprocessResponse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct ConstructionPayloadsRequest {
    #[serde(rename = "network_identifier")]
    pub network_identifier: NetworkIdentifier,

    #[serde(rename = "operations")]
    pub operations: Vec<Operation>,

    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ConstructionMetadataResponseMetadata>,

    #[serde(rename = "public_keys")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_keys: Option<Vec<PublicKey>>,
}

impl ConstructionPayloadsRequest {
    pub fn new(
        network_identifier: NetworkIdentifier,
        operations: Vec<Operation>,
    ) -> ConstructionPayloadsRequest {
        ConstructionPayloadsRequest {
            network_identifier: network_identifier,
            operations: operations,
            metadata: None,
            public_keys: None,
        }
    }
}

/// ConstructionTransactionResponse is returned by `/construction/payloads`. It contains an unsigned transaction blob (that is usually needed to construct the a network transaction from a collection of signatures) and an array of payloads that must be signed by the caller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct ConstructionPayloadsResponse {
    #[serde(rename = "unsigned_transaction")]
    pub unsigned_transaction: String,

    #[serde(rename = "payloads")]
    pub payloads: Vec<SigningPayload>,
}

impl ConstructionPayloadsResponse {
    pub fn new(
        unsigned_transaction: String,
        payloads: Vec<SigningPayload>,
    ) -> ConstructionPayloadsResponse {
        ConstructionPayloadsResponse {
            unsigned_transaction: unsigned_transaction,
            payloads: payloads,
        }
    }
}

/// ConstructionPreprocessRequest is passed to the `/construction/preprocess` endpoint so that a Rosetta implementation can determine which metadata it needs to request for construction. Metadata provided in this GenericMetadata should NEVER be a product of live data (i.e. the caller must follow some network-specific data fetching strategy outside of the Construction API to populate required Metadata). If live data is required for construction, it MUST be fetched in the call to `/construction/metadata`. The caller can provide a max fee they are willing to pay for a transaction. This is an array in the case fees must be paid in multiple currencies. The caller can also provide a suggested fee multiplier to indicate that the suggested fee should be scaled. This may be used to set higher fees for urgent transactions or to pay lower fees when there is less urgency. It is assumed that providing a very low multiplier (like 0.0001) will never lead to a transaction being created with a fee less than the minimum network fee (if applicable). In the case that the caller provides both a max fee and a suggested fee multiplier, the max fee will set an upper bound on the suggested fee (regardless of the multiplier provided).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct ConstructionPreprocessRequest {
    #[serde(rename = "network_identifier")]
    pub network_identifier: NetworkIdentifier,

    #[serde(rename = "operations")]
    pub operations: Vec<Operation>,

    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenericMetadata>,

    #[serde(rename = "max_fee")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee: Option<Vec<Amount>>,

    #[serde(rename = "suggested_fee_multiplier")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_fee_multiplier: Option<f64>,
}

impl ConstructionPreprocessRequest {
    pub fn new(
        network_identifier: NetworkIdentifier,
        operations: Vec<Operation>,
    ) -> ConstructionPreprocessRequest {
        ConstructionPreprocessRequest {
            network_identifier: network_identifier,
            operations: operations,
            metadata: None,
            max_fee: None,
            suggested_fee_multiplier: None,
        }
    }
}

/// ConstructionPreprocessResponse contains `options` that will be sent unmodified to `/construction/metadata`. If it is not necessary to make a request to `/construction/metadata`, `options` should be omitted.  Some blockchains require the PublicKey of particular AccountIdentifiers to construct a valid transaction. To fetch these PublicKeys, populate `required_public_keys` with the AccountIdentifiers associated with the desired PublicKeys. If it is not necessary to retrieve any PublicKeys for construction, `required_public_keys` should be omitted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct ConstructionPreprocessResponse {
    /// The options that will be sent directly to `/construction/metadata` by the caller.
    #[serde(rename = "options")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<GenericMetadata>,

    #[serde(rename = "required_public_keys")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_public_keys: Option<Vec<AccountIdentifier>>,
}

impl ConstructionPreprocessResponse {
    pub fn new() -> ConstructionPreprocessResponse {
        ConstructionPreprocessResponse {
            options: None,
            required_public_keys: None,
        }
    }
}

/// The transaction submission request includes a signed transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct ConstructionSubmitRequest {
    #[serde(rename = "network_identifier")]
    pub network_identifier: NetworkIdentifier,

    #[serde(rename = "signed_transaction")]
    pub signed_transaction: String,
}

impl ConstructionSubmitRequest {
    pub fn new(
        network_identifier: NetworkIdentifier,
        signed_transaction: String,
    ) -> ConstructionSubmitRequest {
        ConstructionSubmitRequest {
            network_identifier: network_identifier,
            signed_transaction: signed_transaction,
        }
    }
}

/// Currency is composed of a canonical Symbol and Decimals. This Decimals value is used to convert an Amount.Value from atomic units (Satoshis) to standard units (Bitcoins).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct Currency {
    /// Canonical symbol associated with a currency.
    #[serde(rename = "symbol")]
    pub symbol: String,

    /// Number of decimal places in the standard unit representation of the amount. For example, BTC has 8 decimals. Note that it is not possible to represent the value of some currency in atomic units that is not base 10.
    #[serde(rename = "decimals")]
    pub decimals: u32,

    /// Any additional information related to the currency itself. For example, it would be useful to populate this GenericMetadata with the contract address of an ERC-20 token.
    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenericMetadata>,
}

impl Currency {
    pub fn new(symbol: String, decimals: u32) -> Currency {
        Currency {
            symbol: symbol,
            decimals: decimals,
            metadata: None,
        }
    }
}

/// CurveType is the type of cryptographic curve associated with a PublicKey. * secp256k1: SEC compressed - `33 bytes` (https://secg.org/sec1-v2.pdf#subsubsection.2.3.3) * secp256r1: SEC compressed - `33 bytes` (https://secg.org/sec1-v2.pdf#subsubsection.2.3.3) * edwards25519: `y (255-bits) || x-sign-bit (1-bit)` - `32 bytes` (https://ed25519.cr.yp.to/ed25519-20110926.pdf) * tweedle: 1st pk : Fq.t (32 bytes) || 2nd pk : Fq.t (32 bytes) (https://github.com/CodaProtocol/coda/blob/develop/rfcs/0038-rosetta-construction-api.md#marshal-keys) * pallas: `x (255 bits) || y-parity-bit (1-bit) - 32 bytes` (https://github.com/zcash/pasta)
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGenericEnum))]
pub enum CurveType {
    #[serde(rename = "secp256k1")]
    SECP256K1,
    #[serde(rename = "secp256r1")]
    SECP256R1,
    #[serde(rename = "edwards25519")]
    EDWARDS25519,
    #[serde(rename = "tweedle")]
    TWEEDLE,
    #[serde(rename = "pallas")]
    PALLAS,
}

impl ::std::fmt::Display for CurveType {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            CurveType::SECP256K1 => write!(f, "{}", "secp256k1"),
            CurveType::SECP256R1 => write!(f, "{}", "secp256r1"),
            CurveType::EDWARDS25519 => write!(f, "{}", "edwards25519"),
            CurveType::TWEEDLE => write!(f, "{}", "tweedle"),
            CurveType::PALLAS => write!(f, "{}", "pallas"),
        }
    }
}

impl ::std::str::FromStr for CurveType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "secp256k1" => Ok(CurveType::SECP256K1),
            "secp256r1" => Ok(CurveType::SECP256R1),
            "edwards25519" => Ok(CurveType::EDWARDS25519),
            "tweedle" => Ok(CurveType::TWEEDLE),
            "pallas" => Ok(CurveType::PALLAS),
            _ => Err(()),
        }
    }
}

/// Used by RelatedTransaction to indicate the direction of the relation (i.e. cross-shard/cross-network sends may reference `backward` to an earlier transaction and async execution may reference `forward`). Can be used to indicate if a transaction relation is from child to parent or the reverse.
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGenericEnum))]
pub enum Direction {
    #[serde(rename = "forward")]
    FORWARD,
    #[serde(rename = "backward")]
    BACKWARD,
}

impl ::std::fmt::Display for Direction {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            Direction::FORWARD => write!(f, "{}", "forward"),
            Direction::BACKWARD => write!(f, "{}", "backward"),
        }
    }
}

impl ::std::str::FromStr for Direction {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "forward" => Ok(Direction::FORWARD),
            "backward" => Ok(Direction::BACKWARD),
            _ => Err(()),
        }
    }
}

/// Instead of utilizing HTTP status codes to describe node errors (which often do not have a good analog), rich errors are returned using this GenericMetadata. Both the code and message fields can be individually used to correctly identify an error. Implementations MUST use unique values for both fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct Error {
    /// Code is a network-specific error code. If desired, this code can be equivalent to an HTTP status code.
    #[serde(rename = "code")]
    pub code: u32,

    /// Message is a network-specific error message. The message MUST NOT change for a given code. In particular, this means that any contextual information should be included in the details field.
    #[serde(rename = "message")]
    pub message: String,

    /// Description allows the implementer to optionally provide additional information about an error. In many cases, the content of this field will be a copy-and-paste from existing developer documentation. Description can ONLY be populated with generic information about a particular type of error. It MUST NOT be populated with information about a particular instantiation of an error (use `details` for this). Whereas the content of Error.Message should stay stable across releases, the content of Error.Description will likely change across releases (as implementers improve error documentation). For this reason, the content in this field is not part of any type assertion (unlike Error.Message).
    #[serde(rename = "description")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// An error is retriable if the same request may succeed if submitted again.
    #[serde(rename = "retriable")]
    pub retriable: bool,

    /// Often times it is useful to return context specific to the request that caused the error (i.e. a sample of the stack trace or impacted account) in addition to the standard error message.
    #[serde(rename = "details")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<ErrorInfo>,
}

impl Error {
    pub fn new(code: u32, message: String, retriable: bool) -> Error {
        Error {
            code: code,
            message: message,
            description: None,
            retriable: retriable,
            details: None,
        }
    }
}

/// EventsBlocksRequest is utilized to fetch a sequence of BlockEvents indicating which blocks were added and removed from storage to reach the current state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct EventsBlocksRequest {
    #[serde(rename = "network_identifier")]
    pub network_identifier: NetworkIdentifier,

    /// offset is the offset into the event stream to sync events from. If this field is not populated, we return the limit events backwards from tip. If this is set to 0, we start from the beginning.
    #[serde(rename = "offset")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,

    /// limit is the maximum number of events to fetch in one call. The implementation may return <= limit events.
    #[serde(rename = "limit")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
}

impl EventsBlocksRequest {
    pub fn new(network_identifier: NetworkIdentifier) -> EventsBlocksRequest {
        EventsBlocksRequest {
            network_identifier: network_identifier,
            offset: None,
            limit: None,
        }
    }
}

/// EventsBlocksResponse contains an ordered collection of BlockEvents and the max retrievable sequence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct EventsBlocksResponse {
    /// max_sequence is the maximum available sequence number to fetch.
    #[serde(rename = "max_sequence")]
    pub max_sequence: i64,

    /// events is an array of BlockEvents indicating the order to add and remove blocks to maintain a canonical view of blockchain state. Lightweight clients can use this event stream to update state without implementing their own block syncing logic.
    #[serde(rename = "events")]
    pub events: Vec<BlockEvent>,
}

impl EventsBlocksResponse {
    pub fn new(max_sequence: i64, events: Vec<BlockEvent>) -> EventsBlocksResponse {
        EventsBlocksResponse {
            max_sequence: max_sequence,
            events: events,
        }
    }
}

/// ExemptionType is used to indicate if the live balance for an account subject to a BalanceExemption could increase above, decrease below, or equal the computed balance. * greater_or_equal: The live balance may increase above or equal the computed balance. This typically   occurs with staking rewards that accrue on each block. * less_or_equal: The live balance may decrease below or equal the computed balance. This typically   occurs as balance moves from locked to spendable on a vesting account. * dynamic: The live balance may increase above, decrease below, or equal the computed balance. This   typically occurs with tokens that have a dynamic supply.
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGenericEnum))]
pub enum ExemptionType {
    #[serde(rename = "greater_or_equal")]
    GREATER_OR_EQUAL,
    #[serde(rename = "less_or_equal")]
    LESS_OR_EQUAL,
    #[serde(rename = "dynamic")]
    DYNAMIC,
}

impl ::std::fmt::Display for ExemptionType {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            ExemptionType::GREATER_OR_EQUAL => write!(f, "{}", "greater_or_equal"),
            ExemptionType::LESS_OR_EQUAL => write!(f, "{}", "less_or_equal"),
            ExemptionType::DYNAMIC => write!(f, "{}", "dynamic"),
        }
    }
}

impl ::std::str::FromStr for ExemptionType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "greater_or_equal" => Ok(ExemptionType::GREATER_OR_EQUAL),
            "less_or_equal" => Ok(ExemptionType::LESS_OR_EQUAL),
            "dynamic" => Ok(ExemptionType::DYNAMIC),
            _ => Err(()),
        }
    }
}

/// A MempoolResponse contains all transaction identifiers in the mempool for a particular network_identifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct MempoolResponse {
    #[serde(rename = "transaction_identifiers")]
    pub transaction_identifiers: Vec<TransactionIdentifier>,
}

impl MempoolResponse {
    pub fn new(transaction_identifiers: Vec<TransactionIdentifier>) -> MempoolResponse {
        MempoolResponse {
            transaction_identifiers: transaction_identifiers,
        }
    }
}

/// A MempoolTransactionRequest is utilized to retrieve a transaction from the mempool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct MempoolTransactionRequest {
    #[serde(rename = "network_identifier")]
    pub network_identifier: NetworkIdentifier,

    #[serde(rename = "transaction_identifier")]
    pub transaction_identifier: TransactionIdentifier,
}

impl MempoolTransactionRequest {
    pub fn new(
        network_identifier: NetworkIdentifier,
        transaction_identifier: TransactionIdentifier,
    ) -> MempoolTransactionRequest {
        MempoolTransactionRequest {
            network_identifier: network_identifier,
            transaction_identifier: transaction_identifier,
        }
    }
}

/// A MempoolTransactionResponse contains an estimate of a mempool transaction. It may not be possible to know the full impact of a transaction in the mempool (ex: fee paid).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct MempoolTransactionResponse {
    #[serde(rename = "transaction")]
    pub transaction: Transaction,

    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenericMetadata>,
}

impl MempoolTransactionResponse {
    pub fn new(transaction: Transaction) -> MempoolTransactionResponse {
        MempoolTransactionResponse {
            transaction: transaction,
            metadata: None,
        }
    }
}

/// A MetadataRequest is utilized in any request where the only argument is optional metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct MetadataRequest {
    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenericMetadata>,
}

impl MetadataRequest {
    pub fn new() -> MetadataRequest {
        MetadataRequest { metadata: None }
    }
}

/// The network_identifier specifies which network a particular GenericMetadata is associated with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct NetworkIdentifier {
    #[serde(rename = "blockchain")]
    pub blockchain: String,

    /// If a blockchain has a specific chain-id or network identifier, it should go in this field. It is up to the client to determine which network-specific identifier is mainnet or testnet.
    #[serde(rename = "network")]
    pub network: String,

    #[serde(rename = "sub_network_identifier")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_network_identifier: Option<SubNetworkIdentifier>,
}

impl NetworkIdentifier {
    pub fn new(blockchain: String, network: String) -> NetworkIdentifier {
        NetworkIdentifier {
            blockchain: blockchain,
            network: network,
            sub_network_identifier: None,
        }
    }
}

/// A NetworkListResponse contains all NetworkIdentifiers that the node can serve information for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct NetworkListResponse {
    #[serde(rename = "network_identifiers")]
    pub network_identifiers: Vec<NetworkIdentifier>,
}

impl NetworkListResponse {
    pub fn new(network_identifiers: Vec<NetworkIdentifier>) -> NetworkListResponse {
        NetworkListResponse {
            network_identifiers: network_identifiers,
        }
    }
}

/// NetworkOptionsResponse contains information about the versioning of the node and the allowed operation statuses, operation types, and errors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct NetworkOptionsResponse {
    #[serde(rename = "version")]
    pub version: Version,

    #[serde(rename = "allow")]
    pub allow: Allow,
}

impl NetworkOptionsResponse {
    pub fn new(version: Version, allow: Allow) -> NetworkOptionsResponse {
        NetworkOptionsResponse {
            version: version,
            allow: allow,
        }
    }
}

/// A NetworkRequest is utilized to retrieve some data specific exclusively to a NetworkIdentifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct NetworkRequest {
    #[serde(rename = "network_identifier")]
    pub network_identifier: NetworkIdentifier,

    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenericMetadata>,
}

impl NetworkRequest {
    pub fn new(network_identifier: NetworkIdentifier) -> NetworkRequest {
        NetworkRequest {
            network_identifier: network_identifier,
            metadata: None,
        }
    }
}

/// NetworkStatusResponse contains basic information about the node's view of a blockchain network. It is assumed that any BlockIdentifier.Index less than or equal to CurrentBlockIdentifier.Index can be queried. If a Rosetta implementation prunes historical state, it should populate the optional `oldest_block_identifier` field with the oldest block available to query. If this is not populated, it is assumed that the `genesis_block_identifier` is the oldest queryable block. If a Rosetta implementation performs some pre-sync before it is possible to query blocks, sync_status should be populated so that clients can still monitor healthiness. Without this field, it may appear that the implementation is stuck syncing and needs to be terminated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct NetworkStatusResponse {
    #[serde(rename = "current_block_identifier")]
    pub current_block_identifier: BlockIdentifier,

    /// The timestamp of the block in milliseconds since the Unix Epoch. The timestamp is stored in milliseconds because some blockchains produce blocks more often than once a second.
    #[serde(rename = "current_block_timestamp")]
    pub current_block_timestamp: i64,

    #[serde(rename = "genesis_block_identifier")]
    pub genesis_block_identifier: BlockIdentifier,

    #[serde(rename = "oldest_block_identifier")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oldest_block_identifier: Option<BlockIdentifier>,

    #[serde(rename = "sync_status")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_status: Option<SyncStatus>,

    #[serde(rename = "peers")]
    pub peers: Vec<Peer>,
}

impl NetworkStatusResponse {
    pub fn new(
        current_block_identifier: BlockIdentifier,
        current_block_timestamp: i64,
        genesis_block_identifier: BlockIdentifier,
        peers: Vec<Peer>,
    ) -> NetworkStatusResponse {
        NetworkStatusResponse {
            current_block_identifier: current_block_identifier,
            current_block_timestamp: current_block_timestamp,
            genesis_block_identifier: genesis_block_identifier,
            oldest_block_identifier: None,
            sync_status: None,
            peers: peers,
        }
    }
}

/// Operations contain all balance-changing information within a transaction. They are always one-sided (only affect 1 AccountIdentifier) and can succeed or fail independently from a Transaction. Operations are used both to represent on-chain data (Data API) and to construct new transactions (Construction API), creating a standard interface for reading and writing to blockchains.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct Operation {
    #[serde(rename = "operation_identifier")]
    pub operation_identifier: OperationIdentifier,

    /// Restrict referenced related_operations to identifier indices < the current operation_identifier.index. This ensures there exists a clear DAG-structure of relations. Since operations are one-sided, one could imagine relating operations in a single transfer or linking operations in a call tree.
    #[serde(rename = "related_operations")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_operations: Option<Vec<OperationIdentifier>>,

    /// Type is the network-specific type of the operation. Ensure that any type that can be returned here is also specified in the NetworkOptionsResponse. This can be very useful to downstream consumers that parse all block data.
    #[serde(rename = "type")]
    pub _type: String,

    /// Status is the network-specific status of the operation. Status is not defined on the transaction GenericMetadata because blockchains with smart contracts may have transactions that partially apply (some operations are successful and some are not). Blockchains with atomic transactions (all operations succeed or all operations fail) will have the same status for each operation. On-chain operations (operations retrieved in the `/block` and `/block/transaction` endpoints) MUST have a populated status field (anything on-chain must have succeeded or failed). However, operations provided during transaction construction (often times called \"intent\" in the documentation) MUST NOT have a populated status field (operations yet to be included on-chain have not yet succeeded or failed).
    #[serde(rename = "status")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    #[serde(rename = "account")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<AccountIdentifier>,

    #[serde(rename = "amount")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<Amount>,

    #[serde(rename = "coin_change")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coin_change: Option<CoinChange>,

    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenericMetadata>,
}

impl Operation {
    pub fn new(operation_identifier: OperationIdentifier, _type: String) -> Operation {
        Operation {
            operation_identifier: operation_identifier,
            related_operations: None,
            _type: _type,
            status: None,
            account: None,
            amount: None,
            coin_change: None,
            metadata: None,
        }
    }
}

/// The operation_identifier uniquely identifies an operation within a transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct OperationIdentifier {
    /// The operation index is used to ensure each operation has a unique identifier within a transaction. This index is only relative to the transaction and NOT GLOBAL. The operations in each transaction should start from index 0. To clarify, there may not be any notion of an operation index in the blockchain being described.
    #[serde(rename = "index")]
    pub index: i64,

    /// Some blockchains specify an operation index that is essential for client use. For example, Bitcoin uses a network_index to identify which UTXO was used in a transaction. network_index should not be populated if there is no notion of an operation index in a blockchain (typically most account-based blockchains).
    #[serde(rename = "network_index")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_index: Option<i64>,
}

impl OperationIdentifier {
    pub fn new(index: i64) -> OperationIdentifier {
        OperationIdentifier {
            index: index,
            network_index: None,
        }
    }
}

/// OperationStatus is utilized to indicate which Operation status are considered successful.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct OperationStatus {
    /// The status is the network-specific status of the operation.
    #[serde(rename = "status")]
    pub status: String,

    /// An Operation is considered successful if the Operation.Amount should affect the Operation.Account. Some blockchains (like Bitcoin) only include successful operations in blocks but other blockchains (like Ethereum) include unsuccessful operations that incur a fee. To reconcile the computed balance from the stream of Operations, it is critical to understand which Operation.Status indicate an Operation is successful and should affect an Account.
    #[serde(rename = "successful")]
    pub successful: bool,
}

impl OperationStatus {
    pub fn new(status: String, successful: bool) -> OperationStatus {
        OperationStatus {
            status: status,
            successful: successful,
        }
    }
}

/// Operator is used by query-related endpoints to determine how to apply conditions. If this field is not populated, the default `and` value will be used.
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGenericEnum))]
pub enum Operator {
    #[serde(rename = "or")]
    OR,
    #[serde(rename = "and")]
    AND,
}

impl ::std::fmt::Display for Operator {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            Operator::OR => write!(f, "{}", "or"),
            Operator::AND => write!(f, "{}", "and"),
        }
    }
}

impl ::std::str::FromStr for Operator {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "or" => Ok(Operator::OR),
            "and" => Ok(Operator::AND),
            _ => Err(()),
        }
    }
}

/// When fetching data by BlockIdentifier, it may be possible to only specify the index or hash. If neither property is specified, it is assumed that the client is making a request at the current block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct PartialBlockIdentifier {
    #[serde(rename = "index")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i64>,

    #[serde(rename = "hash")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

impl PartialBlockIdentifier {
    pub fn new() -> PartialBlockIdentifier {
        PartialBlockIdentifier {
            index: None,
            hash: None,
        }
    }
}

/// A Peer is a representation of a node's peer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct Peer {
    #[serde(rename = "peer_id")]
    pub peer_id: String,

    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenericMetadata>,
}

impl Peer {
    pub fn new(peer_id: String) -> Peer {
        Peer {
            peer_id: peer_id,
            metadata: None,
        }
    }
}

/// PublicKey contains a public key byte array for a particular CurveType encoded in hex. Note that there is no PrivateKey struct as this is NEVER the concern of an implementation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct PublicKey {
    /// Hex-encoded public key bytes in the format specified by the CurveType.
    #[serde(rename = "hex_bytes")]
    pub hex_bytes: String,

    #[serde(rename = "curve_type")]
    pub curve_type: CurveType,
}

impl PublicKey {
    pub fn new(hex_bytes: String, curve_type: CurveType) -> PublicKey {
        PublicKey {
            hex_bytes: hex_bytes,
            curve_type: curve_type,
        }
    }
}

/// The related_transaction allows implementations to link together multiple transactions. An unpopulated network identifier indicates that the related transaction is on the same network.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct RelatedTransaction {
    #[serde(rename = "network_identifier")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_identifier: Option<NetworkIdentifier>,

    #[serde(rename = "transaction_identifier")]
    pub transaction_identifier: TransactionIdentifier,

    #[serde(rename = "direction")]
    pub direction: Direction,
}

impl RelatedTransaction {
    pub fn new(
        transaction_identifier: TransactionIdentifier,
        direction: Direction,
    ) -> RelatedTransaction {
        RelatedTransaction {
            network_identifier: None,
            transaction_identifier: transaction_identifier,
            direction: direction,
        }
    }
}

/// SearchTransactionsRequest is used to search for transactions matching a set of provided conditions in canonical blocks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct SearchTransactionsRequest {
    #[serde(rename = "network_identifier")]
    pub network_identifier: NetworkIdentifier,

    #[serde(rename = "operator")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<Operator>,

    /// max_block is the largest block index to consider when searching for transactions. If this field is not populated, the current block is considered the max_block. If you do not specify a max_block, it is possible a newly synced block will interfere with paginated transaction queries (as the offset could become invalid with newly added rows).
    #[serde(rename = "max_block")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_block: Option<i64>,

    /// offset is the offset into the query result to start returning transactions. If any search conditions are changed, the query offset will change and you must restart your search iteration.
    #[serde(rename = "offset")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,

    /// limit is the maximum number of transactions to return in one call. The implementation may return <= limit transactions.
    #[serde(rename = "limit")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,

    #[serde(rename = "transaction_identifier")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_identifier: Option<TransactionIdentifier>,

    #[serde(rename = "account_identifier")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_identifier: Option<AccountIdentifier>,

    #[serde(rename = "coin_identifier")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coin_identifier: Option<CoinIdentifier>,

    #[serde(rename = "currency")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<Currency>,

    /// status is the network-specific operation type.
    #[serde(rename = "status")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// type is the network-specific operation type.
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _type: Option<String>,

    /// address is AccountIdentifier.Address. This is used to get all transactions related to an AccountIdentifier.Address, regardless of SubAccountIdentifier.
    #[serde(rename = "address")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,

    /// success is a synthetic condition populated by parsing network-specific operation statuses (using the mapping provided in `/network/options`).
    #[serde(rename = "success")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
}

impl SearchTransactionsRequest {
    pub fn new(network_identifier: NetworkIdentifier) -> SearchTransactionsRequest {
        SearchTransactionsRequest {
            network_identifier: network_identifier,
            operator: None,
            max_block: None,
            offset: None,
            limit: None,
            transaction_identifier: None,
            account_identifier: None,
            coin_identifier: None,
            currency: None,
            status: None,
            _type: None,
            address: None,
            success: None,
        }
    }
}

/// SearchTransactionsResponse contains an ordered collection of BlockTransactions that match the query in SearchTransactionsRequest. These BlockTransactions are sorted from most recent block to oldest block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct SearchTransactionsResponse {
    /// transactions is an array of BlockTransactions sorted by most recent BlockIdentifier (meaning that transactions in recent blocks appear first). If there are many transactions for a particular search, transactions may not contain all matching transactions. It is up to the caller to paginate these transactions using the max_block field.
    #[serde(rename = "transactions")]
    pub transactions: Vec<BlockTransaction>,

    /// total_count is the number of results for a given search. Callers typically use this value to concurrently fetch results by offset or to display a virtual page number associated with results.
    #[serde(rename = "total_count")]
    pub total_count: i64,

    /// next_offset is the next offset to use when paginating through transaction results. If this field is not populated, there are no more transactions to query.
    #[serde(rename = "next_offset")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<i64>,
}

impl SearchTransactionsResponse {
    pub fn new(
        transactions: Vec<BlockTransaction>,
        total_count: i64,
    ) -> SearchTransactionsResponse {
        SearchTransactionsResponse {
            transactions: transactions,
            total_count: total_count,
            next_offset: None,
        }
    }
}

/// Signature contains the payload that was signed, the public keys of the keypairs used to produce the signature, the signature (encoded in hex), and the SignatureType. PublicKey is often times not known during construction of the signing payloads but may be needed to combine signatures properly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct Signature {
    #[serde(rename = "signing_payload")]
    pub signing_payload: SigningPayload,

    #[serde(rename = "public_key")]
    pub public_key: PublicKey,

    #[serde(rename = "signature_type")]
    pub signature_type: SignatureType,

    #[serde(rename = "hex_bytes")]
    pub hex_bytes: String,
}

impl Signature {
    pub fn new(
        signing_payload: SigningPayload,
        public_key: PublicKey,
        signature_type: SignatureType,
        hex_bytes: String,
    ) -> Signature {
        Signature {
            signing_payload: signing_payload,
            public_key: public_key,
            signature_type: signature_type,
            hex_bytes: hex_bytes,
        }
    }
}

/// SignatureType is the type of a cryptographic signature. * ecdsa: `r (32-bytes) || s (32-bytes)` - `64 bytes` * ecdsa_recovery: `r (32-bytes) || s (32-bytes) || v (1-byte)` - `65 bytes` * ed25519: `R (32-byte) || s (32-bytes)` - `64 bytes` * schnorr_1: `r (32-bytes) || s (32-bytes)` - `64 bytes`  (schnorr signature implemented by Zilliqa where both `r` and `s` are scalars encoded as `32-bytes` values, most significant byte first.) * schnorr_poseidon: `r (32-bytes) || s (32-bytes)` where s = Hash(1st pk || 2nd pk || r) - `64 bytes`  (schnorr signature w/ Poseidon hash function implemented by O(1) Labs where both `r` and `s` are scalars encoded as `32-bytes` values, least significant byte first. https://github.com/CodaProtocol/signer-reference/blob/master/schnorr.ml )
/// Enumeration of values.
/// Since this enum's variants do not hold data, we can easily define them them as `#[repr(C)]`
/// which helps with FFI.
#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGenericEnum))]
pub enum SignatureType {
    #[serde(rename = "ecdsa")]
    ECDSA,
    #[serde(rename = "ecdsa_recovery")]
    ECDSA_RECOVERY,
    #[serde(rename = "ed25519")]
    ED25519,
    #[serde(rename = "schnorr_1")]
    SCHNORR_1,
    #[serde(rename = "schnorr_poseidon")]
    SCHNORR_POSEIDON,
}

impl ::std::fmt::Display for SignatureType {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            SignatureType::ECDSA => write!(f, "{}", "ecdsa"),
            SignatureType::ECDSA_RECOVERY => write!(f, "{}", "ecdsa_recovery"),
            SignatureType::ED25519 => write!(f, "{}", "ed25519"),
            SignatureType::SCHNORR_1 => write!(f, "{}", "schnorr_1"),
            SignatureType::SCHNORR_POSEIDON => write!(f, "{}", "schnorr_poseidon"),
        }
    }
}

impl ::std::str::FromStr for SignatureType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ecdsa" => Ok(SignatureType::ECDSA),
            "ecdsa_recovery" => Ok(SignatureType::ECDSA_RECOVERY),
            "ed25519" => Ok(SignatureType::ED25519),
            "schnorr_1" => Ok(SignatureType::SCHNORR_1),
            "schnorr_poseidon" => Ok(SignatureType::SCHNORR_POSEIDON),
            _ => Err(()),
        }
    }
}

/// SigningPayload is signed by the client with the keypair associated with an AccountIdentifier using the specified SignatureType. SignatureType can be optionally populated if there is a restriction on the signature scheme that can be used to sign the payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct SigningPayload {
    /// [DEPRECATED by `account_identifier` in `v1.4.4`] The network-specific address of the account that should sign the payload.
    #[serde(rename = "address")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,

    #[serde(rename = "account_identifier")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_identifier: Option<AccountIdentifier>,

    /// Hex-encoded string of the payload bytes.
    #[serde(rename = "hex_bytes")]
    pub hex_bytes: String,

    #[serde(rename = "signature_type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_type: Option<SignatureType>,
}

impl SigningPayload {
    pub fn new(hex_bytes: String) -> SigningPayload {
        SigningPayload {
            address: None,
            account_identifier: None,
            hex_bytes: hex_bytes,
            signature_type: None,
        }
    }
}

/// An account may have state specific to a contract address (ERC-20 token) and/or a stake (delegated balance). The sub_account_identifier should specify which state (if applicable) an account instantiation refers to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct SubAccountIdentifier {
    /// The SubAccount address may be a cryptographic value or some other identifier (ex: bonded) that uniquely specifies a SubAccount.
    #[serde(rename = "address")]
    pub address: String,

    /// If the SubAccount address is not sufficient to uniquely specify a SubAccount, any other identifying information can be stored here. It is important to note that two SubAccounts with identical addresses but differing metadata will not be considered equal by clients.
    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenericMetadata>,
}

impl SubAccountIdentifier {
    pub fn new(address: String) -> SubAccountIdentifier {
        SubAccountIdentifier {
            address: address,
            metadata: None,
        }
    }
}

/// In blockchains with sharded state, the SubNetworkIdentifier is required to query some GenericMetadata on a specific shard. This identifier is optional for all non-sharded blockchains.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct SubNetworkIdentifier {
    #[serde(rename = "network")]
    pub network: String,

    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenericMetadata>,
}

impl SubNetworkIdentifier {
    pub fn new(network: String) -> SubNetworkIdentifier {
        SubNetworkIdentifier {
            network: network,
            metadata: None,
        }
    }
}

/// SyncStatus is used to provide additional context about an implementation's sync status. This GenericMetadata is often used by implementations to indicate healthiness when block data cannot be queried until some sync phase completes or cannot be determined by comparing the timestamp of the most recent block with the current time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct SyncStatus {
    /// CurrentIndex is the index of the last synced block in the current stage. This is a separate field from current_block_identifier in NetworkStatusResponse because blocks with indices up to and including the current_index may not yet be queryable by the caller. To reiterate, all indices up to and including current_block_identifier in NetworkStatusResponse must be queryable via the /block endpoint (excluding indices less than oldest_block_identifier).
    #[serde(rename = "current_index")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_index: Option<i64>,

    /// TargetIndex is the index of the block that the implementation is attempting to sync to in the current stage.
    #[serde(rename = "target_index")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_index: Option<i64>,

    /// Stage is the phase of the sync process.
    #[serde(rename = "stage")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,

    /// synced is a boolean that indicates if an implementation has synced up to the most recent block. If this field is not populated, the caller should rely on a traditional tip timestamp comparison to determine if an implementation is synced. This field is particularly useful for quiescent blockchains (blocks only produced when there are pending transactions). In these blockchains, the most recent block could have a timestamp far behind the current time but the node could be healthy and at tip.
    #[serde(rename = "synced")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synced: Option<bool>,
}

impl SyncStatus {
    pub fn new() -> SyncStatus {
        SyncStatus {
            current_index: None,
            target_index: None,
            stage: None,
            synced: None,
        }
    }
}

/// The timestamp of the block in milliseconds since the Unix Epoch. The timestamp is stored in milliseconds because some blockchains produce blocks more often than once a second.
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct Timestamp(i64);

impl ::std::convert::From<i64> for Timestamp {
    fn from(x: i64) -> Self {
        Timestamp(x)
    }
}

impl ::std::convert::From<Timestamp> for i64 {
    fn from(x: Timestamp) -> Self {
        x.0
    }
}

impl ::std::ops::Deref for Timestamp {
    type Target = i64;
    fn deref(&self) -> &i64 {
        &self.0
    }
}

impl ::std::ops::DerefMut for Timestamp {
    fn deref_mut(&mut self) -> &mut i64 {
        &mut self.0
    }
}

/// Transactions contain an array of Operations that are attributable to the same TransactionIdentifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct Transaction {
    #[serde(rename = "transaction_identifier")]
    pub transaction_identifier: TransactionIdentifier,

    #[serde(rename = "operations")]
    pub operations: Vec<Operation>,

    #[serde(rename = "related_transactions")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_transactions: Option<Vec<RelatedTransaction>>,

    /// Transactions that are related to other transactions (like a cross-shard transaction) should include the tranaction_identifier of these transactions in the metadata.
    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenericMetadata>,
}

impl Transaction {
    pub fn new(
        transaction_identifier: TransactionIdentifier,
        operations: Vec<Operation>,
    ) -> Transaction {
        Transaction {
            transaction_identifier: transaction_identifier,
            operations: operations,
            related_transactions: None,
            metadata: None,
        }
    }
}

/// The transaction_identifier uniquely identifies a transaction in a particular network and block or in the mempool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct TransactionIdentifier {
    /// Any transactions that are attributable only to a block (ex: a block event) should use the hash of the block as the identifier.  This should be normalized according to the case specified in the transaction_hash_case in network options.
    #[serde(rename = "hash")]
    pub hash: String,
}

impl TransactionIdentifier {
    pub fn new(hash: String) -> TransactionIdentifier {
        TransactionIdentifier { hash: hash }
    }
}

/// TransactionIdentifierResponse contains the transaction_identifier of a transaction that was submitted to either `/construction/hash` or `/construction/submit`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct TransactionIdentifierResponse {
    #[serde(rename = "transaction_identifier")]
    pub transaction_identifier: TransactionIdentifier,

    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenericMetadata>,
}

impl TransactionIdentifierResponse {
    pub fn new(transaction_identifier: TransactionIdentifier) -> TransactionIdentifierResponse {
        TransactionIdentifierResponse {
            transaction_identifier: transaction_identifier,
            metadata: None,
        }
    }
}

/// The Version GenericMetadata is utilized to inform the client of the versions of different components of the Rosetta implementation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "conversion", derive(LabelledGeneric))]
pub struct Version {
    /// The rosetta_version is the version of the Rosetta interface the implementation adheres to. This can be useful for clients looking to reliably parse responses.
    #[serde(rename = "rosetta_version")]
    pub rosetta_version: String,

    /// The node_version is the canonical version of the node runtime. This can help clients manage deployments.
    #[serde(rename = "node_version")]
    pub node_version: String,

    /// When a middleware server is used to adhere to the Rosetta interface, it should return its version here. This can help clients manage deployments.
    #[serde(rename = "middleware_version")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub middleware_version: Option<String>,

    /// Any other information that may be useful about versioning of dependent services should be returned here.
    #[serde(rename = "metadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GenericMetadata>,
}

impl Version {
    pub fn new(rosetta_version: String, node_version: String) -> Version {
        Version {
            rosetta_version: rosetta_version,
            node_version: node_version,
            middleware_version: None,
            metadata: None,
        }
    }
}
