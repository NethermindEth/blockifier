use starknet_api::core::{ContractAddress, PatriciaKey};
use starknet_types_core::felt::Felt;

use crate::execution::sierra_utils::felt_to_starkfelt;
use crate::state::cached_state::CachedState;
use crate::test_utils::dict_state_reader::DictStateReader;
use crate::test_utils::testing_context::deploy_contract;

pub trait StateFactory {
    fn get_state(&self) -> CachedState<DictStateReader>;

    fn create_state(&self) -> (ContractAddress, CachedState<DictStateReader>) {
        let mut state = self.get_state();

        let class_hash = Felt::from_hex(self.class_hash()).unwrap();

        let (contract_address, _) =
            deploy_contract(&mut state, class_hash, Felt::from(0), self.args().as_slice()).unwrap();

        let contract_address =
            ContractAddress(PatriciaKey::try_from(felt_to_starkfelt(contract_address)).unwrap());

        (contract_address, state)
    }

    fn args(&self) -> Vec<Felt> {
        vec![]
    }

    fn class_hash(&self) -> &'static str;
}
