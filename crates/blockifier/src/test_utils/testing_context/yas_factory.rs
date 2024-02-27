use starknet_api::core::{ClassHash, ContractAddress};
use starknet_types_core::felt::Felt;

use crate::execution::contract_class::{ContractClass, SierraContractClassV1};
use crate::execution::sierra_utils::{contract_address_to_felt, starkfelt_to_felt};
use crate::test_utils::testing_context::StateFactory;
use crate::test_utils::{TEST_YAS_FACTORY_CONTRACT_CLASS_HASH, YAS_FACTORY_CONTRACT_PATH};

#[derive(Debug, Clone, Default)]
pub struct YASFactory {
    pub deployer: ContractAddress,
    pub pool_class_hash: ClassHash,
}

impl<'a> YASFactory {
    pub fn new(deployer: ContractAddress, pool_class_hash: ClassHash) -> Self {
        YASFactory { deployer, pool_class_hash }
    }
}

impl StateFactory for YASFactory {
    fn args(&self) -> Vec<Felt> {
        // 'YAS', '$YAS', 4000000000000000000, OWNER()
        vec![contract_address_to_felt(self.deployer), starkfelt_to_felt(self.pool_class_hash.0)]
    }

    fn class_hash(&self) -> &'static str {
        TEST_YAS_FACTORY_CONTRACT_CLASS_HASH
    }

    fn contract_class(&self) -> ContractClass {
        SierraContractClassV1::from_file(YAS_FACTORY_CONTRACT_PATH).into()
    }

    fn name(&self) -> String {
        String::from("YASFactory")
    }
}
