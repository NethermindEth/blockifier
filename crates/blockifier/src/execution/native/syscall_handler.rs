use std::collections::HashSet;
use std::hash::RandomState;

// use std::sync::Arc;

// use cairo_felt::Felt252;
use cairo_native::starknet::{
    BlockInfo, ExecutionInfo, ExecutionInfoV2, Secp256k1Point, Secp256r1Point,
    StarkNetSyscallHandler, SyscallResult, TxInfo, TxV2Info, U256,
};
use cairo_vm::vm::runners::cairo_runner::ExecutionResources;
use num_traits::ToPrimitive;
use starknet_api::core::{ContractAddress, EntryPointSelector};
use starknet_api::data_availability::DataAvailabilityMode;
use starknet_api::hash::StarkFelt;
use starknet_api::state::StorageKey;
use starknet_types_core::felt::Felt;

use super::utils::{
    contract_address_to_native_felt, encode_str_as_felts, stark_felt_to_native_felt,
};
use crate::abi::constants;
use crate::execution::call_info::{CallInfo, OrderedEvent, OrderedL2ToL1Message};
use crate::execution::common_hints::ExecutionMode;
use crate::execution::entry_point::EntryPointExecutionContext;
use crate::execution::execution_utils::max_fee_for_execution_info;
use crate::execution::native::utils::{calculate_resource_bounds, default_tx_v2_info};
use crate::execution::syscalls::hint_processor::{
    SyscallExecutionError, BLOCK_NUMBER_OUT_OF_RANGE_ERROR,
};
use crate::execution::syscalls::secp::SecpHintProcessor;
use crate::state::state_api::State;
use crate::transaction::objects::TransactionInfo;
pub struct NativeSyscallHandler<'state> {
    // Input for execution
    pub state: &'state mut dyn State,
    pub execution_resources: &'state mut ExecutionResources,
    pub execution_context: &'state mut EntryPointExecutionContext,

    // Call information
    pub caller_address: ContractAddress,
    pub contract_address: ContractAddress,
    pub entry_point_selector: StarkFelt,

    // Execution results
    pub events: Vec<OrderedEvent>,
    pub l2_to_l1_messages: Vec<OrderedL2ToL1Message>,
    pub inner_calls: Vec<CallInfo>,
    // Additional execution result info
    pub storage_read_values: Vec<StarkFelt>,
    pub accessed_storage_keys: HashSet<StorageKey, RandomState>,

    // Secp hint processors.
    pub secp256k1_hint_processor: SecpHintProcessor<ark_secp256k1::Config>,
    pub secp256r1_hint_processor: SecpHintProcessor<ark_secp256r1::Config>,
}

impl<'state> NativeSyscallHandler<'_> {
    pub fn new(
        state: &'state mut dyn State,
        caller_address: ContractAddress,
        contract_address: ContractAddress,
        entry_point_selector: EntryPointSelector,
        execution_resources: &'state mut ExecutionResources,
        execution_context: &'state mut EntryPointExecutionContext,
    ) -> NativeSyscallHandler<'state> {
        NativeSyscallHandler {
            state,
            caller_address,
            contract_address,
            entry_point_selector: entry_point_selector.0,
            execution_resources,
            execution_context,
            events: Vec::new(),
            l2_to_l1_messages: Vec::new(),
            inner_calls: Vec::new(),
            secp256k1_hint_processor: Default::default(),
            secp256r1_hint_processor: Default::default(),
            storage_read_values: Vec::new(),
            accessed_storage_keys: HashSet::new(),
        }
    }
}

impl<'state> StarkNetSyscallHandler for NativeSyscallHandler<'state> {
    fn get_block_hash(
        &mut self,
        block_number: u64,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<Felt> {
        if self.execution_context.execution_mode == ExecutionMode::Validate {
            let err = SyscallExecutionError::InvalidSyscallInExecutionMode {
                syscall_name: "get_block_hash".to_string(),
                execution_mode: ExecutionMode::Validate,
            };

            return Err(encode_str_as_felts(&err.to_string()));
        }

        let current_block_number =
            self.execution_context.tx_context.block_context.block_info.block_number.0;

        if current_block_number < constants::STORED_BLOCK_HASH_BUFFER
            || block_number > current_block_number - constants::STORED_BLOCK_HASH_BUFFER
        {
            // `panic` is unreachable in this case, also this is covered by tests so we can safely
            // unwrap
            let out_of_range_felt = Felt::from_hex(BLOCK_NUMBER_OUT_OF_RANGE_ERROR).unwrap();

            return Err(vec![out_of_range_felt]);
        }

        let key = StorageKey::try_from(StarkFelt::from(block_number))
            .map_err(|e| encode_str_as_felts(&e.to_string()))?;
        let block_hash_address =
            ContractAddress::try_from(StarkFelt::from(constants::BLOCK_HASH_CONTRACT_ADDRESS))
                .map_err(|e| encode_str_as_felts(&e.to_string()))?;

        match self.state.get_storage_at(block_hash_address, key) {
            Ok(value) => Ok(Felt::from_bytes_be_slice(value.bytes())),
            Err(e) => Err(encode_str_as_felts(&e.to_string())),
        }
    }

    fn get_execution_info(
        &mut self,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<cairo_native::starknet::ExecutionInfo> {
        let block_info = &self.execution_context.tx_context.block_context.block_info;
        let native_block_info: BlockInfo = if self.execution_context.execution_mode
            == ExecutionMode::Validate
        {
            let versioned_constants = self.execution_context.versioned_constants();
            let block_number = block_info.block_number.0;
            let block_timestamp = block_info.block_timestamp.0;
            let validate_block_number_rounding =
                versioned_constants.get_validate_block_number_rounding();
            let rounded_block_number =
                (block_number / validate_block_number_rounding) * validate_block_number_rounding;
            let validate_timestamp_rounding = versioned_constants.get_validate_timestamp_rounding();
            let rounded_timestamp =
                (block_timestamp / validate_timestamp_rounding) * validate_timestamp_rounding;
            BlockInfo {
                block_number: rounded_block_number,
                block_timestamp: rounded_timestamp,
                sequencer_address: Felt::ZERO,
            }
        } else {
            BlockInfo {
                block_number: block_info.block_number.0,
                block_timestamp: block_info.block_timestamp.0,
                sequencer_address: contract_address_to_native_felt(block_info.sequencer_address),
            }
        };

        let tx_info = &self.execution_context.tx_context.tx_info;
        let native_tx_info = TxInfo {
            version: stark_felt_to_native_felt(tx_info.signed_version().0),
            account_contract_address: contract_address_to_native_felt(tx_info.sender_address()),
            max_fee: max_fee_for_execution_info(tx_info).to_u128().unwrap(),
            signature: tx_info.signature().0.into_iter().map(stark_felt_to_native_felt).collect(),
            transaction_hash: stark_felt_to_native_felt(tx_info.transaction_hash().0),
            chain_id: Felt::from_hex(
                &self.execution_context.tx_context.block_context.chain_info.chain_id.as_hex(),
            )
            .unwrap(),
            nonce: stark_felt_to_native_felt(tx_info.nonce().0),
        };

        let caller_address = contract_address_to_native_felt(self.caller_address);
        let contract_address = contract_address_to_native_felt(self.contract_address);
        let entry_point_selector = stark_felt_to_native_felt(self.entry_point_selector);

        Ok(ExecutionInfo {
            block_info: native_block_info,
            tx_info: native_tx_info,
            caller_address,
            contract_address,
            entry_point_selector,
        })
    }

    fn get_execution_info_v2(
        &mut self,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<ExecutionInfoV2> {
        // Get Block Info
        let block_info = &self.execution_context.tx_context.block_context.block_info;
        let native_block_info: BlockInfo = if self.execution_context.execution_mode
            == ExecutionMode::Validate
        {
            let versioned_constants = self.execution_context.versioned_constants();
            let block_number = block_info.block_number.0;
            let block_timestamp = block_info.block_timestamp.0;
            // Round down to the nearest multiple of validate_block_number_rounding.
            let validate_block_number_rounding =
                versioned_constants.get_validate_block_number_rounding();
            let rounded_block_number =
                (block_number / validate_block_number_rounding) * validate_block_number_rounding;
            // Round down to the nearest multiple of validate_timestamp_rounding.
            let validate_timestamp_rounding = versioned_constants.get_validate_timestamp_rounding();
            let rounded_timestamp =
                (block_timestamp / validate_timestamp_rounding) * validate_timestamp_rounding;
            BlockInfo {
                block_number: rounded_block_number,
                block_timestamp: rounded_timestamp,
                sequencer_address: Felt::ZERO,
            }
        } else {
            BlockInfo {
                block_number: block_info.block_number.0,
                block_timestamp: block_info.block_timestamp.0,
                sequencer_address: contract_address_to_native_felt(block_info.sequencer_address),
            }
        };

        // Get Transaction Info
        let tx_info = &self.execution_context.tx_context.tx_info;
        let mut native_tx_info = TxV2Info {
            version: stark_felt_to_native_felt(tx_info.signed_version().0),
            account_contract_address: contract_address_to_native_felt(tx_info.sender_address()),
            max_fee: max_fee_for_execution_info(tx_info).to_u128().unwrap(),
            signature: tx_info.signature().0.into_iter().map(stark_felt_to_native_felt).collect(),
            transaction_hash: stark_felt_to_native_felt(tx_info.transaction_hash().0),
            chain_id: Felt::from_hex(
                &self.execution_context.tx_context.block_context.chain_info.chain_id.as_hex(),
            )
            .unwrap(),
            nonce: stark_felt_to_native_felt(tx_info.nonce().0),
            ..default_tx_v2_info()
        };
        // If handling V3 transaction fill the "default" fields
        if let TransactionInfo::Current(context) = tx_info {
            let to_u32 = |x| match x {
                DataAvailabilityMode::L1 => 0,
                DataAvailabilityMode::L2 => 1,
            };
            native_tx_info = TxV2Info {
                resource_bounds: calculate_resource_bounds(context)?,
                tip: context.tip.0.into(),
                paymaster_data: context
                    .paymaster_data
                    .0
                    .iter()
                    .map(|f| stark_felt_to_native_felt(*f))
                    .collect(),
                nonce_data_availability_mode: to_u32(context.nonce_data_availability_mode),
                fee_data_availability_mode: to_u32(context.fee_data_availability_mode),
                account_deployment_data: context
                    .account_deployment_data
                    .0
                    .iter()
                    .map(|f| stark_felt_to_native_felt(*f))
                    .collect(),
                ..native_tx_info
            };
        }

        let caller_address = contract_address_to_native_felt(self.caller_address);
        let contract_address = contract_address_to_native_felt(self.contract_address);
        let entry_point_selector = stark_felt_to_native_felt(self.entry_point_selector);

        Ok(ExecutionInfoV2 {
            block_info: native_block_info,
            tx_info: native_tx_info,
            caller_address,
            contract_address,
            entry_point_selector,
        })
    }
    fn deploy(
        &mut self,
        _class_hash: Felt,
        _contract_address_salt: Felt,
        _calldata: &[Felt],
        _deploy_from_zero: bool,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<(Felt, Vec<Felt>)> {
        unimplemented!("implement deploy")
    }

    fn replace_class(&mut self, _class_hash: Felt, _remaining_gas: &mut u128) -> SyscallResult<()> {
        unimplemented!("implement replace_class")
    }

    fn library_call(
        &mut self,
        _class_hash: Felt,
        _function_selector: Felt,
        _calldata: &[Felt],
        _remaining_gas: &mut u128,
    ) -> SyscallResult<Vec<Felt>> {
        unimplemented!("implement library_call")
    }

    fn call_contract(
        &mut self,
        _address: Felt,
        _entry_point_selector: Felt,
        _calldata: &[Felt],
        _remaining_gas: &mut u128,
    ) -> SyscallResult<Vec<Felt>> {
        unimplemented!("implement call_contract")
    }

    fn storage_read(
        &mut self,
        _address_domain: u32,
        _address: Felt,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<Felt> {
        unimplemented!("implement storage_read")
    }

    fn storage_write(
        &mut self,
        _address_domain: u32,
        _address: Felt,
        _value: Felt,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<()> {
        unimplemented!("implement storage_write")
    }

    fn emit_event(
        &mut self,
        _keys: &[Felt],
        _data: &[Felt],
        _remaining_gas: &mut u128,
    ) -> SyscallResult<()> {
        unimplemented!("implement emit_event")
    }

    fn send_message_to_l1(
        &mut self,
        _to_address: Felt,
        _payload: &[Felt],
        _remaining_gas: &mut u128,
    ) -> SyscallResult<()> {
        unimplemented!("implement send_message_to_l1")
    }

    fn keccak(&mut self, _input: &[u64], _remaining_gas: &mut u128) -> SyscallResult<U256> {
        unimplemented!("implement keccak")
    }

    fn secp256k1_new(
        &mut self,
        _x: U256,
        _y: U256,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<Option<Secp256k1Point>> {
        unimplemented!("implement secp256k1_new")
    }

    fn secp256k1_add(
        &mut self,
        _p0: Secp256k1Point,
        _p1: Secp256k1Point,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<Secp256k1Point> {
        unimplemented!("implement secp256k1_add")
    }

    fn secp256k1_mul(
        &mut self,
        _p: Secp256k1Point,
        _m: U256,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<Secp256k1Point> {
        unimplemented!("implement secp256k1_mul")
    }

    fn secp256k1_get_point_from_x(
        &mut self,
        _x: U256,
        _y_parity: bool,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<Option<Secp256k1Point>> {
        unimplemented!("implement secp256k1_get_point_from_x")
    }

    fn secp256k1_get_xy(
        &mut self,
        _p: Secp256k1Point,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<(U256, U256)> {
        unimplemented!("implement secp256k1_get_xy")
    }

    fn secp256r1_new(
        &mut self,
        _x: U256,
        _y: U256,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<Option<Secp256r1Point>> {
        unimplemented!("implement secp256r1_new")
    }

    fn secp256r1_add(
        &mut self,
        _p0: Secp256r1Point,
        _p1: Secp256r1Point,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<Secp256r1Point> {
        unimplemented!("implement secp256r1_add")
    }

    fn secp256r1_mul(
        &mut self,
        _p: Secp256r1Point,
        _m: U256,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<Secp256r1Point> {
        unimplemented!("implement secp256r1_mul")
    }

    fn secp256r1_get_point_from_x(
        &mut self,
        _x: U256,
        _y_parity: bool,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<Option<Secp256r1Point>> {
        unimplemented!("implement secp256r1_get_point_from_x")
    }

    fn secp256r1_get_xy(
        &mut self,
        _p: Secp256r1Point,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<(U256, U256)> {
        unimplemented!("implement secp256r1_get_xy")
    }

    fn pop_log(&mut self) {
        todo!("Native syscall handler - pop_log") // unimplemented in cairo native
    }

    fn set_account_contract_address(&mut self, _contract_address: Felt) {
        todo!("Native syscall handler - set_account_contract_address") // unimplemented in cairo native
    }

    fn set_block_number(&mut self, _block_number: u64) {
        todo!("Native syscall handler - set_block_number") // unimplemented in cairo native
    }

    fn set_block_timestamp(&mut self, _block_timestamp: u64) {
        todo!("Native syscall handler - set_block_timestamp") // unimplemented in cairo native
    }

    fn set_caller_address(&mut self, _address: Felt) {
        todo!("Native syscall handler - set_caller_address") // unimplemented in cairo native
    }

    fn set_chain_id(&mut self, _chain_id: Felt) {
        todo!("Native syscall handler - set_chain_id") // unimplemented in cairo native
    }

    fn set_contract_address(&mut self, _address: Felt) {
        todo!("Native syscall handler - set_contract_address") // unimplemented in cairo native
    }

    fn set_max_fee(&mut self, _max_fee: u128) {
        todo!("Native syscall handler - set_max_fee") // unimplemented in cairo native
    }

    fn set_nonce(&mut self, _nonce: Felt) {
        todo!("Native syscall handler - set_nonce") // unimplemented in cairo native
    }

    fn set_sequencer_address(&mut self, _address: Felt) {
        todo!("Native syscall handler - set_sequencer_address") // unimplemented in cairo native
    }

    fn set_signature(&mut self, _signature: &[Felt]) {
        todo!("Native syscall handler - set_signature") // unimplemented in cairo native
    }

    fn set_transaction_hash(&mut self, _transaction_hash: Felt) {
        todo!("Native syscall handler - set_transaction_hash") // unimplemented in cairo native
    }

    fn set_version(&mut self, _version: Felt) {
        todo!("Native syscall handler - set_version") // unimplemented in cairo native
    }
}
