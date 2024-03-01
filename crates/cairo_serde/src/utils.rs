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
