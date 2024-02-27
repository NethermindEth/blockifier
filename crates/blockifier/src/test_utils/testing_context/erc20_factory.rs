use starknet_types_core::felt::Felt;

use crate::execution::contract_class::{ContractClass, SierraContractClassV1};
use crate::execution::sierra_utils::contract_address_to_felt;
use crate::test_utils::testing_context::{Signers, StateFactory};
use crate::test_utils::{ERC20_FULL_CONTRACT_PATH, TEST_ERC20_FULL_CONTRACT_CLASS_HASH};

#[derive(Debug, Clone, Default)]
pub struct ERC20Factory {
    pub name: Option<String>,
}

impl<'a> ERC20Factory {
    pub fn new() -> Self {
        ERC20Factory { name: None }
    }

    pub fn with_name(name: String) -> Self {
        ERC20Factory { name: Some(name) }
    }
}

impl StateFactory for ERC20Factory {
    fn args(&self) -> Vec<Felt> {
        vec![
            contract_address_to_felt(Signers::Alice.into()), // Recipient
            contract_address_to_felt(Signers::Alice.into()), // Owner
        ]
    }

    fn class_hash(&self) -> &'static str {
        TEST_ERC20_FULL_CONTRACT_CLASS_HASH
    }

    fn contract_class(&self) -> ContractClass {
        SierraContractClassV1::from_file(ERC20_FULL_CONTRACT_PATH).into()
    }

    fn name(&self) -> String {
        if let Some(name) = &self.name { name.clone() } else { "ERC20Factory".to_string() }
    }
}
