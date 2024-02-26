use starknet_types_core::felt::Felt;

use crate::execution::contract_class::{ContractClass, SierraContractClassV1};
use crate::test_utils::testing_context::string_utils::string_to_felt;
use crate::test_utils::testing_context::{Signers, StateFactory};
use crate::test_utils::{TEST_YAS_ERC20_CONTRACT_CLASS_HASH, YAS_ERC20_CONTRACT_PATH};

pub struct YASERC20Factory {}

impl<'a> YASERC20Factory {
    pub fn new() -> Self {
        YASERC20Factory {}
    }
}

impl StateFactory for YASERC20Factory {
    fn args(&self) -> Vec<Felt> {
        // 'YAS', '$YAS', 4000000000000000000, OWNER()
        vec![
            string_to_felt("YAS").unwrap(),
            string_to_felt("$YAS").unwrap(),
            Felt::from(4000000000000000000u128),
            Felt::from(0),
            Signers::Alice.into(),
        ]
    }

    fn class_hash(&self) -> &'static str {
        TEST_YAS_ERC20_CONTRACT_CLASS_HASH
    }

    fn contract_class(&self) -> ContractClass {
        SierraContractClassV1::from_file(YAS_ERC20_CONTRACT_PATH).into()
    }

    fn name() -> &'static str {
        "YASERC20Factory"
    }
}
