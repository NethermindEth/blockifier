use starknet_types_core::felt::{Felt, FromStrError};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_to_hex() {
        assert_eq!(string_to_hex_with_prefix("hello"), "0x68656c6c6f");
    }
}
