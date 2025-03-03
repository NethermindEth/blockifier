use std::collections::{HashMap, HashSet};

use cairo_vm::types::builtin_name::BuiltinName;
use cairo_vm::vm::runners::cairo_runner::ExecutionResources;
use pretty_assertions::assert_eq;
use starknet_api::core::PatriciaKey;
use starknet_api::state::StorageKey;
use starknet_api::transaction::Calldata;
use starknet_api::{calldata, felt, patricia_key};
use test_case::test_case;

use crate::abi::abi_utils::selector_from_name;
use crate::context::ChainInfo;
use crate::execution::call_info::{CallExecution, CallInfo, Retdata};
use crate::execution::entry_point::{CallEntryPoint, CallType};
use crate::execution::native::utils::NATIVE_GAS_PLACEHOLDER;
use crate::execution::syscalls::syscall_tests::constants::{
    REQUIRED_GAS_LIBRARY_CALL_TEST, REQUIRED_GAS_STORAGE_READ_WRITE_TEST,
};
use crate::execution::syscalls::SyscallSelector;
use crate::retdata;
use crate::test_utils::contracts::FeatureContract;
use crate::test_utils::initial_test_state::test_state;
use crate::test_utils::{
    get_syscall_resources, trivial_external_entry_point_new, CairoVersion, BALANCE,
};

#[test_case(FeatureContract::TestContract(CairoVersion::Cairo1), REQUIRED_GAS_LIBRARY_CALL_TEST; "VM")]
fn test_library_call(test_contract: FeatureContract, expected_gas: u64) {
    let chain_info = &ChainInfo::create_for_testing();
    let mut state = test_state(chain_info, BALANCE, &[(test_contract, 1)]);

    let inner_entry_point_selector = selector_from_name("test_storage_read_write");
    let calldata = calldata![
        test_contract.get_class_hash().0, // Class hash.
        inner_entry_point_selector.0,     // Function selector.
        felt!(2_u8),                      // Calldata length.
        felt!(1234_u16),                  // Calldata: address.
        felt!(91_u8)                      // Calldata: value.
    ];

    let entry_point_call = CallEntryPoint {
        entry_point_selector: selector_from_name("test_library_call"),
        calldata,
        class_hash: Some(test_contract.get_class_hash()),
        ..trivial_external_entry_point_new(test_contract)
    };

    assert_eq!(
        entry_point_call.execute_directly(&mut state).unwrap().execution,
        CallExecution {
            retdata: retdata![felt!(91_u16)],
            gas_consumed: expected_gas,
            ..Default::default()
        }
    );
}

#[test_case(FeatureContract::SierraTestContract; "Native")]
#[test_case(FeatureContract::TestContract(CairoVersion::Cairo1); "VM")]
fn test_library_call_assert_fails(test_contract: FeatureContract) {
    let chain_info = &ChainInfo::create_for_testing();
    let mut state = test_state(chain_info, BALANCE, &[(test_contract, 1)]);
    let inner_entry_point_selector = selector_from_name("assert_eq");
    let calldata = calldata![
        test_contract.get_class_hash().0, // Class hash.
        inner_entry_point_selector.0,     // Function selector.
        felt!(2_u8),                      // Calldata length.
        felt!(0_u8),                      // Calldata: first assert value.
        felt!(1_u8)                       // Calldata: second assert value.
    ];
    let entry_point_call = CallEntryPoint {
        entry_point_selector: selector_from_name("test_library_call"),
        calldata,
        class_hash: Some(test_contract.get_class_hash()),
        ..trivial_external_entry_point_new(test_contract)
    };

    let err = entry_point_call.execute_directly(&mut state).unwrap_err();
    assert!(err.to_string().contains("x != y"));
}

#[test_case(FeatureContract::SierraTestContract, NATIVE_GAS_PLACEHOLDER; "Native")]
#[test_case(FeatureContract::TestContract(CairoVersion::Cairo1), 276880; "VM")]
fn test_nested_library_call(test_contract: FeatureContract, expected_gas: u64) {
    let chain_info = &ChainInfo::create_for_testing();
    let mut state = test_state(chain_info, BALANCE, &[(test_contract, 1)]);

    let (key, value) = (255_u64, 44_u64);
    let outer_entry_point_selector = selector_from_name("test_library_call");
    let inner_entry_point_selector = selector_from_name("test_storage_read_write");
    let test_class_hash = test_contract.get_class_hash();
    let main_entry_point_calldata = calldata![
        test_class_hash.0,            // Class hash.
        outer_entry_point_selector.0, // Library call function selector.
        inner_entry_point_selector.0, // Storage function selector.
        felt!(key),                   // Calldata: address.
        felt!(value)                  // Calldata: value.
    ];

    // Todo(rodrigo): Execution resources from the VM & Native are mesaured differently
    // helper function to change the expected resource values from both of executions
    let if_native = |a, b| {
        if matches!(test_contract, FeatureContract::SierraTestContract) { a } else { b }
    };

    // Create expected call info tree.
    let main_entry_point = CallEntryPoint {
        entry_point_selector: selector_from_name("test_nested_library_call"),
        calldata: main_entry_point_calldata,
        class_hash: Some(test_class_hash),
        initial_gas: 9999906600,
        ..trivial_external_entry_point_new(test_contract)
    };
    let nested_storage_entry_point = CallEntryPoint {
        entry_point_selector: inner_entry_point_selector,
        calldata: calldata![felt!(key + 1), felt!(value + 1)],
        class_hash: Some(test_class_hash),
        code_address: None,
        call_type: CallType::Delegate,
        initial_gas: if_native(9999725020, 9999745020),
        ..trivial_external_entry_point_new(test_contract)
    };
    let library_entry_point = CallEntryPoint {
        entry_point_selector: outer_entry_point_selector,
        calldata: calldata![
            test_class_hash.0,            // Class hash.
            inner_entry_point_selector.0, // Storage function selector.
            felt!(2_u8),                  // Calldata: address.
            felt!(key + 1),               // Calldata: address.
            felt!(value + 1)              // Calldata: value.
        ],
        class_hash: Some(test_class_hash),
        code_address: None,
        call_type: CallType::Delegate,
        initial_gas: if_native(9999813550, 9999823550),
        ..trivial_external_entry_point_new(test_contract)
    };
    let storage_entry_point = CallEntryPoint {
        calldata: calldata![felt!(key), felt!(value)],
        initial_gas: if_native(9999752538, 9999656870),
        ..nested_storage_entry_point
    };

    // Todo(rodrigo): Execution resources from the VM & Native are mesaured differently
    // Resources are not tracked when using Native
    let default_resources_if_native = |resources| {
        if matches!(test_contract, FeatureContract::SierraTestContract) {
            ExecutionResources::default()
        } else {
            resources
        }
    };

    let storage_entry_point_resources = default_resources_if_native(ExecutionResources {
        n_steps: 243,
        n_memory_holes: 0,
        builtin_instance_counter: HashMap::from([(BuiltinName::range_check, 7)]),
    });
    let nested_storage_call_info = CallInfo {
        call: nested_storage_entry_point,
        execution: CallExecution {
            retdata: retdata![felt!(value + 1)],
            gas_consumed: if_native(NATIVE_GAS_PLACEHOLDER, REQUIRED_GAS_STORAGE_READ_WRITE_TEST),
            ..CallExecution::default()
        },
        resources: storage_entry_point_resources.clone(),
        storage_read_values: vec![felt!(value + 1)],
        accessed_storage_keys: HashSet::from([StorageKey(patricia_key!(key + 1))]),
        ..Default::default()
    };

    let library_call_resources = default_resources_if_native(
        &get_syscall_resources(SyscallSelector::LibraryCall)
            + &ExecutionResources {
                n_steps: 388,
                n_memory_holes: 0,
                builtin_instance_counter: HashMap::from([(BuiltinName::range_check, 15)]),
            },
    );
    let library_call_info = CallInfo {
        call: library_entry_point,
        execution: CallExecution {
            retdata: retdata![felt!(value + 1)],
            gas_consumed: if_native(NATIVE_GAS_PLACEHOLDER, REQUIRED_GAS_LIBRARY_CALL_TEST),
            ..CallExecution::default()
        },
        resources: library_call_resources,
        inner_calls: vec![nested_storage_call_info],
        ..Default::default()
    };

    let storage_call_info = CallInfo {
        call: storage_entry_point,
        execution: CallExecution {
            retdata: retdata![felt!(value)],
            gas_consumed: if_native(NATIVE_GAS_PLACEHOLDER, REQUIRED_GAS_STORAGE_READ_WRITE_TEST),
            ..CallExecution::default()
        },
        resources: storage_entry_point_resources,
        storage_read_values: vec![felt!(value)],
        accessed_storage_keys: HashSet::from([StorageKey(patricia_key!(key))]),
        ..Default::default()
    };

    let main_call_resources = default_resources_if_native(
        &(&get_syscall_resources(SyscallSelector::LibraryCall) * 3)
            + &ExecutionResources {
                n_steps: 749,
                n_memory_holes: 2,
                builtin_instance_counter: HashMap::from([(BuiltinName::range_check, 27)]),
            },
    );
    let expected_call_info = CallInfo {
        call: main_entry_point.clone(),
        execution: CallExecution {
            retdata: retdata![felt!(value)],
            gas_consumed: expected_gas,
            ..CallExecution::default()
        },
        resources: main_call_resources,
        inner_calls: vec![library_call_info, storage_call_info],
        ..Default::default()
    };

    assert_eq!(main_entry_point.execute_directly(&mut state).unwrap(), expected_call_info);
}
