use std::sync::Arc;

use cairo_felt::Felt252;
use cairo_native::starknet::{
    BlockInfo, ExecutionInfoV2, Secp256k1Point, Secp256r1Point, StarkNetSyscallHandler,
    SyscallResult, TxInfo, TxV2Info, U256,
};
use cairo_vm::vm::runners::cairo_runner::ExecutionResources;
use starknet_api::core::{
    calculate_contract_address, ClassHash, ContractAddress, EntryPointSelector, EthAddress,
    PatriciaKey,
};
use starknet_api::deprecated_contract_class::EntryPointType;
use starknet_api::hash::{StarkFelt, StarkHash};
use starknet_api::state::StorageKey;
use starknet_api::transaction::{
    Calldata, ContractAddressSalt, EventContent, EventData, EventKey, L2ToL1Payload,
};
use starknet_types_core::felt::Felt;

use super::sierra_utils::{
    big4int_to_u256, chain_id_to_felt, contract_address_to_felt, encode_str_as_felts,
    felt_to_starkfelt, starkfelt_to_felt, u256_to_biguint,
};
use super::syscalls::exceeds_event_size_limit;
use crate::abi::constants;
use crate::execution::call_info::{CallInfo, MessageToL1, OrderedEvent, OrderedL2ToL1Message};
use crate::execution::common_hints::ExecutionMode;
use crate::execution::contract_class::ContractClass;
use crate::execution::entry_point::{
    CallEntryPoint, CallType, ConstructorContext, EntryPointExecutionContext,
};
use crate::execution::execution_utils::execute_deployment;
use crate::execution::syscalls::hint_processor::{
    execute_inner_call_raw, SyscallExecutionError, BLOCK_NUMBER_OUT_OF_RANGE_ERROR,
    FAILED_TO_CALCULATE_CONTRACT_ADDRESS, FAILED_TO_EXECUTE_CALL, FAILED_TO_READ_RESULT,
    FAILED_TO_WRITE, INVALID_ARGUMENT, INVALID_EXECUTION_MODE_ERROR, INVALID_INPUT_LENGTH_ERROR,
};
use crate::execution::syscalls::secp::{
    SecpAddRequest, SecpAddResponse, SecpGetPointFromXRequest, SecpGetPointFromXResponse,
    SecpHintProcessor, SecpMulRequest, SecpMulResponse, SecpNewRequest, SecpNewResponse,
};
use crate::state::state_api::State;

pub struct NativeSyscallHandler<'state> {
    pub state: &'state mut dyn State,
    pub caller_address: ContractAddress,
    pub contract_address: ContractAddress,
    pub entry_point_selector: StarkFelt,
    pub execution_resources: &'state mut ExecutionResources,
    pub execution_context: &'state mut EntryPointExecutionContext,
    pub events: Vec<OrderedEvent>,
    pub l2_to_l1_messages: Vec<OrderedL2ToL1Message>,
    pub inner_calls: Vec<CallInfo>,

    // Secp hint processors.
    pub secp256k1_hint_processor: SecpHintProcessor<ark_secp256k1::Config>,
    pub secp256r1_hint_processor: SecpHintProcessor<ark_secp256r1::Config>,
}
impl NativeSyscallHandler<'_> {
    fn allocate_point_k1(&mut self, point_x: U256, point_y: U256) -> SyscallResult<usize> {
        let request = SecpNewRequest { x: u256_to_biguint(point_x), y: u256_to_biguint(point_y) };

        let response = self.secp256k1_hint_processor.secp_new_unchecked(request);

        self._parse_allocate_point_response(response)
    }

    fn allocate_point_r1(&mut self, point_x: U256, point_y: U256) -> SyscallResult<usize> {
        let request = SecpNewRequest { x: u256_to_biguint(point_x), y: u256_to_biguint(point_y) };

        let response = self.secp256r1_hint_processor.secp_new_unchecked(request);

        self._parse_allocate_point_response(response)
    }

    fn _parse_allocate_point_response(
        &mut self,
        response: crate::execution::syscalls::SyscallResult<SecpNewResponse>,
    ) -> SyscallResult<usize> {
        match response {
            Ok(SecpNewResponse { optional_ec_point_id: Some(id) }) => Ok(id),
            Ok(SecpNewResponse { optional_ec_point_id: None }) => {
                Err(vec![Felt::from_hex(INVALID_ARGUMENT).unwrap()])
            }
            Err(SyscallExecutionError::SyscallError { error_data }) => {
                Err(error_data.iter().map(|felt| starkfelt_to_felt(*felt)).collect())
            }
            Err(_) => Err(vec![Felt::from_hex(FAILED_TO_EXECUTE_CALL).unwrap()]),
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
            let execution_mode_err = Felt::from_hex(INVALID_EXECUTION_MODE_ERROR).unwrap();

            return Err(vec![execution_mode_err]);
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
            .map_err(|e| vec![Felt::from_bytes_be_slice(e.to_string().as_bytes())])?;
        let block_hash_address =
            ContractAddress::try_from(StarkFelt::from(constants::BLOCK_HASH_CONTRACT_ADDRESS))
                .map_err(|e| vec![Felt::from_bytes_be_slice(e.to_string().as_bytes())])?;

        match self.state.get_storage_at(block_hash_address, key) {
            Ok(value) => Ok(Felt::from_bytes_be_slice(value.bytes())),
            Err(e) => Err(vec![Felt::from_bytes_be_slice(e.to_string().as_bytes())]),
        }
    }

    fn get_execution_info(
        &mut self,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<cairo_native::starknet::ExecutionInfo> {
        let block_context = &self.execution_context.tx_context.block_context.block_info;
        let account_tx_context = &self.execution_context.tx_context.tx_info;

        let block_info: BlockInfo = BlockInfo {
            block_number: block_context.block_number.0,
            block_timestamp: block_context.block_timestamp.0,
            sequencer_address: contract_address_to_felt(block_context.sequencer_address),
        };

        let signature =
            account_tx_context.signature().0.into_iter().map(starkfelt_to_felt).collect();

        let tx_info = TxInfo {
            version: starkfelt_to_felt(account_tx_context.version().0),
            account_contract_address: contract_address_to_felt(account_tx_context.sender_address()),
            // todo(rodro): it is ok to unwrap as default? Also, will this be deprecated soon?
            max_fee: account_tx_context.max_fee().unwrap_or_default().0,
            signature,
            transaction_hash: starkfelt_to_felt(account_tx_context.transaction_hash().0),
            chain_id: chain_id_to_felt(
                &self.execution_context.tx_context.block_context.chain_info.chain_id,
            )
            .unwrap(),
            nonce: starkfelt_to_felt(account_tx_context.nonce().0),
        };

        let caller_address = contract_address_to_felt(self.caller_address);
        let contract_address = contract_address_to_felt(self.contract_address);
        let entry_point_selector = starkfelt_to_felt(self.entry_point_selector);

        Ok(cairo_native::starknet::ExecutionInfo {
            block_info,
            tx_info,
            caller_address,
            contract_address,
            entry_point_selector,
        })
    }

    fn get_execution_info_v2(
        &mut self,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<ExecutionInfoV2> {
        let block_context = &self.execution_context.tx_context.block_context.block_info;
        let account_tx_context = &self.execution_context.tx_context.tx_info;

        Ok(ExecutionInfoV2 {
            block_info: BlockInfo {
                block_number: block_context.block_number.0,
                block_timestamp: block_context.block_timestamp.0,
                sequencer_address: contract_address_to_felt(block_context.sequencer_address),
            },
            tx_info: TxV2Info {
                version: starkfelt_to_felt(account_tx_context.version().0),
                account_contract_address: contract_address_to_felt(
                    account_tx_context.sender_address(),
                ),
                max_fee: account_tx_context.max_fee().unwrap_or_default().0,
                signature: vec![],
                transaction_hash: Default::default(),
                chain_id: Default::default(),
                nonce: Default::default(),
                resource_bounds: vec![],
                tip: 0,
                paymaster_data: vec![],
                nonce_data_availability_mode: 0,
                fee_data_availability_mode: 0,
                account_deployment_data: vec![],
            },
            caller_address: contract_address_to_felt(self.caller_address),
            contract_address: contract_address_to_felt(self.contract_address),
            entry_point_selector: starkfelt_to_felt(self.entry_point_selector),
        })
    }

    fn deploy(
        &mut self,
        class_hash: Felt,
        contract_address_salt: Felt,
        calldata: &[Felt],
        deploy_from_zero: bool,
        remaining_gas: &mut u128,
    ) -> SyscallResult<(Felt, Vec<Felt>)> {
        let deployer_address =
            if deploy_from_zero { ContractAddress::default() } else { self.contract_address };

        let class_hash = ClassHash(felt_to_starkfelt(class_hash));

        let wrapper_calldata = Calldata(Arc::new(
            calldata.iter().map(|felt| felt_to_starkfelt(*felt)).collect::<Vec<StarkFelt>>(),
        ));

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
            self.state,
            &mut self.execution_resources,
            &mut self.execution_context,
            ctor_context,
            wrapper_calldata,
            // todo: handle gas properly
            *remaining_gas as u64,
        )
        .map_err(|_| vec![Felt::from_hex(FAILED_TO_EXECUTE_CALL).unwrap()])?;

        let return_data =
            call_info.execution.retdata.0[..].iter().map(|felt| starkfelt_to_felt(*felt)).collect();

        let contract_address_felt =
            Felt::from_bytes_be_slice(calculated_contract_address.0.key().bytes());

        self.inner_calls.push(call_info);

        Ok((contract_address_felt, return_data))
    }

    fn replace_class(&mut self, class_hash: Felt, _remaining_gas: &mut u128) -> SyscallResult<()> {
        let class_hash = ClassHash(StarkHash::from(felt_to_starkfelt(class_hash)));
        let contract_class = self
            .state
            .get_compiled_contract_class(class_hash)
            .map_err(|e| encode_str_as_felts(&e.to_string()))?;

        match contract_class {
            ContractClass::V0(_) => Err(encode_str_as_felts(
                &SyscallExecutionError::ForbiddenClassReplacement { class_hash }.to_string(),
            )),
            ContractClass::V1(_) | ContractClass::V1Sierra(_) => {
                self.state
                    .set_class_hash_at(self.contract_address, class_hash)
                    .map_err(|e| encode_str_as_felts(&e.to_string()))?;

                Ok(())
            }
        }
    }

    fn library_call(
        &mut self,
        class_hash: Felt,
        function_selector: Felt,
        calldata: &[Felt],
        remaining_gas: &mut u128,
    ) -> SyscallResult<Vec<Felt>> {
        let class_hash = ClassHash(StarkHash::from(felt_to_starkfelt(class_hash)));

        let wrapper_calldata = Calldata(Arc::new(
            calldata.iter().map(|felt| felt_to_starkfelt(*felt)).collect::<Vec<StarkFelt>>(),
        ));

        let entry_point = CallEntryPoint {
            class_hash: Some(class_hash),
            code_address: None,
            entry_point_type: EntryPointType::External,
            entry_point_selector: EntryPointSelector(StarkHash::from(felt_to_starkfelt(
                function_selector,
            ))),
            calldata: wrapper_calldata,
            // The call context remains the same in a library call.
            storage_address: self.contract_address,
            caller_address: self.caller_address,
            call_type: CallType::Delegate,
            initial_gas: *remaining_gas as u64,
        };

        execute_inner_call_raw(
            entry_point,
            self.state,
            &mut self.execution_resources,
            &mut self.execution_context,
        )
    }

    fn call_contract(
        &mut self,
        address: Felt,
        entry_point_selector: Felt,
        calldata: &[Felt],
        remaining_gas: &mut u128,
    ) -> SyscallResult<Vec<Felt>> {
        let contract_address = ContractAddress::try_from(felt_to_starkfelt(address))
            .map_err(|_| vec![Felt::from_hex(INVALID_ARGUMENT).unwrap()])?;

        if self.execution_context.execution_mode == ExecutionMode::Validate
            && self.contract_address != contract_address
        {
            return Err(vec![Felt::from_hex(INVALID_EXECUTION_MODE_ERROR).unwrap()]);
        }

        let wrapper_calldata = Calldata(Arc::new(
            calldata.iter().map(|felt| felt_to_starkfelt(*felt)).collect::<Vec<StarkFelt>>(),
        ));

        let entry_point = CallEntryPoint {
            class_hash: None,
            code_address: Some(contract_address),
            entry_point_type: EntryPointType::External,
            entry_point_selector: EntryPointSelector(StarkHash::from(felt_to_starkfelt(
                entry_point_selector,
            ))),
            calldata: wrapper_calldata,
            storage_address: contract_address,
            caller_address: self.caller_address,
            call_type: CallType::Call,
            initial_gas: *remaining_gas as u64,
        };

        execute_inner_call_raw(
            entry_point,
            self.state,
            &mut self.execution_resources,
            &mut self.execution_context,
        )
    }

    fn storage_read(
        &mut self,
        _address_domain: u32,
        address: Felt,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<Felt> {
        // TODO - in progress - Dom
        let storage_key = StorageKey(
            PatriciaKey::try_from(felt_to_starkfelt(address))
                .map_err(|_| vec![Felt::from_hex(INVALID_ARGUMENT).unwrap()])?,
        );
        let read_result = self.state.get_storage_at(self.contract_address, storage_key);
        let unsafe_read_result =
            read_result.map_err(|_| vec![Felt::from_hex(FAILED_TO_READ_RESULT).unwrap()])?;

        Ok(starkfelt_to_felt(unsafe_read_result))
    }

    fn storage_write(
        &mut self,
        _address_domain: u32,
        address: Felt,
        value: Felt,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<()> {
        let storage_key = StorageKey(
            PatriciaKey::try_from(felt_to_starkfelt(address))
                .map_err(|_| vec![Felt::from_hex(INVALID_ARGUMENT).unwrap()])?,
        );

        let write_result =
            self.state.set_storage_at(self.contract_address, storage_key, felt_to_starkfelt(value));
        write_result.map_err(|_| vec![Felt::from_hex(FAILED_TO_WRITE).unwrap()])?;
        Ok(())
    }

    fn emit_event(
        &mut self,
        keys: &[Felt],
        data: &[Felt],
        _remaining_gas: &mut u128,
    ) -> SyscallResult<()> {
        let order = self.execution_context.n_emitted_events;
        let event = EventContent {
            keys: keys
                .iter()
                .map(|felt| EventKey(felt_to_starkfelt(*felt)))
                .collect::<Vec<EventKey>>(),
            data: EventData(data.iter().map(|felt| felt_to_starkfelt(*felt)).collect()),
        };

        exceeds_event_size_limit(
            self.execution_context.versioned_constants(),
            self.execution_context.n_emitted_events + 1,
            &event,
        )
        .map_err(|e| encode_str_as_felts(&e.to_string()))?;

        self.events.push(OrderedEvent { order, event });

        self.execution_context.n_emitted_events += 1;

        Ok(())
    }

    fn send_message_to_l1(
        &mut self,
        to_address: Felt,
        payload: &[Felt],
        _remaining_gas: &mut u128,
    ) -> SyscallResult<()> {
        let order = self.execution_context.n_sent_messages_to_l1;

        self.l2_to_l1_messages.push(OrderedL2ToL1Message {
            order,
            message: MessageToL1 {
                to_address: EthAddress::try_from(felt_to_starkfelt(to_address))
                    .map_err(|_| vec![Felt::from_hex(INVALID_ARGUMENT).unwrap()])?,
                payload: L2ToL1Payload(
                    payload.iter().map(|felt| felt_to_starkfelt(*felt)).collect(),
                ),
            },
        });

        self.execution_context.n_sent_messages_to_l1 += 1;

        Ok(())
    }

    fn keccak(&mut self, input: &[u64], _remaining_gas: &mut u128) -> SyscallResult<U256> {
        const CHUNK_SIZE: usize = 17;
        let length = input.len();

        if length % CHUNK_SIZE != 0 {
            return Err(vec![Felt::from_hex(INVALID_INPUT_LENGTH_ERROR).unwrap()]);
        }

        let n_chunks = length / CHUNK_SIZE;
        let mut state = [0u64; 25];

        for i in 0..n_chunks {
            let chunk = &input[i * CHUNK_SIZE..(i + 1) * CHUNK_SIZE];
            for (i, val) in chunk.iter().enumerate() {
                state[i] ^= val;
            }
            keccak::f1600(&mut state)
        }

        Ok(U256 {
            lo: state[2] as u128 | ((state[3] as u128) << 64),
            hi: state[0] as u128 | ((state[1] as u128) << 64),
        })
    }

    fn secp256k1_add(
        &mut self,
        p0: Secp256k1Point,
        p1: Secp256k1Point,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<Secp256k1Point> {
        let p_p0 = self.allocate_point_k1(p0.x, p0.y)?;
        let p_p1 = self.allocate_point_k1(p1.x, p1.y)?;
        let request = SecpAddRequest { lhs_id: Felt252::from(p_p0), rhs_id: Felt252::from(p_p1) };

        match self.secp256k1_hint_processor.secp_add(request) {
            Ok(SecpAddResponse { ec_point_id: id }) => {
                let id = Felt252::from(id);

                let point = self
                    .secp256k1_hint_processor
                    .get_point_by_id(id)
                    .map_err(|_| vec![Felt::from_hex(INVALID_ARGUMENT).unwrap()])?;
                let x = big4int_to_u256(point.x.0);
                let y = big4int_to_u256(point.y.0);

                Ok(Secp256k1Point { x, y })
            }
            Err(SyscallExecutionError::SyscallError { error_data }) => {
                Err(error_data.iter().map(|felt| starkfelt_to_felt(*felt)).collect())
            }
            Err(_) => Err(vec![Felt::from_hex(FAILED_TO_EXECUTE_CALL).unwrap()]),
        }
    }

    fn secp256k1_get_point_from_x(
        &mut self,
        x: U256,
        y_parity: bool,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<Option<Secp256k1Point>> {
        let request = SecpGetPointFromXRequest { x: u256_to_biguint(x), y_parity };

        match self.secp256k1_hint_processor.secp_get_point_from_x(request) {
            Ok(SecpGetPointFromXResponse { optional_ec_point_id: Some(id) }) => {
                let id = Felt252::from(id);

                let point = self
                    .secp256k1_hint_processor
                    .get_point_by_id(id)
                    .map_err(|_| vec![Felt::from_hex(INVALID_ARGUMENT).unwrap()])?;
                let x = big4int_to_u256(point.x.0);
                let y = big4int_to_u256(point.y.0);

                Ok(Some(Secp256k1Point { x, y }))
            }
            Ok(SecpGetPointFromXResponse { optional_ec_point_id: None }) => Ok(None),
            Err(SyscallExecutionError::SyscallError { error_data }) => {
                Err(error_data.iter().map(|felt| starkfelt_to_felt(*felt)).collect())
            }
            Err(_) => Err(vec![Felt::from_hex(FAILED_TO_EXECUTE_CALL).unwrap()]),
        }
    }

    fn secp256k1_get_xy(
        &mut self,
        p: Secp256k1Point,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<(U256, U256)> {
        Ok((p.x, p.y))
    }

    fn secp256k1_mul(
        &mut self,
        p: Secp256k1Point,
        m: U256,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<Secp256k1Point> {
        let p_id = self.allocate_point_k1(p.x, p.y)?;
        let request =
            SecpMulRequest { ec_point_id: Felt252::from(p_id), multiplier: u256_to_biguint(m) };

        match self.secp256k1_hint_processor.secp_mul(request) {
            Ok(SecpMulResponse { ec_point_id: id }) => {
                let id = Felt252::from(id);

                let point = self
                    .secp256k1_hint_processor
                    .get_point_by_id(id)
                    .map_err(|_| vec![Felt::from_hex(INVALID_ARGUMENT).unwrap()])?;
                let x = big4int_to_u256(point.x.0);
                let y = big4int_to_u256(point.y.0);

                Ok(Secp256k1Point { x, y })
            }
            Err(SyscallExecutionError::SyscallError { error_data }) => {
                Err(error_data.iter().map(|felt| starkfelt_to_felt(*felt)).collect())
            }
            Err(_) => Err(vec![Felt::from_hex(FAILED_TO_EXECUTE_CALL).unwrap()]),
        }
    }

    fn secp256k1_new(
        &mut self,
        x: U256,
        y: U256,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<Option<Secp256k1Point>> {
        let request = SecpNewRequest { x: u256_to_biguint(x), y: u256_to_biguint(y) };

        match self.secp256k1_hint_processor.secp_new(request) {
            Ok(SecpNewResponse { optional_ec_point_id: Some(_) }) => {
                Ok(Some(Secp256k1Point { x, y }))
            }
            Ok(SecpNewResponse { optional_ec_point_id: None }) => Ok(None),
            Err(SyscallExecutionError::SyscallError { error_data }) => {
                Err(error_data.iter().map(|felt| starkfelt_to_felt(*felt)).collect())
            }
            Err(_) => Err(vec![Felt::from_hex(FAILED_TO_EXECUTE_CALL).unwrap()]),
        }
    }

    fn secp256r1_add(
        &mut self,
        p0: Secp256r1Point,
        p1: Secp256r1Point,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<Secp256r1Point> {
        let p_p0 = self.allocate_point_r1(p0.x, p0.y)?;
        let p_p1 = self.allocate_point_r1(p1.x, p1.y)?;
        let request = SecpAddRequest { lhs_id: Felt252::from(p_p0), rhs_id: Felt252::from(p_p1) };

        match self.secp256r1_hint_processor.secp_add(request) {
            Ok(SecpAddResponse { ec_point_id: id }) => {
                let id = Felt252::from(id);

                let point = self
                    .secp256r1_hint_processor
                    .get_point_by_id(id)
                    .map_err(|_| vec![Felt::from_hex(INVALID_ARGUMENT).unwrap()])?;
                let x = big4int_to_u256(point.x.0);
                let y = big4int_to_u256(point.y.0);

                Ok(Secp256r1Point { x, y })
            }
            Err(SyscallExecutionError::SyscallError { error_data }) => {
                Err(error_data.iter().map(|felt| starkfelt_to_felt(*felt)).collect())
            }
            Err(_) => Err(vec![Felt::from_hex(FAILED_TO_EXECUTE_CALL).unwrap()]),
        }
    }

    fn secp256r1_get_point_from_x(
        &mut self,
        x: U256,
        y_parity: bool,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<Option<Secp256r1Point>> {
        let request = SecpGetPointFromXRequest { x: u256_to_biguint(x), y_parity };

        match self.secp256r1_hint_processor.secp_get_point_from_x(request) {
            Ok(SecpGetPointFromXResponse { optional_ec_point_id: Some(id) }) => {
                let id = Felt252::from(id);

                let point = self
                    .secp256r1_hint_processor
                    .get_point_by_id(id)
                    .map_err(|_| vec![Felt::from_hex(INVALID_ARGUMENT).unwrap()])?;
                let x = big4int_to_u256(point.x.0);
                let y = big4int_to_u256(point.y.0);

                Ok(Some(Secp256r1Point { x, y }))
            }
            Ok(SecpGetPointFromXResponse { optional_ec_point_id: None }) => Ok(None),
            Err(SyscallExecutionError::SyscallError { error_data }) => {
                Err(error_data.iter().map(|felt| starkfelt_to_felt(*felt)).collect())
            }
            Err(_) => Err(vec![Felt::from_hex(FAILED_TO_EXECUTE_CALL).unwrap()]),
        }
    }

    fn secp256r1_get_xy(
        &mut self,
        p: Secp256r1Point,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<(U256, U256)> {
        Ok((p.x, p.y))
    }

    fn secp256r1_mul(
        &mut self,
        p: Secp256r1Point,
        m: U256,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<Secp256r1Point> {
        let p_id = self.allocate_point_r1(p.x, p.y)?;
        let request =
            SecpMulRequest { ec_point_id: Felt252::from(p_id), multiplier: u256_to_biguint(m) };

        match self.secp256r1_hint_processor.secp_mul(request) {
            Ok(SecpMulResponse { ec_point_id: id }) => {
                let id = Felt252::from(id);

                let point = self
                    .secp256r1_hint_processor
                    .get_point_by_id(id)
                    .map_err(|_| vec![Felt::from_hex(INVALID_ARGUMENT).unwrap()])?;

                let x = big4int_to_u256(point.x.0);
                let y = big4int_to_u256(point.y.0);

                Ok(Secp256r1Point { x, y })
            }
            Err(SyscallExecutionError::SyscallError { error_data }) => {
                Err(error_data.iter().map(|felt| starkfelt_to_felt(*felt)).collect())
            }
            Err(_) => Err(vec![Felt::from_hex(FAILED_TO_EXECUTE_CALL).unwrap()]),
        }
    }

    fn secp256r1_new(
        &mut self,
        x: U256,
        y: U256,
        _remaining_gas: &mut u128,
    ) -> SyscallResult<Option<Secp256r1Point>> {
        let request = SecpNewRequest { x: u256_to_biguint(x), y: u256_to_biguint(y) };

        match self.secp256r1_hint_processor.secp_new(request) {
            Ok(SecpNewResponse { optional_ec_point_id: Some(_) }) => {
                Ok(Some(Secp256r1Point { x, y }))
            }
            Ok(SecpNewResponse { optional_ec_point_id: None }) => Ok(None),
            Err(SyscallExecutionError::SyscallError { error_data }) => {
                Err(error_data.iter().map(|felt| starkfelt_to_felt(*felt)).collect())
            }
            Err(_) => Err(vec![Felt::from_hex(FAILED_TO_EXECUTE_CALL).unwrap()]),
        }
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
