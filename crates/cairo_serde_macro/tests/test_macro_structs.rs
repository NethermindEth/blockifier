use cairo_serde::string_to_felt;
use cairo_serde::traits::CairoSerializable;
use cairo_serde_macro::CairoSerde;
use starknet_types_core::felt::Felt;

#[derive(Debug, Default, Clone, CairoSerde)]
pub struct Struct0 {
    u8f: u8,
    u16: u8,
}

#[test]
fn test_struct_serialization() {
    let s = Struct0 { u8f: 0x01, u16: 0x02 };

    assert_eq!(s.to_vec_felt(), vec![Felt::from(0x01u8), Felt::from(0x02u8)])
}

#[derive(Debug, Default, Clone, CairoSerde)]
pub struct Struct1 {
    str_f: String,
    s0: Struct0,
}

#[test]
fn test_nested_struct_serialization() {
    let s = Struct1 { str_f: "test".to_string(), s0: Struct0 { u8f: 0x01, u16: 0x02 } };

    assert_eq!(
        s.to_vec_felt(),
        vec![string_to_felt("test").unwrap(), Felt::from(0x01u8), Felt::from(0x02u8),]
    )
}
