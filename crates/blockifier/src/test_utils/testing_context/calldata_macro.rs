use starknet_api::hash::StarkFelt;
#[macro_export]
macro_rules! s_calldata {
    ($($field:expr),+) => {
        {
            let mut calldata = Vec::<::cairo_serde::traits::UniversalFelt>::new();
            $(calldata.extend(::cairo_serde::traits::CairoSerializable::serialize_cairo(&$field));)*
            calldata
        }
    }

}

#[macro_export]
macro_rules! s_calldata_felt {
    ($($field:expr),+) => {
        {
            let mut calldata = Vec::<::cairo_serde::traits::UniversalFelt>::new();
            $(calldata.extend(::cairo_serde::traits::CairoSerializable::serialize_cairo(&$field));)*
            calldata.iter().map(|felt| felt.as_felt()).collect::<Vec<Felt>>()
        }
    }
}

#[macro_export]
macro_rules! s_calldata_starkfelt {
    ($($field:expr),+) => {
        {
            let mut calldata = Vec::<::cairo_serde::traits::UniversalFelt>::new();
            $(calldata.extend(::cairo_serde::traits::CairoSerializable::serialize_cairo(&$field));)*
            calldata.iter().map(|felt| felt.as_starkfelt()).collect::<Vec<StarkFelt>>()
        }
    }
}
