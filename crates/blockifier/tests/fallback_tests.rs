use std::collections::HashMap;

use blockifier::execution::contract_class::SierraContractClassV1;
use blockifier::state::cached_state::CachedState;
use blockifier::test_utils::dict_state_reader::DictStateReader;
use blockifier::test_utils::TestContext;
use starknet_api::core::{ClassHash, ContractAddress, PatriciaKey};
use starknet_api::hash::{StarkFelt, StarkHash};
use starknet_api::{class_hash, contract_address, patricia_key};

const TEST_FALLBACK_CONTRACT_CLASS: &str =
    "./fallback_test_contract/target/dev/fallback_test_contract_SimpleContract.contract_class.json";

const TEST_FALLBACK_CONTRACT_CONTRACT_ADDRESS: &str = "0x10";

const TEST_FALLBACK_CONTRACT_CLASS_HASH: &str = "0x1";

fn prepare_env() -> (ContractAddress, CachedState<DictStateReader>) {
    let state = {
        let address_to_class_hash: HashMap<ContractAddress, ClassHash> = HashMap::from([(
            contract_address!(TEST_FALLBACK_CONTRACT_CONTRACT_ADDRESS),
            class_hash!(TEST_FALLBACK_CONTRACT_CLASS_HASH),
        )]);

        let class_hash_to_class = HashMap::from([(
            class_hash!(TEST_FALLBACK_CONTRACT_CLASS_HASH),
            SierraContractClassV1::from_file(TEST_FALLBACK_CONTRACT_CLASS).into(),
        )]);

        CachedState::from(DictStateReader {
            address_to_class_hash,
            class_hash_to_class,
            ..Default::default()
        })
    };

    let contract_address = contract_address!(TEST_FALLBACK_CONTRACT_CONTRACT_ADDRESS);

    (contract_address, state)
}

fn new_test_context() -> TestContext {
    let (contract_address, state) = prepare_env();

    TestContext { contract_address, state, caller_address: Default::default(), events: vec![] }
}

#[test]
fn test() {
    let mut context = new_test_context();

    assert_eq!(context.call_entry_point("set_value", vec![StarkFelt::from(1u8)]), vec![]);

    // println!("{:?}", context.call_entry_point("get_value", vec![]));
    //
    // assert!(false);
}
