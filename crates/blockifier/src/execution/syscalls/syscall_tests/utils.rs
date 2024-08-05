use assert_matches::assert_matches;
use starknet_api::core::ClassHash;

use crate::execution::contract_class::ContractClass;
use crate::state::state_api::State;
use crate::test_utils::contracts::FeatureContract;

pub fn assert_contract_uses_native(class_hash: ClassHash, state: &dyn State) {
    assert_matches!(
        state
            .get_compiled_contract_class(class_hash)
            .unwrap_or_else(|_| panic!("Expected contract class at {class_hash}")),
        ContractClass::V1Native(_)
    )
}

pub fn assert_contract_uses_vm(class_hash: ClassHash, state: &dyn State) {
    assert_matches!(
        state
            .get_compiled_contract_class(class_hash)
            .unwrap_or_else(|_| panic!("Expected contract class at {class_hash}")),
        ContractClass::V1(_) | ContractClass::V0(_)
    )
}

pub fn assert_consistent_contract_version(contract: FeatureContract, state: &dyn State) {
    let hash = contract.get_class_hash();
    match contract {
        FeatureContract::SierraTestContract | FeatureContract::SierraExecutionInfoV1Contract => {
            assert_contract_uses_native(hash, state)
        }
        FeatureContract::SecurityTests
        | FeatureContract::ERC20(_)
        | FeatureContract::LegacyTestContract
        | FeatureContract::AccountWithLongValidate(_)
        | FeatureContract::AccountWithoutValidations(_)
        | FeatureContract::Empty(_)
        | FeatureContract::FaultyAccount(_)
        | FeatureContract::TestContract(_) => assert_contract_uses_vm(hash, state),
    }
}

pub fn verify_compiler_version(contract: FeatureContract, expected_version: &str) {
    // Read and parse file content.
    let raw_contract: serde_json::Value =
        serde_json::from_str(&contract.get_raw_class()).expect("Error parsing JSON");

    // Verify version.
    if let Some(compiler_version) = raw_contract["compiler_version"].as_str() {
        assert_eq!(compiler_version, expected_version);
    } else {
        panic!("'compiler_version' not found or not a valid string in JSON.");
    }
}
