use starknet_api::core::ContractAddress;
use starknet_types_core::felt::Felt;

use crate::execution::contract_class::{ContractClass, SierraContractClassV1};
use crate::execution::sierra_utils::contract_address_to_felt;
use crate::test_utils::testing_context::{Signers, StateFactory};
use crate::test_utils::{TEST_YAS_FAUCET_CONTRACT_CLASS_HASH, YAS_FAUCET_CONTRACT_PATH};

#[derive(Debug, Clone, Default)]
pub struct YASFaucetFactory {
    args: Vec<Felt>,
}

impl YASFaucetFactory {
    pub fn new(yas_token_address: ContractAddress) -> Self {
        // OWNER(), yas_token.contract_address, 1000, 86400
        YASFaucetFactory {
            args: vec![
                Signers::Alice.into(),
                contract_address_to_felt(yas_token_address),
                Felt::from(1000u128),
                Felt::from(0u128),
                Felt::from(86400u128),
            ],
        }
    }
}

impl StateFactory for YASFaucetFactory {
    fn args(&self) -> Vec<Felt> {
        self.args.clone()
    }

    fn class_hash(&self) -> &'static str {
        TEST_YAS_FAUCET_CONTRACT_CLASS_HASH
    }

    fn contract_class(&self) -> ContractClass {
        SierraContractClassV1::from_file(YAS_FAUCET_CONTRACT_PATH).into()
    }

    fn name(&self) -> String {
        String::from("YASFaucetFactory")
    }
}
