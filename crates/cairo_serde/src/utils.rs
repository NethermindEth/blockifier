use primitive_types::U256;
use starknet_api::core::ContractAddress;
use starknet_api::hash::StarkFelt;
use starknet_types_core::felt::{Felt, FromStrError};

pub fn starkfelt_to_felt(starkfelt: StarkFelt) -> Felt {
    Felt::from_bytes_be_slice(starkfelt.bytes())
}

pub fn felt_to_starkfelt(felt: Felt) -> StarkFelt {
    StarkFelt::new(felt.to_bytes_be()).unwrap()
}

pub fn string_to_felt(s: &str) -> Result<Felt, FromStrError> {
    Felt::from_hex(&string_to_hex_with_prefix(s))
}

pub fn string_to_hex_with_prefix(s: &str) -> String {
    let mut hex = String::from("0x");
    for c in s.chars() {
        hex.push_str(&format!("{:02x}", c as u8));
    }
    hex
}

pub fn get_hi_lo_from_u256(value: U256) -> (u128, u128) {
    let [a, b, c, d] = value.0;

    let full_vec = a
        .to_le_bytes()
        .iter()
        .chain(b.to_le_bytes().iter())
        .chain(c.to_le_bytes().iter())
        .chain(d.to_le_bytes().iter())
        .map(|e| *e)
        .collect::<Vec<_>>();

    let hi = u128::from_le_bytes(full_vec[16..32].try_into().unwrap());
    let lo = u128::from_le_bytes(full_vec[0..16].try_into().unwrap());

    (hi, lo)
}

pub fn contract_address_to_felt(contract_address: ContractAddress) -> Felt {
    Felt::from_bytes_be_slice(contract_address.0.key().bytes())
}
