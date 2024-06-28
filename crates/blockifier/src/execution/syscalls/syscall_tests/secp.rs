use starknet_api::transaction::Calldata;
use test_case::test_case;

use crate::abi::abi_utils::selector_from_name;
use crate::context::ChainInfo;
use crate::execution::call_info::{CallExecution, CallInfo};
use crate::execution::entry_point::CallEntryPoint;
use crate::execution::native::utils::NATIVE_GAS_PLACEHOLDER;
use crate::test_utils::contracts::FeatureContract;
use crate::test_utils::initial_test_state::test_state;
use crate::test_utils::{trivial_external_entry_point_new, CairoVersion, BALANCE};

#[test_case("test_secp256k1_get_point_from_x"; "get_point_from_x")]
#[test_case("test_secp256k1_mul"; "mul")]
#[test_case("test_secp256k1"; "full")]
/// Test whether cairo native and the vm return the same result for the syscalls.
fn test_secp256k1(entry_point: &str) {
    let mut res_vm =
        run_secp256k1(FeatureContract::TestContract(CairoVersion::Cairo1), entry_point);
    let mut res_native = run_secp256k1(FeatureContract::SierraTestContract, entry_point);

    // Zero-out the gas_consumed as the native runner does not keep proper track of it.
    // For now comparing just the results suffices.
    res_vm.execution.gas_consumed = 0;
    res_native.execution.gas_consumed = 0;

    pretty_assertions::assert_eq!(res_vm.execution, res_native.execution);
}

/// Start the execution of test_contract.cairo from the entry_point.
///
/// This requires that test_contract.cairo already has been compiled to sierra_test_contract.sierra.json and test_contract.casm.json.
fn run_secp256k1(test_contract: FeatureContract, entry_point: &str) -> CallInfo {
    let chain_info = &ChainInfo::create_for_testing();
    let mut state = test_state(chain_info, BALANCE, &[(test_contract, 1)]);

    let calldata = Calldata(vec![].into());
    let entry_point_call = CallEntryPoint {
        entry_point_selector: selector_from_name(entry_point),
        calldata,
        ..trivial_external_entry_point_new(test_contract)
    };

    entry_point_call.execute_directly(&mut state).unwrap()
}

#[test_case(FeatureContract::SierraTestContract, NATIVE_GAS_PLACEHOLDER; "Native")]
#[test_case(FeatureContract::TestContract(CairoVersion::Cairo1), 27582560; "VM")]
fn test_secp256r1(test_contract: FeatureContract, expected_gas: u64) {
    let chain_info = &ChainInfo::create_for_testing();
    let mut state = test_state(chain_info, BALANCE, &[(test_contract, 1)]);

    let calldata = Calldata(vec![].into());
    let entry_point_call = CallEntryPoint {
        entry_point_selector: selector_from_name("test_secp256r1"),
        calldata,
        ..trivial_external_entry_point_new(test_contract)
    };

    pretty_assertions::assert_eq!(
        entry_point_call.execute_directly(&mut state).unwrap().execution,
        CallExecution { gas_consumed: expected_gas, ..Default::default() }
    );
}
