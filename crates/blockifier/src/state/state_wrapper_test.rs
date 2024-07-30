use starknet_api::core::{ClassHash, CompiledClassHash, ContractAddress, Nonce};
use starknet_api::hash::{StarkFelt, StarkHash};
use starknet_api::state::StorageKey;

use crate::state::cached_state::CachedState;
use crate::state::state_api::{State, StateReader};
use crate::state::state_wrapper::DynStateWrapper;

#[test]
fn set_class_hash_at() {
    let contract_address = ContractAddress::from(1u128);
    let mut state_1 = CachedState::default();

    state_1.set_class_hash_at(contract_address, ClassHash(StarkHash::from(1u128))).unwrap();

    let mut state_2 = DynStateWrapper::new(&mut state_1);

    assert_eq!(
        state_2.get_raw_class_hash_at(contract_address).unwrap(),
        ClassHash(StarkHash::from(1u128))
    );

    assert_eq!(
        state_2.get_class_hash_at(contract_address).unwrap(),
        ClassHash(StarkHash::from(1u128))
    );

    state_2.set_class_hash_at(contract_address, ClassHash(StarkHash::from(2u128))).unwrap();

    assert_eq!(
        state_2.get_raw_class_hash_at(ContractAddress::from(1u128)).unwrap(),
        ClassHash(StarkHash::from(1u128))
    );

    assert_eq!(
        state_2.get_class_hash_at(ContractAddress::from(1u128)).unwrap(),
        ClassHash(StarkHash::from(2u128))
    );

    state_2.commit().unwrap();

    assert_eq!(
        state_2.get_class_hash_at(ContractAddress::from(1u128)).unwrap(),
        ClassHash(StarkHash::from(2u128))
    );

    drop(state_2);

    assert_eq!(
        state_1.get_class_hash_at(ContractAddress::from(1u128)).unwrap(),
        ClassHash(StarkHash::from(2u128))
    );
}

#[test]
fn test_nonce() {
    let contract_address = ContractAddress::from(1u128);

    let mut state_1 = CachedState::default();

    state_1.increment_nonce(contract_address).unwrap();

    let mut state_2 = DynStateWrapper::new(&mut state_1);

    assert_eq!(state_2.get_raw_nonce_at(contract_address).unwrap(), Nonce(StarkFelt::from(1u128)));

    assert_eq!(state_2.get_nonce_at(contract_address).unwrap(), Nonce(StarkFelt::from(1u128)));

    state_2.increment_nonce(contract_address).unwrap();

    assert_eq!(state_2.get_raw_nonce_at(contract_address).unwrap(), Nonce(StarkFelt::from(1u128)));

    assert_eq!(state_2.get_nonce_at(contract_address).unwrap(), Nonce(StarkFelt::from(2u128)));

    state_2.commit().unwrap();

    assert_eq!(state_2.get_nonce_at(contract_address).unwrap(), Nonce(StarkFelt::from(2u128)));

    drop(state_2);

    assert_eq!(state_1.get_nonce_at(contract_address).unwrap(), Nonce(StarkFelt::from(2u128)));
}

#[test]
fn test_storage() {
    let contract_address = ContractAddress::from(1u128);
    let storage_key = StorageKey::from(1u128);

    let mut state_1 = CachedState::default();

    state_1.set_storage_at(contract_address, storage_key, StarkFelt::from(1u128)).unwrap();

    let mut state_2 = DynStateWrapper::new(&mut state_1);

    assert_eq!(
        state_2.get_raw_storage_at(contract_address, storage_key).unwrap(),
        StarkFelt::from(1u128)
    );

    assert_eq!(
        state_2.get_storage_at(contract_address, storage_key).unwrap(),
        StarkFelt::from(1u128)
    );

    state_2.set_storage_at(contract_address, storage_key, StarkFelt::from(2u128)).unwrap();

    assert_eq!(
        state_2.get_raw_storage_at(contract_address, storage_key).unwrap(),
        StarkFelt::from(1u128)
    );

    assert_eq!(
        state_2.get_storage_at(contract_address, storage_key).unwrap(),
        StarkFelt::from(2u128)
    );

    state_2.commit().unwrap();

    assert_eq!(
        state_2.get_storage_at(contract_address, storage_key).unwrap(),
        StarkFelt::from(2u128)
    );

    drop(state_2);

    assert_eq!(
        state_1.get_storage_at(contract_address, storage_key).unwrap(),
        StarkFelt::from(2u128)
    );
}

#[test]
fn test_compiled_class_hash() {
    let class_hash = ClassHash(StarkHash::from(1u128));
    let compiled_class_hash = CompiledClassHash(StarkHash::from(2u128));

    let mut state_1 = CachedState::default();

    state_1.set_compiled_class_hash(class_hash, compiled_class_hash).unwrap();

    let mut state_2 = DynStateWrapper::new(&mut state_1);

    assert_eq!(state_2.get_raw_compiled_class_hash(class_hash).unwrap(), compiled_class_hash);

    assert_eq!(state_2.get_compiled_class_hash(class_hash).unwrap(), compiled_class_hash);

    state_2.set_compiled_class_hash(class_hash, CompiledClassHash(StarkHash::from(3u128))).unwrap();

    assert_eq!(state_2.get_raw_compiled_class_hash(class_hash).unwrap(), compiled_class_hash);

    assert_eq!(
        state_2.get_compiled_class_hash(class_hash).unwrap(),
        CompiledClassHash(StarkHash::from(3u128))
    );

    state_2.commit().unwrap();

    assert_eq!(
        state_2.get_compiled_class_hash(class_hash).unwrap(),
        CompiledClassHash(StarkHash::from(3u128))
    );

    drop(state_2);

    assert_eq!(
        state_1.get_compiled_class_hash(class_hash).unwrap(),
        CompiledClassHash(StarkHash::from(3u128))
    );
}

#[test]
fn test_multiple_nonce_updates() {
    let contract_address = ContractAddress::from(1u128);

    let mut state_1 = CachedState::default();

    state_1.increment_nonce(contract_address).unwrap();

    let mut state_2 = DynStateWrapper::new(&mut state_1);

    assert_eq!(state_2.get_raw_nonce_at(contract_address).unwrap(), Nonce(StarkFelt::from(1u128)));

    assert_eq!(state_2.get_nonce_at(contract_address).unwrap(), Nonce(StarkFelt::from(1u128)));

    state_2.increment_nonce(contract_address).unwrap();
    state_2.increment_nonce(contract_address).unwrap();

    assert_eq!(state_2.get_raw_nonce_at(contract_address).unwrap(), Nonce(StarkFelt::from(1u128)));

    assert_eq!(state_2.get_nonce_at(contract_address).unwrap(), Nonce(StarkFelt::from(3u128)));

    state_2.commit().unwrap();

    assert_eq!(state_2.get_nonce_at(contract_address).unwrap(), Nonce(StarkFelt::from(3u128)));

    drop(state_2);

    assert_eq!(state_1.get_nonce_at(contract_address).unwrap(), Nonce(StarkFelt::from(3u128)));
}
