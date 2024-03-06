use cairo_serde::get_hi_lo_from_u256;
use cairo_serde_macro::CairoSerde;
use primitive_types::U256;
use starknet_types_core::felt::Felt;

#[derive(Debug, Copy, Clone, Default, CairoSerde)]
pub struct YasU256 {
    pub lo: u128,
    pub hi: u128,
}

impl YasU256 {
    pub fn from_u128(value: u128) -> Self {
        Self { lo: value, hi: 0 }
    }
}

#[derive(Debug, Copy, Clone, Default, CairoSerde)]
pub struct YasI8 {
    pub value: u8,
    pub sign: bool,
}

#[derive(Debug, Copy, Clone, Default, CairoSerde)]
pub struct YasI16 {
    pub value: u16,
    pub sign: bool,
}

#[derive(Debug, Copy, Clone, Default, CairoSerde)]
pub struct YasI32 {
    pub value: u32,
    pub sign: bool,
}

impl YasI32 {
    pub fn from_i32(value: i32) -> Self {
        Self { value: value.abs() as u32, sign: value < 0 }
    }
}

#[derive(Debug, Copy, Clone, Default, CairoSerde)]
pub struct YasI64 {
    pub value: u64,
    pub sign: bool,
}

#[derive(Debug, Copy, Clone, Default, CairoSerde)]
pub struct YasI128 {
    pub value: u128,
    pub sign: bool,
}

#[derive(Debug, Copy, Clone, Default, CairoSerde)]
pub struct YasI256 {
    pub value: YasU256,
    pub sign: bool,
}
#[derive(Debug, Copy, Clone, Default, CairoSerde)]
pub struct FixedType {
    pub value: YasU256,
    pub sign: bool,
}

impl FixedType {
    pub fn from_u128(value: u128) -> Self {
        Self { value: YasU256::from_u128(value), sign: false }
    }

    pub fn from_i128(value: i128) -> Self {
        Self { value: YasU256::from_u128(value.abs() as u128), sign: value < 0 }
    }

    pub fn from_yas_u256(value: YasU256) -> Self {
        Self { value, sign: false }
    }

    pub fn from_u256(value: U256) -> Self {
        let (hi, lo) = get_hi_lo_from_u256(value);

        Self { value: YasU256 { lo, hi }, sign: false }
    }
}
