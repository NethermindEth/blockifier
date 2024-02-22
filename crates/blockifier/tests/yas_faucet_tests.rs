use blockifier::test_utils::TestContext;
use starknet_api::core::ContractAddress;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct YasTokenContract {
    pub contract_address: ContractAddress,
}

impl YasTokenContract {
    pub fn new(_context: &mut TestContext) -> Self {
        Self::default()
    }
}

pub struct YasFaucetContract {
    pub contract_address: ContractAddress,
}

impl YasFaucetContract {
    pub fn new(_context: &mut TestContext) -> Self {
        Self { contract_address: ContractAddress::from([1; 32]) }
    }
}

pub fn setup_faucet_test_env() -> (YasTokenContract, YasFaucetContract) {
    let context = TestContext::new();
    let token_contract = YasTokenContract::new(ContractAddress::from([0; 32]));
    let faucet_contract = YasFaucetContract::new(ContractAddress::from([1; 32]));
    (token_contract, faucet_contract)
}
