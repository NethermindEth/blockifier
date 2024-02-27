use blockifier::execution::call_info::CallInfo;
use blockifier::execution::contract_class::SierraContractClassV1;
use blockifier::execution::sierra_utils::felt_to_starkfelt;
use blockifier::test_utils::testing_context::{
    string_to_felt, Signers, StateFactory, TestContext, YASFactory, OTHER, OWNER, TOKEN_A, TOKEN_B,
    ZERO,
};
use blockifier::test_utils::{TEST_YAS_POOL_CONTRACT_CLASS_HASH, YAS_POOL_CONTRACT_PATH};
use starknet_api::class_hash;
use starknet_api::core::{ClassHash, ContractAddress};
use starknet_api::hash::{StarkFelt, StarkHash};
use starknet_types_core::felt::Felt;
use test_case::test_case;

// pool class hash can not be zero
pub const POOL_CLASS_HASH_CAN_NOT_BE_ZERO: &str =
    "0x00706f6f6c20636c61737320686173682063616e206e6f74206265207a65726f";

pub fn setup(
    deployer: ContractAddress,
    pool_class_hash: Option<ClassHash>,
) -> (TestContext, CallInfo) {
    let pool_class_hash =
        pool_class_hash.unwrap_or_else(|| class_hash!(TEST_YAS_POOL_CONTRACT_CLASS_HASH));

    let (mut context, call_info) =
        TestContext::new_with_callinfo(YASFactory::new(deployer, pool_class_hash));

    context.add_manual_class_hash(
        pool_class_hash,
        SierraContractClassV1::from_file(YAS_POOL_CONTRACT_PATH).into(),
    );

    (context, call_info)
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

#[cfg(test)]
mod constructor_tests {
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

#[test_case(
    FeeAmount::Custom;
    "fee_amount_custom"
)]
#[test_case(
    FeeAmount::Low;
    "fee_amount_low"
)]
#[test_case(
    FeeAmount::Medium;
    "fee_amount_medium"
)]
#[test_case(
    FeeAmount::High;
    "fee_amount_high"
)]
fn test_create_pool(fee_amount: FeeAmount) {
    let (mut context, _) = setup(OWNER().into(), None);

    context.clean_events();
    context.set_caller(OTHER().into());

    let pool_deployed = context.call_entry_point(
        YASFactory::name(),
        "create_pool",
        vec![TOKEN_A().into(), TOKEN_B().into(), StarkFelt::from(fee_amount.fee_amount())],
    );

    let pool_token_a_token_b = context.call_entry_point(
        YASFactory::name(),
        "pool",
        vec![TOKEN_A().into(), TOKEN_B().into(), StarkFelt::from(fee_amount.fee_amount())],
    );

    let pool_token_b_token_a = context.call_entry_point(
        YASFactory::name(),
        "pool",
        vec![TOKEN_B().into(), TOKEN_A().into(), StarkFelt::from(fee_amount.fee_amount())],
    );

    assert_eq!(pool_deployed, pool_token_a_token_b);
    assert_eq!(pool_deployed, pool_token_b_token_a);

    // Verify PoolCreated event emitted
    let event = context.get_event(0).unwrap();
    let pool_deployed = *pool_deployed.first().unwrap();

    assert_eq!(event.data[0], TOKEN_A().into());
    assert_eq!(event.data[1], TOKEN_B().into());
    assert_eq!(event.data[2], Felt::from(fee_amount.fee_amount()));
    assert_eq!(event.data[3], Felt::from(fee_amount.tick_spacing()));
    assert_eq!(event.data[4], Felt::from(0u8));
    assert_eq!(event.data[5], pool_deployed);
}

#[test_case(
    TOKEN_A(),
    TOKEN_A(),
    FeeAmount::Low,
    "tokens must be different";
    "fails_if_tokens_are_the_same"
)]
#[test_case(
    ZERO(),
    TOKEN_B(),
    FeeAmount::Low,
    "tokens addresses cannot be zero";
    "token_a_zero"
)]
#[test_case(
    TOKEN_A(),
    ZERO(),
    FeeAmount::Low,
    "tokens addresses cannot be zero";
    "token_b_zero"
)]
#[test_case(
    TOKEN_A(),
    TOKEN_B(),
    FeeAmount::Other(1, 1),
    "tick spacing not initialized";
    "fails_if_fee_amount_is_not_enabled"
)]
fn test_create_pool_arguments_edge_cases(
    token_a: Signers,
    token_b: Signers,
    fee_amount: FeeAmount,
    error_message: &'static str,
) {
    let (mut context, _) = setup(OWNER().into(), None);

    context.clean_events();
    context.set_caller(OTHER().into());

    let pool_deployed = context.call_entry_point(
        YASFactory::name(),
        "create_pool",
        vec![token_a.into(), token_b.into(), StarkFelt::from(fee_amount.fee_amount())],
    );

    assert_eq!(pool_deployed, vec![string_to_felt(&error_message).unwrap()]);
}

#[test]
fn test_create_pool_fails_if_token_pair_is_already_created() {
    let (mut context, _) = setup(OWNER().into(), None);

    context.clean_events();
    context.set_caller(OTHER().into());

    let _ = context.call_entry_point(
        YASFactory::name(),
        "create_pool",
        vec![TOKEN_A().into(), TOKEN_B().into(), StarkFelt::from(FeeAmount::Low.fee_amount())],
    );

    let pool_deployed = context.call_entry_point(
        YASFactory::name(),
        "create_pool",
        vec![TOKEN_A().into(), TOKEN_B().into(), StarkFelt::from(FeeAmount::Low.fee_amount())],
    );

    assert_eq!(pool_deployed, vec![string_to_felt("token pair already created").unwrap()]);
}

#[test]
fn test_create_pool_fails_if_token_pair_is_already_created_invert_order() {
    let (mut context, _) = setup(OWNER().into(), None);

    context.clean_events();
    context.set_caller(OTHER().into());

    let _ = context.call_entry_point(
        YASFactory::name(),
        "create_pool",
        vec![TOKEN_A().into(), TOKEN_B().into(), StarkFelt::from(FeeAmount::Low.fee_amount())],
    );

    let pool_deployed = context.call_entry_point(
        YASFactory::name(),
        "create_pool",
        vec![TOKEN_B().into(), TOKEN_A().into(), StarkFelt::from(FeeAmount::Low.fee_amount())],
    );

    assert_eq!(pool_deployed, vec![string_to_felt("token pair already created").unwrap()]);
}

#[cfg(test)]
mod set_owner_tests {
    use super::*;

    #[test]
    fn test_fails_if_caller_is_not_owner() {
        let (mut context, _) = setup(OWNER().into(), None);

        context.clean_events();
        context.set_caller(OTHER().into());

        let result =
            context.call_entry_point(YASFactory::name(), "set_owner", vec![OTHER().into()]);

        assert_eq!(result, vec![string_to_felt("only owner can do this action!").unwrap()]);
    }

    #[test]
    fn test_success_when_caller_is_owner_and_emits_events() {
        let (mut context, _) = setup(OWNER().into(), None);

        context.clean_events();
        context.set_caller(OWNER().into());

        let result =
            context.call_entry_point(YASFactory::name(), "set_owner", vec![OTHER().into()]);

        assert_eq!(result, vec![]);

        assert_eq!(
            context.call_entry_point(YASFactory::name(), "owner", vec![]),
            vec![OTHER().into()]
        );

        let event = context.get_event(0).unwrap();

        assert_eq!(event.data[0], OWNER().into());
        assert_eq!(event.data[1], OTHER().into());
    }
}

#[cfg(test)]
mod set_enable_fee_amount_tests {
    use super::*;

    #[test]
    fn test_fails_if_caller_is_not_owner() {
        let (mut context, _) = setup(OWNER().into(), None);

        context.clean_events();
        context.set_caller(OTHER().into());

        let result = context.call_entry_point(
            YASFactory::name(),
            "enable_fee_amount",
            vec![StarkFelt::from(100u32), StarkFelt::from(2u32), StarkFelt::from(0u32)],
        );

        assert_eq!(result, vec![string_to_felt("only owner can do this action!").unwrap()]);
    }

    #[test]
    fn test_fails_if_fee_is_too_large() {
        let (mut context, _) = setup(OWNER().into(), None);

        context.clean_events();
        context.set_caller(OWNER().into());

        let result = context.call_entry_point(
            YASFactory::name(),
            "enable_fee_amount",
            vec![StarkFelt::from(1000000u128), StarkFelt::from(20u32), StarkFelt::from(0u32)],
        );

        assert_eq!(result, vec![string_to_felt("fee cannot be gt 1000000").unwrap()]);
    }

    #[test]
    fn test_fails_if_tick_spacing_is_too_small() {
        let (mut context, _) = setup(OWNER().into(), None);

        context.clean_events();
        context.set_caller(OWNER().into());

        let result = context.call_entry_point(
            YASFactory::name(),
            "enable_fee_amount",
            vec![StarkFelt::from(500u32), StarkFelt::from(0u32), StarkFelt::from(0u32)],
        );

        assert_eq!(result, vec![string_to_felt("wrong tick_spacing (0<ts<16384)").unwrap()]);
    }

    #[test]
    fn test_fails_if_already_initialized() {
        let (mut context, _) = setup(OWNER().into(), None);

        context.clean_events();
        context.set_caller(OWNER().into());

        let _ = context.call_entry_point(
            YASFactory::name(),
            "enable_fee_amount",
            vec![StarkFelt::from(50u32), StarkFelt::from(1u32), StarkFelt::from(0u32)],
        );

        let result = context.call_entry_point(
            YASFactory::name(),
            "enable_fee_amount",
            vec![StarkFelt::from(50u32), StarkFelt::from(10u32), StarkFelt::from(0u32)],
        );

        assert_eq!(result, vec![string_to_felt("fee amount already initialized").unwrap()]);
    }

    #[test]
    fn test_set_fee_amount_in_the_mapping() {
        let (mut context, _) = setup(OWNER().into(), None);

        context.clean_events();
        context.set_caller(OWNER().into());

        let _ = context.call_entry_point(
            YASFactory::name(),
            "enable_fee_amount",
            vec![StarkFelt::from(50u32), StarkFelt::from(1u32), StarkFelt::from(0u32)],
        );

        let result = context.call_entry_point(
            YASFactory::name(),
            "fee_amount_tick_spacing",
            vec![StarkFelt::from(50u32)],
        );

        assert_eq!(result, vec![Felt::from(1u32), Felt::from(0u32)]);
    }

    #[test]
    fn test_emits_event() {
        let (mut context, _) = setup(OWNER().into(), None);

        context.clean_events();
        context.set_caller(OWNER().into());

        let result = context.call_entry_point(
            YASFactory::name(),
            "enable_fee_amount",
            vec![StarkFelt::from(50u32), StarkFelt::from(1u32), StarkFelt::from(0u32)],
        );

        assert_eq!(result, vec![]);

        let event = context.get_event(0).unwrap();

        assert_eq!(event.data[0], Felt::from(50u32), "fee event should be 50");
        assert_eq!(event.data[1], Felt::from(1u32), "tick_spacing event should be 1");
        assert_eq!(event.data[2], Felt::from(0u32), "tick_spacing event should be 1");
    }

    #[test]
    fn test_enables_pool_creation() {
        let (mut context, _) = setup(OWNER().into(), None);

        context.clean_events();
        context.set_caller(OWNER().into());

        let _ = context.call_entry_point(
            YASFactory::name(),
            "enable_fee_amount",
            vec![StarkFelt::from(250u32), StarkFelt::from(15u32), StarkFelt::from(0u32)],
        );

        let pool_deployed = context.call_entry_point(
            YASFactory::name(),
            "create_pool",
            vec![TOKEN_A().into(), TOKEN_B().into(), StarkFelt::from(250u32)],
        );

        let pool_token_a_token_b = context.call_entry_point(
            YASFactory::name(),
            "pool",
            vec![TOKEN_A().into(), TOKEN_B().into(), StarkFelt::from(250u32)],
        );

        let pool_token_b_token_a = context.call_entry_point(
            YASFactory::name(),
            "pool",
            vec![TOKEN_B().into(), TOKEN_A().into(), StarkFelt::from(250u32)],
        );

        assert_eq!(pool_deployed, pool_token_a_token_b, "wrong pool in order result");
        assert_eq!(pool_deployed, pool_token_b_token_a, "wrong pool in reverse result");

        // Verify PoolCreated event emitted
        let event = context.get_event(1).unwrap();
        let pool_deployed = *pool_deployed.first().unwrap();

        assert_eq!(event.data[0], TOKEN_A().into(), "event token_0 should be TOKEN_A");
        assert_eq!(event.data[1], TOKEN_B().into(), "event token_1 should be TOKEN_B");
        assert_eq!(event.data[2], Felt::from(250u32), "event fee should be 250");
        assert_eq!(event.data[3], Felt::from(15u32), "tick_spacing.lo event should be 15");
        assert_eq!(event.data[4], Felt::from(0u32), "tick_spacing.hi event should be 0");
        assert_eq!(event.data[5], pool_deployed, "wrong event pool address");
    }
}
