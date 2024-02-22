use starknet_api::core::{ClassHash, ContractAddress, PatriciaKey};
use starknet_api::hash::StarkHash;
use starknet_api::{class_hash, contract_address, patricia_key};
use starknet_types_core::felt::Felt;

use crate::execution::contract_class::{ContractClass, SierraContractClassV1};
use crate::execution::sierra_utils::contract_address_to_felt;
use crate::state::cached_state::CachedState;
use crate::test_utils::dict_state_reader::DictStateReader;
use crate::test_utils::testing_context::{create_custom_deploy_test_state, Signers, StateFactory};
use crate::test_utils::{
    ERC20_FULL_CONTRACT_PATH, TEST_ERC20_FULL_CONTRACT_ADDRESS, TEST_ERC20_FULL_CONTRACT_CLASS_HASH,
};

pub fn prepare_erc20_deploy_test_state() -> (ContractAddress, CachedState<DictStateReader>) {
    ERC20Factory::new().create_state()
}

pub struct ERC20Factory {
    args: Vec<Felt>,
    #[allow(dead_code)]
    state: CachedState<DictStateReader>,
}

impl ERC20Factory {
    pub fn new() -> Self {
        ERC20Factory {
            args: vec![
                contract_address_to_felt(Signers::Alice.into()), // Recipient
                contract_address_to_felt(Signers::Alice.into()), // Owner
            ],
            state: create_custom_deploy_test_state(vec![], vec![]),
        }
    }

    pub fn new_with_test_state_args(
        address_to_class_hash: Vec<(ContractAddress, ClassHash)>,
        class_hash_to_class: Vec<(ClassHash, ContractClass)>,
    ) -> Self {
        ERC20Factory {
            args: vec![],
            state: create_custom_deploy_test_state(address_to_class_hash, class_hash_to_class),
        }
    }
}

impl StateFactory for ERC20Factory {
    fn get_state(&self) -> CachedState<DictStateReader> {
        create_custom_deploy_test_state(
            vec![(
                contract_address!(TEST_ERC20_FULL_CONTRACT_ADDRESS),
                class_hash!(TEST_ERC20_FULL_CONTRACT_CLASS_HASH),
            )],
            vec![(
                class_hash!(TEST_ERC20_FULL_CONTRACT_CLASS_HASH),
                SierraContractClassV1::from_file(ERC20_FULL_CONTRACT_PATH).into(),
            )],
        )
    }

    fn args(&self) -> Vec<Felt> {
        self.args.clone()
    }

    fn class_hash(&self) -> &'static str {
        TEST_ERC20_FULL_CONTRACT_CLASS_HASH
    }
}
