use crate::test_utils::testing_context::{
    StateFactory, YASERC20Factory, YASFactory, YASFaucetFactory,
};

#[allow(non_snake_case)]
pub fn FAUCET_NAME() -> String {
    YASFaucetFactory::default().name()
}

#[allow(non_snake_case)]
pub fn YASERC20_NAME() -> String {
    YASERC20Factory::default().name()
}

#[allow(non_snake_case)]
pub fn FACTORY_NAME() -> String {
    YASFactory::default().name()
}

#[derive(Debug, Clone, Copy)]
pub enum FeeAmount {
    Custom,
    Low,
    Medium,
    High,
    Other(u32, u32),
}

impl FeeAmount {
    pub fn fee_amount(&self) -> u32 {
        match self {
            FeeAmount::Custom => 100,
            FeeAmount::Low => 500,
            FeeAmount::Medium => 3000,
            FeeAmount::High => 10000,
            FeeAmount::Other(amount, _) => *amount,
        }
    }

    pub fn tick_spacing(&self) -> u32 {
        match self {
            FeeAmount::Custom => 2,
            FeeAmount::Low => 10,
            FeeAmount::Medium => 60,
            FeeAmount::High => 200,
            FeeAmount::Other(_, amount) => *amount,
        }
    }
}
