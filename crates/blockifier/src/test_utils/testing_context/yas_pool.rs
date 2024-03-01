use starknet_types_core::felt::Felt;

use crate::execution::contract_class::{ContractClass, SierraContractClassV1};
use crate::test_utils::testing_context::StateFactory;
use crate::test_utils::{TEST_YAS_POOL_CONTRACT_CLASS_HASH, YAS_POOL_CONTRACT_PATH};

#[derive(Debug, Clone, Default)]
pub struct YASPoolFactory {
    args: Vec<Felt>,
    name: Option<String>,
}

impl YASPoolFactory {
    pub fn new(args: Vec<Felt>) -> Self {
        YASPoolFactory { args, name: None }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }
}

impl StateFactory for YASPoolFactory {
    fn args(&self) -> Vec<Felt> {
        self.args.clone()
    }

    fn class_hash(&self) -> &'static str {
        TEST_YAS_POOL_CONTRACT_CLASS_HASH
    }

    fn contract_class(&self) -> ContractClass {
        SierraContractClassV1::from_file(YAS_POOL_CONTRACT_PATH).into()
    }

    fn name(&self) -> String {
        self.name.clone().unwrap_or(String::from("YASPoolFactory"))
    }
}
