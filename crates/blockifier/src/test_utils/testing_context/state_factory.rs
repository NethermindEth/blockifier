use starknet_api::class_hash;
use starknet_api::core::{ClassHash, ContractAddress, PatriciaKey};
use starknet_api::hash::StarkHash;
use starknet_types_core::felt::Felt;

use crate::execution::contract_class::ContractClass;
use crate::execution::sierra_utils::felt_to_starkfelt;
use crate::state::cached_state::CachedState;
use crate::test_utils::dict_state_reader::DictStateReader;
use crate::test_utils::testing_context::deploy_contract;
pub trait StateFactory {
    fn create_state(&self, state: &mut CachedState<DictStateReader>) -> ContractAddress {
        if state.state.class_hash_to_class.get(&class_hash!(self.class_hash())).is_none() {
            state
                .state
                .class_hash_to_class
                .insert(class_hash!(self.class_hash()), self.contract_class());
        }

        let class_hash = Felt::from_hex(self.class_hash()).unwrap();

        let (contract_address, _) =
            deploy_contract(state, class_hash, Felt::from(0), self.args().as_slice()).unwrap();

        let contract_address =
            ContractAddress(PatriciaKey::try_from(felt_to_starkfelt(contract_address)).unwrap());

        contract_address
    }

    fn args(&self) -> Vec<Felt> {
        vec![]
    }

    fn class_hash(&self) -> &'static str;

    fn contract_class(&self) -> ContractClass;

    fn name() -> &'static str;
}
