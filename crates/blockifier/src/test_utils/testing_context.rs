mod erc20_factory;
mod signers;
mod state_factory;
mod string_utils;
mod test_event;
mod yas_erc20_factory;
mod yas_factory;
mod yas_faucet_factory;

use std::collections::HashMap;
use std::sync::Arc;

use cairo_native::starknet::SyscallResult;
pub use erc20_factory::*;
pub use signers::*;
use starknet_api::block::BlockTimestamp;
use starknet_api::core::{calculate_contract_address, ClassHash, ContractAddress};
use starknet_api::hash::StarkFelt;
use starknet_api::transaction::{Calldata, ContractAddressSalt};
use starknet_types_core::felt::Felt;
pub use state_factory::*;
pub use test_event::*;
pub use yas_erc20_factory::*;
pub use yas_factory::*;
pub use yas_faucet_factory::*;

use crate::abi::abi_utils::selector_from_name;
use crate::block_context::BlockContext;
use crate::execution::call_info::CallInfo;
use crate::execution::common_hints::ExecutionMode;
use crate::execution::contract_class::ContractClass;
use crate::execution::entry_point::{
    CallEntryPoint, ConstructorContext, EntryPointExecutionContext,
};
use crate::execution::execution_utils::execute_deployment;
use crate::execution::sierra_utils::{felt_to_starkfelt, starkfelt_to_felt};
use crate::execution::syscalls::hint_processor::{
    FAILED_TO_CALCULATE_CONTRACT_ADDRESS, FAILED_TO_EXECUTE_CALL,
};
use crate::state::cached_state::CachedState;
use crate::state::state_api::State;
use crate::test_utils::dict_state_reader::DictStateReader;
use crate::test_utils::external_entry_point;
use crate::transaction::objects::AccountTransactionContext;

pub fn create_custom_deploy_test_state(
    address_to_class_hash: Vec<(ContractAddress, ClassHash)>,
    class_hash_to_class: Vec<(ClassHash, ContractClass)>,
) -> CachedState<DictStateReader> {
    CachedState::from(DictStateReader {
        address_to_class_hash: HashMap::from_iter(address_to_class_hash),
        class_hash_to_class: HashMap::from_iter(class_hash_to_class),
        ..Default::default()
    })
}

pub struct TestContext {
    pub contract_addresses: HashMap<&'static str, ContractAddress>,
    pub state: CachedState<DictStateReader>,
    pub caller_address: ContractAddress,
    pub events: Vec<TestEvent>,
    pub block_context: BlockContext,
}

impl TestContext {
    pub fn new<T: StateFactory>(factory: T) -> Self {
        let mut state = create_custom_deploy_test_state(vec![], vec![]);
        let (contract_address, _retdata) = factory.create_state(&mut state);
        // get type name of factory
        Self {
            contract_addresses: HashMap::from([(T::name(), contract_address)]),
            state,
            caller_address: Signers::Alice.into(),
            events: vec![],
            block_context: BlockContext::create_for_testing(),
        }
    }

    pub fn new_with_callinfo<T: StateFactory>(factory: T) -> (Self, CallInfo) {
        let mut state = create_custom_deploy_test_state(vec![], vec![]);
        let (contract_address, call_info) = factory.create_state(&mut state);
        // get type name of factory
        (
            Self {
                contract_addresses: HashMap::from([(T::name(), contract_address)]),
                state,
                caller_address: Signers::Alice.into(),
                events: call_info.execution.events.iter().map(|e| e.clone().into()).collect(),
                block_context: BlockContext::create_for_testing(),
            },
            call_info,
        )
    }

    pub fn with_caller(mut self, caller_address: ContractAddress) -> Self {
        self.caller_address = caller_address;

        self
    }

    pub fn contract_address(&self, contract_name: &str) -> ContractAddress {
        self.contract_addresses.get(contract_name).unwrap().clone()
    }

    pub fn patch_with_factory<T: StateFactory>(&mut self, factory: T) {
        let (contract_address, _) = factory.create_state(&mut self.state);

        self.contract_addresses.insert(T::name(), contract_address);
    }

    pub fn add_manual_class_hash(&mut self, class_hash: ClassHash, contract_class: ContractClass) {
        self.state.state.class_hash_to_class.insert(class_hash, contract_class);
    }

    pub fn call_entry_point(
        &mut self,
        contract_name: &str,
        entry_point_name: &str,
        calldata: Vec<StarkFelt>,
    ) -> Vec<Felt> {
        let result = self.call_entry_point_raw(contract_name, entry_point_name, calldata);
        result.execution.retdata.0.iter().map(|felt| starkfelt_to_felt(*felt)).collect()
    }

    pub fn set_timestamp(&mut self, timestamp: u64) {
        self.block_context.block_timestamp = BlockTimestamp(timestamp);
    }

    pub fn get_timestamp(&self) -> u64 {
        self.block_context.block_timestamp.0
    }

    pub fn call_entry_point_raw(
        &mut self,
        contract_name: &str,
        entry_point_name: &str,
        calldata: Vec<StarkFelt>,
    ) -> CallInfo {
        let entry_point_selector = selector_from_name(entry_point_name);
        let calldata = Calldata(Arc::new(calldata));
        let contract_address = self.contract_address(contract_name);

        let entry_point_call = CallEntryPoint {
            calldata,
            entry_point_selector,
            code_address: Some(contract_address),
            storage_address: contract_address,
            caller_address: self.caller_address,
            ..external_entry_point(Some(contract_address))
        };

        let result = entry_point_call
            .execute_directly_given_block_context(&mut self.state, self.block_context.clone())
            .unwrap();

        let events = result.execution.events.clone();

        self.events.extend(events.iter().map(|e| e.clone().into()));

        result
    }

    pub fn get_event(&self, index: usize) -> Option<TestEvent> {
        self.events.get(index).cloned()
    }

    pub fn get_caller(&self) -> ContractAddress {
        self.caller_address
    }
}

pub fn deploy_contract(
    state: &mut dyn State,
    class_hash: Felt,
    contract_address_salt: Felt,
    calldata: &[Felt],
) -> SyscallResult<(Felt, CallInfo)> {
    let deployer_address: ContractAddress = Signers::Alice.into();

    let class_hash = ClassHash(felt_to_starkfelt(class_hash));

    let wrapper_calldata =
        Calldata(Arc::new(calldata.iter().map(|felt| felt_to_starkfelt(*felt)).collect()));

    let calculated_contract_address = calculate_contract_address(
        ContractAddressSalt(felt_to_starkfelt(contract_address_salt)),
        class_hash,
        &wrapper_calldata,
        deployer_address,
    )
    .map_err(|_| vec![Felt::from_hex(FAILED_TO_CALCULATE_CONTRACT_ADDRESS).unwrap()])?;

    let ctor_context = ConstructorContext {
        class_hash,
        code_address: Some(calculated_contract_address),
        storage_address: calculated_contract_address,
        caller_address: deployer_address,
    };

    let call_info = execute_deployment(
        state,
        &mut Default::default(),
        &mut EntryPointExecutionContext::new(
            &BlockContext::create_for_testing(),
            &AccountTransactionContext::Current(Default::default()),
            ExecutionMode::Execute,
            false,
        )
        .unwrap(),
        ctor_context,
        wrapper_calldata,
        u64::MAX,
    )
    .map_err(|_| vec![Felt::from_hex(FAILED_TO_EXECUTE_CALL).unwrap()])?;

    let contract_address_felt = starkfelt_to_felt(*calculated_contract_address.0.key());
    Ok((contract_address_felt, call_info))
}
