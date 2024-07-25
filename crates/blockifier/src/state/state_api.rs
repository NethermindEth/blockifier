use std::collections::{HashMap, HashSet};

use starknet_api::core::{ClassHash, CompiledClassHash, ContractAddress, Nonce};
use starknet_api::hash::StarkFelt;
use starknet_api::state::StorageKey;
use starknet_api::StarknetApiError;
use starknet_core::types::FieldElement;

use crate::abi::abi_utils::get_fee_token_var_address;
use crate::abi::sierra_types::next_storage_key;
use crate::execution::contract_class::ContractClass;
use crate::state::errors::StateError;

pub type StateResult<T> = Result<T, StateError>;

// TODO(barak, 01/10/2023): Remove this enum from here once it can be used from starknet_api.
pub enum DataAvailabilityMode {
    L1 = 0,
    L2 = 1,
}

/// A read-only API for accessing Starknet global state.
///
/// The `self` argument is mutable for flexibility during reads (for example, caching reads),
/// and to allow for the `State` trait below to also be considered a `StateReader`.
pub trait StateReader {
    /// Returns the storage value under the given key in the given contract instance (represented by
    /// its address).
    /// Default: 0 for an uninitialized contract address.
    fn get_storage_at(
        &self,
        contract_address: ContractAddress,
        key: StorageKey,
    ) -> StateResult<StarkFelt>;

    /// Returns the nonce of the given contract instance.
    /// Default: 0 for an uninitialized contract address.
    fn get_nonce_at(&self, contract_address: ContractAddress) -> StateResult<Nonce>;

    /// Returns the class hash of the contract class at the given contract instance.
    /// Default: 0 (uninitialized class hash) for an uninitialized contract address.
    fn get_class_hash_at(&self, contract_address: ContractAddress) -> StateResult<ClassHash>;

    /// Returns the contract class of the given class hash.
    fn get_compiled_contract_class(&self, class_hash: ClassHash) -> StateResult<ContractClass>;

    /// Returns the compiled class hash of the given class hash.
    fn get_compiled_class_hash(&self, class_hash: ClassHash) -> StateResult<CompiledClassHash>;

    /// Returns the storage value representing the balance (in fee token) at the given address.
    // TODO(Dori, 1/7/2023): When a standard representation for large integers is set, change the
    //    return type to that.
    // TODO(Dori, 1/9/2023): NEW_TOKEN_SUPPORT Determine fee token address based on tx version,
    //   once v3 is introduced.
    fn get_fee_token_balance(
        &mut self,
        contract_address: ContractAddress,
        fee_token_address: ContractAddress,
    ) -> Result<(StarkFelt, StarkFelt), StateError> {
        let low_key = get_fee_token_var_address(contract_address);
        let high_key = next_storage_key(&low_key)?;
        let low = self.get_storage_at(fee_token_address, low_key)?;
        let high = self.get_storage_at(fee_token_address, high_key)?;

        Ok((low, high))
    }
}

/// A class defining the API for writing to Starknet global state.
///
/// Reader functionality should be delegated to the associated type; which is passed in by
/// dependency-injection.
pub trait State: StateReader {
    /// Sets the storage value under the given key in the given contract instance.
    fn set_storage_at(
        &mut self,
        contract_address: ContractAddress,
        key: StorageKey,
        value: StarkFelt,
    ) -> StateResult<()>;

    /// Increments the nonce of the given contract instance.
    fn increment_nonce(&mut self, contract_address: ContractAddress) -> StateResult<()>;

    /// Allocates the given address to the given class hash.
    /// Raises an exception if the address is already assigned;
    /// meaning: this is a write once action.
    fn set_class_hash_at(
        &mut self,
        contract_address: ContractAddress,
        class_hash: ClassHash,
    ) -> StateResult<()>;

    /// Sets the given contract class under the given class hash.
    fn set_contract_class(
        &mut self,
        class_hash: ClassHash,
        contract_class: ContractClass,
    ) -> StateResult<()>;

    /// Sets the given compiled class hash under the given class hash.
    fn set_compiled_class_hash(
        &mut self,
        class_hash: ClassHash,
        compiled_class_hash: CompiledClassHash,
    ) -> StateResult<()>;

    /// Marks the given set of PC values as visited for the given class hash.
    // TODO(lior): Once we have a BlockResources object, move this logic there. Make sure reverted
    //   entry points do not affect the final set of PCs.
    fn add_visited_pcs(&mut self, class_hash: ClassHash, pcs: &HashSet<usize>);
}

pub struct DynStateWrapper<'a> {
    pub state: &'a mut dyn State,

    pub storage_updates: HashMap<(ContractAddress, StorageKey), StarkFelt>,
    pub nonce_updates: HashMap<ContractAddress, u128>,
    pub class_hashes: HashMap<ContractAddress, ClassHash>,
    pub contract_classes: HashMap<ClassHash, ContractClass>,
    pub compiled_class_hashes: HashMap<ClassHash, CompiledClassHash>,
}

impl<'a> DynStateWrapper<'a> {
    pub fn new(state: &'a mut dyn State) -> Self {
        Self {
            state,
            storage_updates: Default::default(),
            nonce_updates: Default::default(),
            class_hashes: Default::default(),
            contract_classes: Default::default(),
            compiled_class_hashes: Default::default(),
        }
    }

    pub fn commit(&mut self) -> StateResult<()> {
        for (k, v) in &self.storage_updates {
            self.state.set_storage_at(k.0, k.1, *v)?
        }

        for (k, v) in &self.nonce_updates {
            for _ in 0..*v {
                self.state.increment_nonce(*k)?;
            }
        }

        for (k, v) in &self.class_hashes {
            self.state.set_class_hash_at(*k, *v)?;
        }

        for (k, v) in &self.contract_classes {
            self.state.set_contract_class(*k, v.clone())?;
        }

        for (k, v) in &self.compiled_class_hashes {
            self.state.set_compiled_class_hash(*k, *v)?;
        }

        Ok(())
    }

    pub fn abort(&mut self) {
        self.storage_updates.clear();
        self.nonce_updates.clear();
        self.class_hashes.clear();
        self.contract_classes.clear();
        self.compiled_class_hashes.clear();
    }
}

impl StateReader for DynStateWrapper<'_> {
    fn get_storage_at(
        &self,
        contract_address: ContractAddress,
        key: StorageKey,
    ) -> StateResult<StarkFelt> {
        Ok(self
            .storage_updates
            .get(&(contract_address, key))
            .map(|e| *e)
            .unwrap_or(self.state.get_storage_at(contract_address, key)?))
    }

    fn get_nonce_at(&self, contract_address: ContractAddress) -> StateResult<Nonce> {
        let current_nonce = FieldElement::from(self.state.get_nonce_at(contract_address)?.0);

        let delta = *self.nonce_updates.get(&contract_address).unwrap_or(&0u128);
        let delta = FieldElement::from(delta);

        // Check if an overflow occurred during increment.
        match StarkFelt::from(current_nonce + FieldElement::ONE * delta) {
            StarkFelt::ZERO => Err(StateError::from(StarknetApiError::OutOfRange {
                string: format!("{:?}", current_nonce),
            })),
            incremented_felt => Ok(Nonce(incremented_felt)),
        }
    }

    fn get_class_hash_at(&self, contract_address: ContractAddress) -> StateResult<ClassHash> {
        Ok(self
            .class_hashes
            .get(&contract_address)
            .map(|e| *e)
            .unwrap_or(self.state.get_class_hash_at(contract_address)?))
    }

    fn get_compiled_contract_class(&self, class_hash: ClassHash) -> StateResult<ContractClass> {
        Ok(self
            .contract_classes
            .get(&class_hash)
            .map(|e| e.clone())
            .unwrap_or(self.state.get_compiled_contract_class(class_hash)?))
    }

    fn get_compiled_class_hash(&self, class_hash: ClassHash) -> StateResult<CompiledClassHash> {
        Ok(self
            .compiled_class_hashes
            .get(&class_hash)
            .map(|e| e.clone())
            .unwrap_or(self.state.get_compiled_class_hash(class_hash)?))
    }

    fn get_fee_token_balance(
        &mut self,
        contract_address: ContractAddress,
        fee_token_address: ContractAddress,
    ) -> Result<(StarkFelt, StarkFelt), StateError> {
        self.state.get_fee_token_balance(contract_address, fee_token_address)
    }
}

impl State for DynStateWrapper<'_> {
    fn set_storage_at(
        &mut self,
        contract_address: ContractAddress,
        key: StorageKey,
        value: StarkFelt,
    ) -> StateResult<()> {
        self.storage_updates.insert((contract_address, key), value);

        Ok(())
    }

    fn increment_nonce(&mut self, contract_address: ContractAddress) -> StateResult<()> {
        let value = self.nonce_updates.get(&contract_address);

        if let Some(value) = value {
            self.nonce_updates.insert(contract_address, value + 1);
        } else {
            self.nonce_updates.insert(contract_address, 1);
        }

        Ok(())
    }

    fn set_class_hash_at(
        &mut self,
        contract_address: ContractAddress,
        class_hash: ClassHash,
    ) -> StateResult<()> {
        self.class_hashes.insert(contract_address, class_hash);

        Ok(())
    }

    fn set_contract_class(
        &mut self,
        class_hash: ClassHash,
        contract_class: ContractClass,
    ) -> StateResult<()> {
        self.contract_classes.insert(class_hash, contract_class);

        Ok(())
    }

    fn set_compiled_class_hash(
        &mut self,
        class_hash: ClassHash,
        compiled_class_hash: CompiledClassHash,
    ) -> StateResult<()> {
        self.compiled_class_hashes.insert(class_hash, compiled_class_hash);

        Ok(())
    }

    fn add_visited_pcs(&mut self, class_hash: ClassHash, pcs: &HashSet<usize>) {
        self.state.add_visited_pcs(class_hash, pcs)
    }
}
