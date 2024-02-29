use starknet_api::hash::StarkFelt;
use starknet_types_core::felt::{Felt, FromStrError};

use crate::execution::sierra_utils::felt_to_starkfelt;

pub fn string_to_hex_with_prefix(s: &str) -> String {
    let mut hex = String::from("0x");
    for c in s.chars() {
        hex.push_str(&format!("{:02x}", c as u8));
    }
    hex
}

pub fn string_to_felt(s: &str) -> Result<Felt, FromStrError> {
    Felt::from_hex(&string_to_hex_with_prefix(s))
}

pub fn string_to_starkfelt(s: &str) -> Result<StarkFelt, FromStrError> {
    Ok(felt_to_starkfelt(string_to_felt(s)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_to_hex() {
        assert_eq!(string_to_hex_with_prefix("hello"), "0x68656c6c6f");
    }
}
