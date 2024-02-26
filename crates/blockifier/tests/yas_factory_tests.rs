use blockifier::execution::call_info::CallInfo;
use blockifier::test_utils::testing_context::{TestContext, YASFactory};
use blockifier::test_utils::TEST_YAS_POOL_CONTRACT_CLASS_HASH;
use starknet_api::class_hash;
use starknet_api::core::{ClassHash, ContractAddress};
use starknet_api::hash::StarkHash;

// pool class hash can not be zero
pub const POOL_CLASS_HASH_CAN_NOT_BE_ZERO: &str =
    "0x00706f6f6c20636c61737320686173682063616e206e6f74206265207a65726f";

pub fn setup(
    deployer: ContractAddress,
    pool_class_hash: Option<ClassHash>,
) -> (TestContext, CallInfo) {
    let pool_class_hash =
        pool_class_hash.unwrap_or_else(|| class_hash!(TEST_YAS_POOL_CONTRACT_CLASS_HASH));

    let (context, call_info) =
        TestContext::new_with_callinfo(YASFactory::new(deployer, pool_class_hash));

    (context, call_info)
}

pub enum FeeAmount {
    Custom,
    Low,
    Medium,
    High,
}

impl FeeAmount {
    pub fn fee_amount(&self) -> u32 {
        match self {
            FeeAmount::Custom => 100,
            FeeAmount::Low => 500,
            FeeAmount::Medium => 3000,
            FeeAmount::High => 10000,
        }
    }

    pub fn tick_spacing(&self) -> u32 {
        match self {
            FeeAmount::Custom => 2,
            FeeAmount::Low => 10,
            FeeAmount::Medium => 60,
            FeeAmount::High => 200,
        }
    }
}

#[cfg(test)]
mod constructor_tests {
    use blockifier::execution::sierra_utils::felt_to_starkfelt;
    use blockifier::test_utils::testing_context::{StateFactory, OWNER, ZERO};
    use starknet_api::hash::StarkFelt;
    use starknet_types_core::felt::Felt;

    use super::*;

    #[test]
    fn test_fails_when_pool_class_hash_is_zero() {
        let (_, call_info) = setup(OWNER().into(), Some(ClassHash(StarkFelt::from(0u8))));

        assert!(call_info.execution.failed);

        assert_eq!(
            call_info.execution.retdata.0,
            vec![felt_to_starkfelt(Felt::from_hex(POOL_CLASS_HASH_CAN_NOT_BE_ZERO).unwrap())]
        )
    }

    #[test]
    fn test_deployer_should_be_owner() {
        let (mut context, _) = setup(OWNER().into(), None);

        assert_eq!(
            context.call_entry_point(YASFactory::name(), "owner", vec![]),
            vec![OWNER().into()]
        );
    }

    #[test]
    fn test_initial_enabled_fee_amounts() {
        let (mut context, _) = setup(OWNER().into(), None);

        assert_eq!(
            context.call_entry_point(
                YASFactory::name(),
                "fee_amount_tick_spacing",
                vec![StarkFelt::from(FeeAmount::Custom.fee_amount())]
            ),
            vec![Felt::from(FeeAmount::Custom.tick_spacing()), Felt::from(0u8)]
        );

        assert_eq!(
            context.call_entry_point(
                YASFactory::name(),
                "fee_amount_tick_spacing",
                vec![StarkFelt::from(FeeAmount::Low.fee_amount())]
            ),
            vec![Felt::from(FeeAmount::Low.tick_spacing()), Felt::from(0u8)]
        );

        assert_eq!(
            context.call_entry_point(
                YASFactory::name(),
                "fee_amount_tick_spacing",
                vec![StarkFelt::from(FeeAmount::Medium.fee_amount())]
            ),
            vec![Felt::from(FeeAmount::Medium.tick_spacing()), Felt::from(0u8)]
        );

        assert_eq!(
            context.call_entry_point(
                YASFactory::name(),
                "fee_amount_tick_spacing",
                vec![StarkFelt::from(FeeAmount::High.fee_amount())]
            ),
            vec![Felt::from(FeeAmount::High.tick_spacing()), Felt::from(0u8)]
        );
    }

    #[test]
    fn test_emits_all_events() {
        let (context, _) = setup(OWNER().into(), None);

        // OwnerChanged <from> <to>
        assert_eq!(context.get_event(0).unwrap().data, vec![ZERO().into(), OWNER().into()]);

        // FeeAmountEnabled[CUSTOM] <fee_amount> <tick_spacing.lo> <tick_spacing.hi>
        assert_eq!(
            context.get_event(1).unwrap().data,
            vec![
                Felt::from(FeeAmount::Custom.fee_amount()),
                Felt::from(FeeAmount::Custom.tick_spacing()),
                Felt::from(0u8),
            ]
        );

        // FeeAmountEnabled[LOW] <fee_amount> <tick_spacing.lo> <tick_spacing.hi>
        assert_eq!(
            context.get_event(2).unwrap().data,
            vec![
                Felt::from(FeeAmount::Low.fee_amount()),
                Felt::from(FeeAmount::Low.tick_spacing()),
                Felt::from(0u8),
            ]
        );

        // FeeAmountEnabled[MEDIUM] <fee_amount> <tick_spacing.lo> <tick_spacing.hi>
        assert_eq!(
            context.get_event(3).unwrap().data,
            vec![
                Felt::from(FeeAmount::Medium.fee_amount()),
                Felt::from(FeeAmount::Medium.tick_spacing()),
                Felt::from(0u8),
            ]
        );

        // FeeAmountEnabled[HIGH] <fee_amount> <tick_spacing.lo> <tick_spacing.hi>
        assert_eq!(
            context.get_event(4).unwrap().data,
            vec![
                Felt::from(FeeAmount::High.fee_amount()),
                Felt::from(FeeAmount::High.tick_spacing()),
                Felt::from(0u8),
            ]
        );
    }
}
