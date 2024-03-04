use primitive_types::U256;
use starknet_api::hash::StarkFelt;
use starknet_types_core::felt::Felt;

use crate::utils::{felt_to_starkfelt, starkfelt_to_felt, string_to_felt};

#[derive(Clone, Debug)]
pub enum UniversalFelt {
    Felt(Felt),
    StarkFelt(StarkFelt),
}

impl From<Felt> for UniversalFelt {
    fn from(felt: Felt) -> Self {
        UniversalFelt::Felt(felt)
    }
}

impl From<StarkFelt> for UniversalFelt {
    fn from(stark_felt: StarkFelt) -> Self {
        UniversalFelt::StarkFelt(stark_felt)
    }
}

impl UniversalFelt {
    pub fn as_felt(&self) -> Felt {
        match self {
            UniversalFelt::Felt(felt) => *felt,
            UniversalFelt::StarkFelt(stark_felt) => starkfelt_to_felt(*stark_felt),
        }
    }

    pub fn as_starkfelt(&self) -> StarkFelt {
        match self {
            UniversalFelt::Felt(felt) => felt_to_starkfelt(*felt),
            UniversalFelt::StarkFelt(stark_felt) => *stark_felt,
        }
    }
}

pub trait CairoSerializable {
    fn serialize_cairo(&self) -> Vec<UniversalFelt>;

    fn to_vec_felt(&self) -> Vec<Felt> {
        let serialized = self.serialize_cairo();
        serialized.into_iter().map(|felt| felt.as_felt()).collect()
    }

    fn to_vec_starkfelt(&self) -> Vec<StarkFelt> {
        let serialized = self.serialize_cairo();
        serialized.into_iter().map(|felt| felt.as_starkfelt()).collect()
    }

    fn into_vec<T>(&self) -> Vec<T>
    where
        T: From<UniversalFelt>,
    {
        let serialized = self.serialize_cairo();
        serialized.into_iter().map(|felt| felt.into()).collect()
    }
}

impl From<UniversalFelt> for Felt {
    fn from(universal_felt: UniversalFelt) -> Self {
        universal_felt.as_felt()
    }
}

impl From<UniversalFelt> for StarkFelt {
    fn from(universal_felt: UniversalFelt) -> Self {
        universal_felt.as_starkfelt()
    }
}

macro_rules! impl_for_primitive {
    ($type_name:ty) => {
        impl CairoSerializable for $type_name {
            fn serialize_cairo(&self) -> Vec<UniversalFelt> {
                vec![Felt::from(*self).into()]
            }
        }
    };
}

impl_for_primitive!(u8);
impl_for_primitive!(u16);
impl_for_primitive!(u32);
impl_for_primitive!(u64);
impl_for_primitive!(u128);
impl_for_primitive!(i8);
impl_for_primitive!(i16);
impl_for_primitive!(i32);
impl_for_primitive!(i64);
impl_for_primitive!(i128);
impl_for_primitive!(bool);

impl CairoSerializable for String {
    fn serialize_cairo(&self) -> Vec<UniversalFelt> {
        vec![string_to_felt(self).unwrap().into()]
    }
}

impl<T> CairoSerializable for Vec<T>
where
    T: CairoSerializable,
{
    fn serialize_cairo(&self) -> Vec<UniversalFelt> {
        let mut result = vec![UniversalFelt::from(Felt::from(self.len() as u32))];

        for item in self {
            result.extend(item.serialize_cairo());
        }

        result
    }
}

impl CairoSerializable for UniversalFelt {
    fn serialize_cairo(&self) -> Vec<UniversalFelt> {
        vec![self.clone()]
    }
}

impl CairoSerializable for () {
    fn serialize_cairo(&self) -> Vec<UniversalFelt> {
        vec![]
    }
}

impl CairoSerializable for StarkFelt {
    fn serialize_cairo(&self) -> Vec<UniversalFelt> {
        vec![(*self).into()]
    }
}

impl CairoSerializable for Felt {
    fn serialize_cairo(&self) -> Vec<UniversalFelt> {
        vec![(*self).into()]
    }
}

impl CairoSerializable for U256 {
    fn serialize_cairo(&self) -> Vec<UniversalFelt> {
        let (hi, lo) = crate::utils::get_hi_lo_from_u256(*self);
        vec![Felt::from(lo).into(), Felt::from(hi).into()]
    }
}
