//
// use evm::{ExitReason, ExitRevert, ExitSucceed};
// // use fp_ethereum::ValidatedTransaction;
// // use frame_support::{
// //     dispatch::{DispatchClass, GetDispatchInfo},
// //     weights::Weight,
// // };
// use pallet_evm::AddressMapping;
//
//
// fn eip2930_erc20_creation_unsigned_transaction() -> EIP2930UnsignedTransaction {
//     EIP2930UnsignedTransaction {
//         nonce: U256::zero(),
//         gas_price: U256::from(1),
//         gas_limit: U256::from(0x100000),
//         action: ethereum::TransactionAction::Create,
//         value: U256::zero(),
//         input: hex::decode(ERC20_CONTRACT_BYTECODE.trim_end()).unwrap(),
//     }
// }

use std::collections::{BTreeMap, BTreeSet};
use std::marker::PhantomData;
use std::mem;
use evm::backend::{Backend, Log};
use evm::executor::stack::{Accessed, PrecompileHandle, PrecompileSet, StackExecutor, StackState, StackSubstateMetadata};
use evm::{Config, ExitError, Transfer};
use parity_scale_codec::{Decode, Encode};
use primitive_types::{H160, H256, U256};
use serde::{Deserialize, Serialize};


// #[proc_macro_derive(TypeInfo, attributes(scale_info, codec))]
// pub fn type_info(input: TokenStream) -> TokenStream {
//     match generate(input.into()) {
//         Ok(output) => output.into(),
//         Err(err) => err.to_compile_error().into(),
//     }
// }


#[derive(Clone, Copy, Eq, PartialEq, Debug, Encode, Decode)] //TypeInfo
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WeightInfo {
    pub ref_time_limit: Option<u64>,
    pub proof_size_limit: Option<u64>,
    pub ref_time_usage: Option<u64>,
    pub proof_size_usage: Option<u64>,
}

#[derive(Clone, Eq, PartialEq, Default, Debug, Encode, Decode)]
#[derive(Serialize, Deserialize)]
/// External input from the transaction.
pub struct Vicinity {
    /// Current transaction gas price.
    pub gas_price: U256,
    /// Origin of the transaction.
    pub origin: H160,
}

/// Substrate backend for EVM.
pub struct SubstrateStackState<'vicinity, 'config
    //,T: Config
> {
    vicinity: &'vicinity Vicinity,
    substate: SubstrateStackSubstate<'config>,
    original_storage: BTreeMap<(H160, H256), H256>,
    recorded: Recorded,
    weight_info: Option<WeightInfo>,
    // _marker: PhantomData<T>,
}

impl<'vicinity, 'config,
    // T: Config
> SubstrateStackState<'vicinity, 'config
    // , T
> {
    /// Create a new backend with given vicinity.
    pub fn new(
        vicinity: &'vicinity Vicinity,
        metadata: StackSubstateMetadata<'config>,
        weight_info: Option<WeightInfo>,
    ) -> Self {
        Self {
            vicinity,
            substate: SubstrateStackSubstate {
                metadata,
                deletes: BTreeSet::new(),
                logs: Vec::new(),
                parent: None,
            },
            // _marker: PhantomData,
            original_storage: BTreeMap::new(),
            recorded: Default::default(),
            weight_info,
        }
    }

    pub fn weight_info(&self) -> Option<WeightInfo> {
        self.weight_info
    }

    pub fn recorded(&self) -> &Recorded {
        &self.recorded
    }

    pub fn info_mut(&mut self) -> (&mut Option<WeightInfo>, &mut Recorded) {
        (&mut self.weight_info, &mut self.recorded)
    }
}


struct SubstrateStackSubstate<'config> {
    metadata: StackSubstateMetadata<'config>,
    deletes: BTreeSet<H160>,
    logs: Vec<Log>,
    parent: Option<Box<SubstrateStackSubstate<'config>>>,
}

impl<'config> SubstrateStackSubstate<'config> {
    pub fn metadata(&self) -> &StackSubstateMetadata<'config> {
        &self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut StackSubstateMetadata<'config> {
        &mut self.metadata
    }

    pub fn enter(&mut self, gas_limit: u64, is_static: bool) {
        let mut entering = Self {
            metadata: self.metadata.spit_child(gas_limit, is_static),
            parent: None,
            deletes: BTreeSet::new(),
            logs: Vec::new(),
        };
        mem::swap(&mut entering, self);

        self.parent = Some(Box::new(entering));
        println!("Entering transaction")
        // sp_io::storage::start_transaction();
    }

    pub fn exit_commit(&mut self) -> Result<(), ExitError> {
        let mut exited = *self.parent.take().expect("Cannot commit on root substate");
        mem::swap(&mut exited, self);

        self.metadata.swallow_commit(exited.metadata)?;
        self.logs.append(&mut exited.logs);
        self.deletes.append(&mut exited.deletes);

        // sp_io::storage::commit_transaction();
        println!("Committing transaction");
        Ok(())
    }

    pub fn exit_revert(&mut self) -> Result<(), ExitError> {
        let mut exited = *self.parent.take().expect("Cannot discard on root substate");
        mem::swap(&mut exited, self);
        self.metadata.swallow_revert(exited.metadata)?;

        println!("Rollback transaction");
        // sp_io::storage::rollback_transaction();
        Ok(())
    }

    pub fn exit_discard(&mut self) -> Result<(), ExitError> {
        let mut exited = *self.parent.take().expect("Cannot discard on root substate");
        mem::swap(&mut exited, self);
        self.metadata.swallow_discard(exited.metadata)?;
        println!("Rollback transaction");
        // sp_io::storage::rollback_transaction();
        Ok(())
    }

    pub fn deleted(&self, address: H160) -> bool {
        if self.deletes.contains(&address) {
            return true;
        }

        if let Some(parent) = self.parent.as_ref() {
            return parent.deleted(address);
        }

        false
    }

    pub fn set_deleted(&mut self, address: H160) {
        self.deletes.insert(address);
    }

    pub fn log(&mut self, address: H160, topics: Vec<H256>, data: Vec<u8>) {
        self.logs.push(Log {
            address,
            topics,
            data,
        });
    }

    fn recursive_is_cold<F: Fn(&Accessed) -> bool>(&self, f: &F) -> bool {
        let local_is_accessed = self.metadata.accessed().as_ref().map(f).unwrap_or(false);
        if local_is_accessed {
            false
        } else {
            self.parent
                .as_ref()
                .map(|p| p.recursive_is_cold(f))
                .unwrap_or(true)
        }
    }
}

#[derive(Default, Clone, Eq, PartialEq)]
pub struct Recorded {
    account_codes: Vec<H160>,
    account_storages: BTreeMap<(H160, H256), bool>,
}



impl<'vicinity, 'config,
    // T: Config
> Backend for SubstrateStackState<'vicinity, 'config
    // , T
>
    // where
        // BalanceOf<T>: TryFrom<U256> + Into<U256>,
{
    fn gas_price(&self) -> U256 {
        self.vicinity.gas_price
    }
    fn origin(&self) -> H160 {
        self.vicinity.origin
    }

    fn block_hash(&self, number: U256) -> H256 {
        // if number > U256::from(u32::MAX) {
            H256::default()
        // } else {
        //     T::BlockHashMapping::block_hash(number.as_u32())
        // }
    }

    fn block_number(&self) -> U256 {
        // let number: u128 = frame_system::Pallet::<T>::block_number().unique_saturated_into();
        U256::from(1) //number)
    }

    fn block_coinbase(&self) -> H160 {

        H160::zero()
        // Pallet::<T>::find_author()
    }

    fn block_timestamp(&self) -> U256 {
        let now: u128 = 1000; //T::Timestamp::now().unique_saturated_into();
        U256::from(now / 1000)
    }

    fn block_difficulty(&self) -> U256 {
        U256::zero()
    }

    fn block_randomness(&self) -> Option<H256> {
        None
    }

    fn block_gas_limit(&self) -> U256 {
        U256::exp10(1000)
        // T::BlockGasLimit::get()
    }

    fn block_base_fee_per_gas(&self) -> U256 {
        // let (base_fee, _) = T::FeeCalculator::min_gas_price();
        // base_fee
        U256::one()
    }

    fn chain_id(&self) -> U256 {
        // U256::from(T::ChainId::get())
        U256::zero()
    }

    fn exists(&self, _address: H160) -> bool {
        true
    }

    fn basic(&self, address: H160) -> evm::backend::Basic {
        // let (account, _) = Pallet::<T>::account_basic(&address);

        evm::backend::Basic {
            balance: U256::exp10(10), //account.balance,
            nonce: U256::zero() //account.nonce,
        }
    }

    fn code(&self, address: H160) -> Vec<u8> {
        // TODO Might need to do this
        vec![]
        // <AccountCodes<T>>::get(address)
    }

    fn storage(&self, address: H160, index: H256) -> H256 {
        H256::zero()
        // <AccountStorages<T>>::get(address, index)
    }

    fn original_storage(&self, address: H160, index: H256) -> Option<H256> {
        Some(
            self.original_storage
                .get(&(address, index))
                .cloned()
                .unwrap_or_else(|| self.storage(address, index)),
        )
    }
}

impl<'vicinity, 'config,
    // T: Config
> StackState<'config>
for SubstrateStackState<'vicinity, 'config
    // , T
>
    // where
    //     BalanceOf<T>: TryFrom<U256> + Into<U256>,
{
    fn metadata(&self) -> &StackSubstateMetadata<'config> {
        self.substate.metadata()
    }

    fn metadata_mut(&mut self) -> &mut StackSubstateMetadata<'config> {
        self.substate.metadata_mut()
    }

    fn enter(&mut self, gas_limit: u64, is_static: bool) {
        self.substate.enter(gas_limit, is_static)
    }

    fn exit_commit(&mut self) -> Result<(), ExitError> {
        self.substate.exit_commit()
    }

    fn exit_revert(&mut self) -> Result<(), ExitError> {
        self.substate.exit_revert()
    }

    fn exit_discard(&mut self) -> Result<(), ExitError> {
        self.substate.exit_discard()
    }

    fn is_empty(&self, address: H160) -> bool {
        true //} //Pallet::<T>::is_account_empty(&address)
    }

    fn deleted(&self, address: H160) -> bool {
        self.substate.deleted(address)
    }

    fn inc_nonce(&mut self, address: H160) -> Result<(), ExitError> {
        // let account_id = T::AddressMapping::into_account_id(address);
        // frame_system::Pallet::<T>::inc_account_nonce(&account_id);
        Ok(())
    }

    fn set_storage(&mut self, address: H160, index: H256, value: H256) {
        // We cache the current value if this is the first time we modify it
        // in the transaction.
        // use sp_std::collections::btree_map::Entry::Vacant;
        // if let Vacant(e) = self.original_storage.entry((address, index)) {
        //     let original = <AccountStorages<T>>::get(address, index);
        //     // No need to cache if same value.
        //     if original != value {
        //         e.insert(original);
        //     }
        // }
        //
        // // Then we insert or remove the entry based on the value.
        // if value == H256::default() {
        //     log::debug!(
		// 		target: "evm",
		// 		"Removing storage for {:?} [index: {:?}]",
		// 		address,
		// 		index,
		// 	);
        //     <AccountStorages<T>>::remove(address, index);
        // } else {
        //     log::debug!(
		// 		target: "evm",
		// 		"Updating storage for {:?} [index: {:?}, value: {:?}]",
		// 		address,
		// 		index,
		// 		value,
		// 	);
        //     <AccountStorages<T>>::insert(address, index, value);
        // }
    }

    fn reset_storage(&mut self, address: H160) {
        // #[allow(deprecated)]
        //     let _ = <AccountStorages<T>>::remove_prefix(address, None);
    }

    fn log(&mut self, address: H160, topics: Vec<H256>, data: Vec<u8>) {
        self.substate.log(address, topics, data)
    }

    fn set_deleted(&mut self, address: H160) {
        self.substate.set_deleted(address)
    }

    fn set_code(&mut self, address: H160, code: Vec<u8>) {
        // log::debug!(
		// 	target: "evm",
		// 	"Inserting code ({} bytes) at {:?}",
		// 	code.len(),
		// 	address
		// );
        // Pallet::<T>::create_account(address, code);
    }

    fn transfer(&mut self, transfer: Transfer) -> Result<(), ExitError> {
        // let source = T::AddressMapping::into_account_id(transfer.source);
        // let target = T::AddressMapping::into_account_id(transfer.target);
        // T::Currency::transfer(
        //     &source,
        //     &target,
        //     transfer
        //         .value
        //         .try_into()
        //         .map_err(|_| ExitError::OutOfFund)?,
        //     ExistenceRequirement::AllowDeath,
        // )
        //     .map_err(|_| ExitError::OutOfFund)
        Ok(())
    }

    fn reset_balance(&mut self, _address: H160) {
        // Do nothing on reset balance in Substrate.
        //
        // This function exists in EVM because a design issue
        // (arguably a bug) in SELFDESTRUCT that can cause total
        // issuance to be reduced. We do not need to replicate this.
    }

    fn touch(&mut self, _address: H160) {
        // Do nothing on touch in Substrate.
        //
        // EVM pallet considers all accounts to exist, and distinguish
        // only empty and non-empty accounts. This avoids many of the
        // subtle issues in EIP-161.
    }

    fn is_cold(&self, address: H160) -> bool {
        self.substate
            .recursive_is_cold(&|a| a.accessed_addresses.contains(&address))
    }

    fn is_storage_cold(&self, address: H160, key: H256) -> bool {
        self.substate
            .recursive_is_cold(&|a: &Accessed| a.accessed_storage.contains(&(address, key)))
    }

    fn code_size(&self, address: H160) -> U256 {
        U256::zero()
        // U256::from(<Pallet<T>>::account_code_metadata(address).size)
    }

    fn code_hash(&self, address: H160) -> H256 {
        H256::zero()
        // <Pallet<T>>::account_code_metadata(address).hash
    }
}

pub const ERC20_CONTRACT_BYTECODE: &str = include_str!("res/erc20_contract_bytecode.txt");
pub const GREETER_CONTRACT_BYTECODE: &str = include_str!("res/greeter.txt");

#[test]
fn debug(){

    let config = Config{
        gas_ext_code: 0,
        gas_ext_code_hash: 0,
        gas_sstore_set: 0,
        gas_sstore_reset: 0,
        refund_sstore_clears: 0,
        max_refund_quotient: 0,
        gas_balance: 0,
        gas_sload: 0,
        gas_sload_cold: 0,
        gas_suicide: 0,
        gas_suicide_new_account: 0,
        gas_call: 0,
        gas_expbyte: 0,
        gas_transaction_create: 0,
        gas_transaction_call: 0,
        gas_transaction_zero_data: 0,
        gas_transaction_non_zero_data: 0,
        gas_access_list_address: 0,
        gas_access_list_storage_key: 0,
        gas_account_access_cold: 0,
        gas_storage_read_warm: 0,
        sstore_gas_metering: false,
        sstore_revert_under_stipend: false,
        increase_state_access_gas: false,
        decrease_clears_refund: false,
        disallow_executable_format: false,
        warm_coinbase_address: false,
        err_on_call_with_more_gas: false,
        call_l64_after_gas: false,
        empty_considered_exists: false,
        create_increase_nonce: false,
        stack_limit: 1000,
        memory_limit: 1000,
        call_stack_limit: 1000,
        create_contract_limit: None,
        max_initcode_size: None,
        call_stipend: 0,
        has_delegate_call: false,
        has_create2: false,
        has_revert: false,
        has_return_data: false,
        has_bitwise_shifting: false,
        has_chain_id: false,
        has_self_balance: false,
        has_ext_code_hash: false,
        has_base_fee: false,
        has_push0: false,
        estimate: false,
    };
    let gas_limit: u64 = 1e5 as u64;
    let maybe_weight_info = None;

    // Execute the EVM call.
    let vicinity = Vicinity {
        gas_price: U256::one(),
        origin: H160::zero(),
    };

    let precompiles = ();
    let metadata = StackSubstateMetadata::new(gas_limit, &config);
    let state = SubstrateStackState::new(
        &vicinity, metadata, maybe_weight_info);
    let mut executor = StackExecutor::
    new_with_precompiles(state, &config, &precompiles);
    let vec1 = hex::decode(ERC20_CONTRACT_BYTECODE).unwrap();
    let (exit_reason, vec) = executor.transact_create(
        H160::zero(),
        U256::one(),
        vec1,
        gas_limit,
        vec![]
    );
    println!("exit_reason: {:?}", exit_reason);
    println!("output data?: {}", hex::encode(vec));
    // executor.execute()


}