use starknet_types_core::felt::Felt;

use crate::execution::contract_class::{ContractClass, SierraContractClassV1};
use crate::test_utils::testing_context::StateFactory;
use crate::test_utils::{TEST_YAS_ROUTER_CONTRACT_CLASS_HASH, YAS_ROUTER_CONTRACT_PATH};

#[derive(Debug, Clone, Default)]
pub struct YASRouterFactory {
    args: Vec<Felt>,
}

impl YASRouterFactory {
    pub fn new() -> Self {
        YASRouterFactory { args: vec![] }
    }
}

impl StateFactory for YASRouterFactory {
    fn args(&self) -> Vec<Felt> {
        self.args.clone()
    }

    fn class_hash(&self) -> &'static str {
        TEST_YAS_ROUTER_CONTRACT_CLASS_HASH
    }

    fn contract_class(&self) -> ContractClass {
        SierraContractClassV1::from_file(YAS_ROUTER_CONTRACT_PATH).into()
    }

    fn name(&self) -> String {
        String::from("YASRouterFactory")
    }
}
