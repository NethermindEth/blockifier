use blockifier::execution::contract_class::SierraContractClassV1;
use blockifier::execution::sierra_utils::{contract_address_to_starkfelt, felt_to_starkfelt};
use blockifier::s_calldata_starkfelt;
use blockifier::test_utils::testing_context::{
    FeeAmount, FixedType, Signers, StateFactory, TestContext, YASERC20Factory, YASFactory,
    YASPoolFactory, YASRouterFactory, YasU256, FACTORY_NAME, OWNER, WALLET,
};
use blockifier::test_utils::{TEST_YAS_POOL_CONTRACT_CLASS_HASH, YAS_POOL_CONTRACT_PATH};
use cairo_serde::get_hi_lo_from_u256;
use num_traits::ToPrimitive;
use primitive_types::U256;
use starknet_api::class_hash;
use starknet_api::core::{ClassHash, ContractAddress, PatriciaKey};
use starknet_api::hash::{StarkFelt, StarkHash};
use starknet_types_core::felt::Felt;

const MINT_AMOUNT: u128 = 1000000000000000000000000u128;
// const MAX_SQRT_RATIO_HI = 4295128739u128;

// 1461446703485210103287273052203988822378723970342
// const MAX_SQRT_RATIO_LO = 1461446703485210103287273052203988822378723970342u128;
// 4295128739
pub const MIN_TICK: i32 = -887272;
pub const MAX_TICK: i32 = 887272;
pub const MIN_SQRT_RATIO: u128 = 4295128739;

// /// The maximum value that can be returned from `get_sqrt_ratio_at_tick`. Equivalent to
// get_sqrt_ratio_at_tick(MAX_TICK). const MAX_SQRT_RATIO: u256 =
// 1461446703485210103287273052203988822378723970342;
pub fn get_yas_u256_from_str(num: &str) -> YasU256 {
    let num_parsed = U256::from_dec_str(num).unwrap();

    let (hi, lo) = get_hi_lo_from_u256(num_parsed);

    YasU256 { lo, hi }
}

pub fn get_u256_from_str(num: &str) -> U256 {
    U256::from_dec_str(num).unwrap()
}

pub fn min_tick(tick_spacing: i32) -> i32 {
    (MIN_TICK / tick_spacing) * tick_spacing
}

pub fn max_tick(tick_spacing: i32) -> i32 {
    (MAX_TICK / tick_spacing) * tick_spacing
}

fn encode_price_sqrt_1_2() -> FixedType {
    FixedType::from_u128(56022770974786139918731938227u128)
}

pub fn max_sqrt_ratio() -> U256 {
    get_u256_from_str("1461446703485210103287273052203988822378723970342")
}

fn token_0_factory() -> YASERC20Factory<'static> {
    YASERC20Factory::new(Some("YAS0"), Some("$YAS0"), Some(Felt::from(MINT_AMOUNT)), Some(OWNER()))
}

fn token_1_factory() -> YASERC20Factory<'static> {
    YASERC20Factory::new(Some("YAS1"), Some("$YAS1"), Some(Felt::from(MINT_AMOUNT)), Some(OWNER()))
}

fn token_0_name() -> String {
    token_0_factory().name()
}

fn token_1_name() -> String {
    token_1_factory().name()
}

fn yas_router() -> String {
    YASRouterFactory::new().name()
}

fn yas_pool() -> String {
    YAS_POOL_FACTORY_NAME()
}

fn setup() -> TestContext {
    // <Router>
    let mut context = TestContext::new(YASRouterFactory::new());

    // <Factory>
    let pool_class_hash = class_hash!(TEST_YAS_POOL_CONTRACT_CLASS_HASH);
    context.patch_with_factory(YASFactory::new(OWNER().into(), pool_class_hash));
    context.add_manual_class_hash(
        pool_class_hash,
        SierraContractClassV1::from_file(YAS_POOL_CONTRACT_PATH).into(),
    );

    // <Token 0>
    context.patch_with_factory(token_0_factory());

    // <Token 1>
    context.patch_with_factory(token_1_factory());

    // Initialization
    context.set_caller(OWNER().into());

    let result = context.call_entry_point(
        &token_0_name(),
        "transfer",
        vec![WALLET().into(), StarkFelt::from(MINT_AMOUNT), StarkFelt::from(0u128)],
    );

    assert_eq!(result, vec![Felt::from(true)]);

    let result = context.call_entry_point(
        &token_1_name(),
        "transfer",
        vec![
            WALLET().into(),
            StarkFelt::from(1000000000000000000000000u128),
            StarkFelt::from(0u128),
        ],
    );

    assert_eq!(result, vec![Felt::from(true)]);

    // Give permission to expend WALLET() tokens
    context.set_caller(WALLET().into());

    let yas_router_address = context.contract_address(&yas_router());

    let result = context.call_entry_point(
        &token_0_name(),
        "approve",
        vec![
            contract_address_to_starkfelt(yas_router_address),
            StarkFelt::from(MINT_AMOUNT),
            StarkFelt::from(0u128),
        ],
    );

    assert_eq!(result, vec![Felt::from(true)]);

    let result = context.call_entry_point(
        &token_1_name(),
        "approve",
        vec![
            contract_address_to_starkfelt(yas_router_address),
            StarkFelt::from(MINT_AMOUNT),
            StarkFelt::from(0u128),
        ],
    );

    assert_eq!(result, vec![Felt::from(true)]);

    context
}

fn init_pool(context: &mut TestContext) {
    // Create pool
    let yas_pool_address = context.call_entry_point(
        &FACTORY_NAME(),
        "create_pool",
        vec![
            contract_address_to_starkfelt(context.contract_address(&token_0_name())),
            contract_address_to_starkfelt(context.contract_address(&token_1_name())),
            StarkFelt::from(FeeAmount::Medium.fee_amount()),
        ],
    );

    let yas_pool_address =
        ContractAddress(PatriciaKey::try_from(felt_to_starkfelt(yas_pool_address[0])).unwrap());

    // Initialize yas_pool
    context.set_caller(OWNER().into());
    context.register_contract(yas_pool(), yas_pool_address.clone());

    let result = context.call_entry_point(
        &yas_pool(),
        "initialize",
        s_calldata_starkfelt!(encode_price_sqrt_1_10()),
    );

    assert_eq!(result, vec![]);

    let (min_tick, max_tick) = get_min_tick_and_max_tick();

    context.set_caller(WALLET().into());

    let result = context.call_entry_point(
        &yas_router(),
        "mint",
        vec![
            contract_address_to_starkfelt(yas_pool_address),
            WALLET().into(),
            StarkFelt::from(min_tick.abs() as u32),
            felt_to_starkfelt(Felt::from(true)),
            StarkFelt::from(max_tick.abs() as u32),
            felt_to_starkfelt(Felt::from(false)),
            StarkFelt::from(3161u128),
        ],
    );

    assert_eq!(result, vec![]);
}

fn deploy_only_pool() -> TestContext {
    let mut context = TestContext::new_empty();

    context.add_manual_class_hash(
        class_hash!(TEST_YAS_POOL_CONTRACT_CLASS_HASH),
        SierraContractClassV1::from_file(YAS_POOL_CONTRACT_PATH).into(),
    );

    context.add_manual_contract(
        yas_pool(),
        ContractAddress::from(10230129302481021u128),
        class_hash!(TEST_YAS_POOL_CONTRACT_CLASS_HASH),
    );

    context
}

fn MOCK_FACTORY_ADDRESS() -> Signers {
    Signers::Custom(ContractAddress::from(10230129302481021u128))
}

fn MOCK_TOKEN_1_ADDRESS() -> Signers {
    Signers::Custom(ContractAddress::from(10230129302481022u128))
}

fn MOCK_TOKEN_2_ADDRESS() -> Signers {
    Signers::Custom(ContractAddress::from(10230129302481023u128))
}

fn YAS_POOL_FACTORY_NAME() -> String {
    YASPoolFactory::new(vec![
        MOCK_FACTORY_ADDRESS().into(),
        MOCK_TOKEN_1_ADDRESS().into(),
        MOCK_TOKEN_2_ADDRESS().into(),
        Felt::from(FeeAmount::Medium.fee_amount()),
        Felt::from(FeeAmount::Medium.tick_spacing()),
        Felt::from(0u8),
    ])
    .name()
}

fn encode_price_sqrt_1_10() -> FixedType {
    // // returns result of encode_price_sqrt(1, 10) on v3-core typescript impl.
    // fn encode_price_sqrt_1_10() -> FixedType {
    //     FP64x96Impl::new(25054144837504793118641380156, false)
    // }

    FixedType::from_u128(25054144837504793118641380156u128)
}

fn get_min_tick_and_max_tick() -> (i32, i32) {
    // fn get_min_tick_and_max_tick() -> (i32, i32) {
    //     let tick_spacing = IntegerTrait::<i32>::new(tick_spacing(FeeAmount::MEDIUM), false);
    //     let min_tick = i32_div_no_round(MIN_TICK(), tick_spacing) * tick_spacing;
    //     let max_tick = i32_div_no_round(MAX_TICK(), tick_spacing) * tick_spacing;
    //     (min_tick, max_tick)
    // }

    let tick_spacing = FeeAmount::Medium.tick_spacing() as i32;

    let min_tick = (MIN_TICK / tick_spacing) * tick_spacing;
    let max_tick = (MAX_TICK / tick_spacing) * tick_spacing;

    (min_tick, max_tick)
}

#[cfg(test)]
mod constructor_tests {
    use super::*;

    #[test]
    fn deploys() {
        // let _ = setup_with_pool();
    }
}
#[cfg(test)]
mod initialize_tests {
    use blockifier::test_utils::testing_context::{string_to_felt, FixedType, YasI32};
    use blockifier::{s_calldata, s_calldata_felt, s_calldata_starkfelt};
    use cairo_serde::traits::CairoSerializable;

    use super::*;

    #[test]
    fn test_fails_if_already_initialized() {
        let mut context = deploy_only_pool();

        let result = context.call_entry_point(
            &yas_pool(),
            "initialize",
            s_calldata_starkfelt!(encode_price_sqrt_1_10()),
        );

        assert_eq!(result, vec![]);

        let result = context.call_entry_point(
            &yas_pool(),
            "initialize",
            s_calldata_starkfelt!(encode_price_sqrt_1_10()),
        );

        assert_eq!(result, vec![string_to_felt("AI").unwrap()]);
    }

    #[test]
    fn test_fails_if_price_is_too_low() {
        let mut context = deploy_only_pool();

        let result = context.call_entry_point(
            &yas_pool(),
            "initialize",
            s_calldata_starkfelt!(FixedType::from_u128(1)),
        );

        assert_eq!(result, vec![string_to_felt("R").unwrap()]);
    }

    #[test]
    fn test_fails_if_price_is_min_sqrt_ratio_minus_1() {
        let mut context = deploy_only_pool();

        let result = context.call_entry_point(
            &yas_pool(),
            "initialize",
            s_calldata_starkfelt!(FixedType::from_u128(MIN_SQRT_RATIO - 1)),
        );

        assert_eq!(result, vec![string_to_felt("R").unwrap()]);
    }

    #[test]
    fn test_fails_if_price_is_too_high() {
        let mut context = deploy_only_pool();

        let result = context.call_entry_point(
            &yas_pool(),
            "initialize",
            s_calldata_starkfelt!(FixedType::from_u256(U256::from(2).pow(U256::from(160)) - 1)),
        );

        assert_eq!(result, vec![string_to_felt("R").unwrap()]);
    }

    #[test]
    fn test_fails_if_price_is_max_sqrt_ratio() {
        let mut context = deploy_only_pool();

        let result = context.call_entry_point(
            &yas_pool(),
            "initialize",
            s_calldata_starkfelt!(FixedType::from_u256(max_sqrt_ratio())),
        );

        assert_eq!(result, vec![string_to_felt("R").unwrap()]);
    }

    #[test]
    fn test_can_be_initialized_at_min_sqrt_ratio() {
        let mut context = deploy_only_pool();

        let sqrt_price_x96 = FixedType::from_u128(MIN_SQRT_RATIO);

        let result = context.call_entry_point(
            &yas_pool(),
            "initialize",
            s_calldata_starkfelt!(sqrt_price_x96),
        );

        assert_eq!(result, vec![]);

        let result = context.call_entry_point(&yas_pool(), "get_slot_0", vec![]);

        assert_eq!(
            result,
            s_calldata_felt!(
                FixedType::from_u128(MIN_SQRT_RATIO),
                YasI32::from_i32(min_tick(1)),
                0u8
            )
        );
    }

    #[test]
    fn test_can_be_initialized_at_max_sqrt_ratio_minus_1() {
        let mut context = deploy_only_pool();

        let sqrt_price_x96 = FixedType::from_u256(max_sqrt_ratio() - 1);

        let result = context.call_entry_point(
            &yas_pool(),
            "initialize",
            s_calldata_starkfelt!(sqrt_price_x96),
        );

        assert_eq!(result, vec![]);

        let result = context.call_entry_point(&yas_pool(), "get_slot_0", vec![]);

        assert_eq!(
            result,
            s_calldata_felt!(sqrt_price_x96, YasI32::from_i32(max_tick(1) - 1), 0u128)
        );
    }

    #[test]
    fn test_sets_initial_variables() {
        let mut context = deploy_only_pool();

        let result = context.call_entry_point(
            &yas_pool(),
            "initialize",
            s_calldata_starkfelt!(encode_price_sqrt_1_2()),
        );

        assert_eq!(result, vec![]);

        let result = context.call_entry_point(&yas_pool(), "get_slot_0", vec![]);

        assert_eq!(
            result,
            s_calldata_felt!(encode_price_sqrt_1_2(), YasI32::from_i32(-6932), 0u128)
        );
    }

    #[test]
    fn test_emits_an_initialized_event() {
        let mut context = deploy_only_pool();

        let sqrt_price_x96 = encode_price_sqrt_1_2();
        let tick = YasI32::from_i32(-6932);

        let result = context.call_entry_point(
            &yas_pool(),
            "initialize",
            s_calldata_starkfelt!(sqrt_price_x96),
        );

        assert_eq!(result, vec![]);

        let event = context.get_event(0).unwrap();

        assert_eq!(event.data, s_calldata_felt!(sqrt_price_x96, tick));
    }
}
