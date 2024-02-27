use starknet_types_core::felt::Felt;

use crate::execution::contract_class::{ContractClass, SierraContractClassV1};
use crate::test_utils::testing_context::string_utils::string_to_felt;
use crate::test_utils::testing_context::{Signers, StateFactory};
use crate::test_utils::{TEST_YAS_ERC20_CONTRACT_CLASS_HASH, YAS_ERC20_CONTRACT_PATH};

#[derive(Debug, Clone, Default)]
pub struct YASERC20Factory<'a> {
    name: Option<&'a str>,
    symbol: Option<&'a str>,
    initial_supply: Option<Felt>,
    recipient: Option<Signers>,
}

impl<'a> YASERC20Factory<'a> {
    pub fn new(
        name: Option<&'a str>,
        symbol: Option<&'a str>,
        initial_supply: Option<Felt>,
        recipient: Option<Signers>,
    ) -> Self {
        YASERC20Factory { name, symbol, initial_supply, recipient }
    }
}

impl StateFactory for YASERC20Factory<'_> {
    fn args(&self) -> Vec<Felt> {
        vec![
            string_to_felt(self.name.unwrap_or("YAS")).unwrap(),
            string_to_felt(self.symbol.unwrap_or("$YAS")).unwrap(),
            self.initial_supply.unwrap_or(Felt::from(4000000000000000000u128)),
            Felt::from(0),
            self.recipient.unwrap_or(Signers::Alice).into(),
        ]
    }

    fn class_hash(&self) -> &'static str {
        TEST_YAS_ERC20_CONTRACT_CLASS_HASH
    }

    fn contract_class(&self) -> ContractClass {
        SierraContractClassV1::from_file(YAS_ERC20_CONTRACT_PATH).into()
    }

    fn name(&self) -> String {
        format!("{}{}", "YASERC20Factory", self.name.unwrap_or(""))
    }
}
