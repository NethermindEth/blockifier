use starknet_api::core::{ContractAddress, PatriciaKey};
use starknet_api::hash::{StarkFelt, StarkHash};
use starknet_api::patricia_key;
use starknet_types_core::felt::Felt;

use crate::execution::sierra_utils::{contract_address_to_felt, felt_to_starkfelt};

#[derive(Debug, Clone, Copy)]
pub enum Signers {
    Alice,
    Bob,
    Charlie,
}

impl Signers {
    pub fn get_address(&self) -> ContractAddress {
        match self {
            Signers::Alice => ContractAddress(patricia_key!(0x001u128)),
            Signers::Bob => ContractAddress(patricia_key!(0x002u128)),
            Signers::Charlie => ContractAddress(patricia_key!(0x003u128)),
        }
    }
}

impl Into<ContractAddress> for Signers {
    fn into(self) -> ContractAddress {
        self.get_address()
    }
}

impl Into<Felt> for Signers {
    fn into(self) -> Felt {
        contract_address_to_felt(self.get_address())
    }
}

impl Into<StarkFelt> for Signers {
    fn into(self) -> StarkFelt {
        felt_to_starkfelt(contract_address_to_felt(self.get_address()))
    }
}
