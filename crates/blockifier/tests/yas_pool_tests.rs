use blockifier::execution::contract_class::SierraContractClassV1;
use blockifier::execution::sierra_utils::{contract_address_to_starkfelt, felt_to_starkfelt};
use blockifier::test_utils::testing_context::{
    FeeAmount, FixedType, Signers, StateFactory, TestContext, YASERC20Factory, YASFactory,
    YASPoolFactory, YASRouterFactory, YasI256, YasI32, YasU256, FACTORY_NAME, OWNER, WALLET,
};
use blockifier::test_utils::{TEST_YAS_POOL_CONTRACT_CLASS_HASH, YAS_POOL_CONTRACT_PATH};
use blockifier::{s_calldata_felt, s_calldata_starkfelt};
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

fn encode_price_sqrt_1_1() -> FixedType {
    FixedType::from_u128(79228162514264337593543950336u128)
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

fn setup_with(
    initial_price: FixedType,
    usdc_amount: u128,
    eth_amount: u128,
    mint_amount: YasU256,
) -> TestContext {
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
    let usdc = 1000000 * usdc_amount;

    context.patch_with_factory(YASERC20Factory::new(
        Some("YAS0"),
        Some("USDC"),
        Some(Felt::from(usdc)),
        Some(OWNER()),
    ));

    // <Token 1>
    let eth = 1000000 * eth_amount;

    context.patch_with_factory(YASERC20Factory::new(
        Some("YAS1"),
        Some("ETH"),
        Some(Felt::from(eth)),
        Some(OWNER()),
    ));

    context.set_caller(OWNER().into());
    let yas_router_address = context.contract_address(&yas_router());

    let result = context.call_entry_point(
        &token_0_name(),
        "transfer",
        s_calldata_starkfelt!(WALLET(), usdc),
    );
    assert_eq!(result, vec![Felt::from(true)]);

    let result =
        context.call_entry_point(&token_1_name(), "transfer", s_calldata_starkfelt!(WALLET(), eth));
    assert_eq!(result, vec![Felt::from(true)]);

    // Give permission to expend WALLET() tokens
    context.set_caller(WALLET().into());

    let result = context.call_entry_point(
        &token_0_name(),
        "approve",
        s_calldata_starkfelt!(yas_router_address, usdc),
    );
    assert_eq!(result, vec![Felt::from(true)]);

    let result = context.call_entry_point(
        &token_1_name(),
        "approve",
        s_calldata_starkfelt!(yas_router_address, eth),
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

fn setup_pool_for_swap_test(
    initial_price: FixedType,
    fee_amount: FeeAmount,
    mint_positions: Vec<MintPosition>,
) -> TestContext {
    let mut context = TestContext::new(YASRouterFactory::new());

    context.patch_with_factory(YASFactory::new(
        OWNER().into(),
        class_hash!(TEST_YAS_POOL_CONTRACT_CLASS_HASH),
    ));

    context.add_manual_class_hash(
        class_hash!(TEST_YAS_POOL_CONTRACT_CLASS_HASH),
        SierraContractClassV1::from_file(YAS_POOL_CONTRACT_PATH).into(),
    );

    context.patch_with_factory(token_0_factory());
    context.patch_with_factory(token_1_factory());

    context.set_caller(OWNER().into());

    let yas_router_address = context.contract_address(&yas_router());

    let result = context.call_entry_point(
        &token_0_name(),
        "transfer",
        s_calldata_starkfelt!(WALLET(), MINT_AMOUNT),
    );

    assert_eq!(result, vec![Felt::from(true)]);

    let result = context.call_entry_point(
        &token_1_name(),
        "transfer",
        s_calldata_starkfelt!(WALLET(), MINT_AMOUNT),
    );

    assert_eq!(result, vec![Felt::from(true)]);

    // Give permission to expend WALLET() tokens
    context.set_caller(WALLET().into());

    let result = context.call_entry_point(
        &token_0_name(),
        "approve",
        s_calldata_starkfelt!(yas_router_address, MINT_AMOUNT),
    );

    assert_eq!(result, vec![Felt::from(true)]);

    let result = context.call_entry_point(
        &token_1_name(),
        "approve",
        s_calldata_starkfelt!(yas_router_address, MINT_AMOUNT),
    );

    assert_eq!(result, vec![Felt::from(true)]);

    let yas_pool_address = context.call_entry_point(
        &FACTORY_NAME(),
        "create_pool",
        s_calldata_starkfelt!(yas_router_address, MINT_AMOUNT),
    );

    context.set_caller(OWNER().into());

    let result =
        context.call_entry_point(&yas_pool(), "initialize", s_calldata_starkfelt!(initial_price));

    assert_eq!(result, vec![]);

    context.set_caller(WALLET().into());

    context
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

fn mint_positions(
    mut context: &mut TestContext,
    yas_pool_address: ContractAddress,
    mint_positions: Vec<MintPosition>,
) {
    for i in 0..mint_positions.len() {
        let mint_position = mint_positions[i];

        let result = context.call_entry_point(
            &yas_router(),
            "mint",
            s_calldata_starkfelt!(
                yas_pool_address,
                mint_position.tick_lower,
                mint_position.tick_upper,
                mint_position.amount
            ),
        );

        assert_eq!(result, vec![]);
    }
}

fn deploy(
    factory: Signers,
    token_0: Signers,
    token_1: Signers,
    fee: u32,
    tick_spacing: YasI32,
) -> TestContext {
    let context = TestContext::new(YASPoolFactory::new(s_calldata_felt!(
        factory.get_address(),
        token_0.get_address(),
        token_1.get_address(),
        fee,
        tick_spacing
    )));

    return context;
}

#[cfg(test)]
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

// TODO: tests are not implemented because of mock_contract_states() function
#[cfg(test)]
pub mod update_position_tests {
    #[test]
    fn test_add_liquidity_when_call_update_position_then_position_is_updated() {
        todo!("test_add_liquidity_when_call_update_position_then_position_is_updated")
    }

    #[test]
    fn test_sub_liquidity_when_call_update_position_then_position_is_updated() {
        todo!("test_sub_liquidity_when_call_update_position_then_position_is_updated")
    }

    #[test]
    fn test_sub_liquidity_gt_available_when_call_update_position_should_panic() {
        todo!("test_sub_liquidity_gt_available_when_call_update_position_should_panic")
    }

    #[test]
    fn test_add_liquidity_gt_max_liq_when_call_update_position_should_panic() {
        todo!("test_add_liquidity_gt_max_liq_when_call_update_position_should_panic")
    }
}

#[cfg(test)]
pub mod mint_tests {
    use blockifier::test_utils::testing_context::{TOKEN_A, TOKEN_B};

    use super::*;

    #[test]
    fn test_fails_not_initialized() {
        let mut context = deploy(
            MOCK_FACTORY_ADDRESS(),
            TOKEN_A(),
            TOKEN_B(),
            FeeAmount::Medium.fee_amount(),
            YasI32::from_i32(FeeAmount::Medium.tick_spacing() as i32),
        );

        let sqrt_price_x96 = encode_price_sqrt_1_1();

        let (min_tick, max_tick) = get_min_tick_and_max_tick();

        let yas_pool_address = context.contract_address(&yas_pool());

        let result = context.call_entry_point(
            &yas_pool(),
            "mint",
            s_calldata_starkfelt!(
                yas_pool_address,
                YasI32::from_i32(min_tick),
                YasI32::from_i32(max_tick),
                1u128,
                Vec::<Felt>::new()
            ),
        );

        println!("{:?}", result.first().unwrap().to_hex_string());

        assert_eq!(result, s_calldata_felt!("LOK"));
    }

    mod failure_cases {
        use super::*;

        #[test]
        fn test_fails_tick_lower_greater_than_tick_upper() {
            let mut context = setup();

            let yas_pool_address = context.contract_address(&yas_pool());

            let result = context.call_entry_point(
                &yas_router(),
                "mint",
                s_calldata_starkfelt!(
                    yas_pool_address,
                    WALLET(),
                    YasI32::from_i32(1),
                    YasI32::from_i32(0),
                    1u128
                ),
            );

            assert_eq!(result, s_calldata_felt!("TLU"));
        }

        #[test]
        fn test_fails_tick_lower_than_min() {
            let mut context = setup();

            init_pool(&mut context);

            let yas_pool_address = context.contract_address(&yas_pool());

            let result = context.call_entry_point(
                &yas_router(),
                "mint",
                s_calldata_starkfelt!(
                    yas_pool_address,
                    WALLET(),
                    YasI32::from_i32(MIN_TICK - 1),
                    YasI32::from_i32(0),
                    1u128
                ),
            );

            assert_eq!(result, s_calldata_felt!("TLM"));
        }

        #[test]
        fn test_fails_tick_greater_than_max() {
            let mut context = setup();

            init_pool(&mut context);

            let yas_pool_address = context.contract_address(&yas_pool());

            let result = context.call_entry_point(
                &yas_router(),
                "mint",
                s_calldata_starkfelt!(
                    yas_pool_address,
                    WALLET(),
                    YasI32::from_i32(0),
                    YasI32::from_i32(MAX_TICK + 1),
                    1u128
                ),
            );

            assert_eq!(result, s_calldata_felt!("TUM"));
        }

        #[test]
        fn test_fails_amount_exceeds_the_max() {
            let mut context = setup();

            init_pool(&mut context);

            let yas_pool_address = context.contract_address(&yas_pool());

            let max_liquidity_per_tick =
                context.call_entry_point(&yas_pool(), "get_max_liquidity_per_tick", vec![]);
            let max_liquidity_per_tick = max_liquidity_per_tick.first().unwrap();

            let tick_spacing = context.call_entry_point(&yas_pool(), "get_tick_spacing", vec![]);
            let tick_spacing = tick_spacing.first().unwrap();

            let (min_tick, max_tick) = get_min_tick_and_max_tick();

            let result = context.call_entry_point(
                &yas_router(),
                "mint",
                s_calldata_starkfelt!(
                    yas_pool_address,
                    WALLET(),
                    (Felt::from(min_tick.abs()) - tick_spacing),
                    Felt::from(true),
                    Felt::from(max_tick) - tick_spacing,
                    Felt::from(false),
                    max_liquidity_per_tick + Felt::from(1)
                ),
            );

            assert_eq!(result, s_calldata_felt!("LO"));
        }

        #[test]
        fn test_amount_max() {
            let mut context = setup();

            init_pool(&mut context);

            let yas_pool_address = context.contract_address(&yas_pool());

            let max_liquidity_gross =
                context.call_entry_point(&yas_pool(), "get_max_liquidity_per_tick", vec![]);
            let max_liquidity_gross = max_liquidity_gross.first().unwrap();

            let tick_spacing = context.call_entry_point(&yas_pool(), "get_tick_spacing", vec![]);
            let tick_spacing = tick_spacing.first().unwrap();

            let (min_tick, max_tick) = get_min_tick_and_max_tick();

            let result = context.call_entry_point(
                &yas_router(),
                "mint",
                s_calldata_starkfelt!(
                    yas_pool_address,
                    WALLET(),
                    Felt::from(min_tick.abs()) - tick_spacing,
                    true,
                    Felt::from(max_tick) - tick_spacing,
                    false,
                    max_liquidity_gross
                ),
            );

            println!("{:?}", result.first().unwrap().to_hex_string());

            assert_eq!(result, vec![]);
        }

        #[test_case::test_case(1, 1, 1; "m_1_1_1")]
        #[test_case::test_case(2, 1, 1; "m_2_1_1")]
        #[test_case::test_case(1, 2, 1; "m_1_2_1")]
        #[test_case::test_case(1, 1, 0; "m_1_2_0")]
        fn test_fails_amount_at_tick_greater_than_max(
            multiplier_a: u128,
            multiplier_b: u128,
            multiplier_c: u128,
        ) {
            let mut context = setup();

            init_pool(&mut context);

            let yas_pool_address = context.contract_address(&yas_pool());

            let tick_spacing = context.call_entry_point(&yas_pool(), "get_tick_spacing", vec![]);
            let tick_spacing = tick_spacing.first().unwrap();

            let (min_tick, max_tick) = get_min_tick_and_max_tick();

            let result = context.call_entry_point(
                &yas_router(),
                "mint",
                s_calldata_starkfelt!(
                    yas_pool_address,
                    WALLET(),
                    Felt::from(min_tick.abs()) - tick_spacing,
                    Felt::from(true),
                    Felt::from(max_tick) - tick_spacing,
                    Felt::from(false),
                    Felt::from(1000)
                ),
            );

            let max_liquidity_gross =
                context.call_entry_point(&yas_pool(), "get_max_liquidity_per_tick", vec![]);
            let max_liquidity_gross = max_liquidity_gross.first().unwrap();

            let result = context.call_entry_point(
                &yas_router(),
                "mint",
                s_calldata_starkfelt!(
                    yas_pool_address,
                    WALLET(),
                    Felt::from(min_tick.abs()) - tick_spacing * Felt::from(multiplier_a),
                    Felt::from(true),
                    Felt::from(max_tick) - tick_spacing * Felt::from(multiplier_b),
                    Felt::from(false),
                    max_liquidity_gross - Felt::from(1000u128) + Felt::from(1 * multiplier_c)
                ),
            );

            assert_eq!(result, s_calldata_felt!("LO"));
        }

        #[test]
        fn test_fails_amount_is_zero() {
            let mut context = setup();

            init_pool(&mut context);

            let yas_pool_address = context.contract_address(&yas_pool());
            let tick_spacing = context.call_entry_point(&yas_pool(), "get_tick_spacing", vec![]);
            let tick_spacing = tick_spacing.first().unwrap();
            let (min_tick, max_tick) = get_min_tick_and_max_tick();

            let result = context.call_entry_point(
                &yas_router(),
                "mint",
                s_calldata_starkfelt!(
                    yas_pool_address,
                    WALLET(),
                    Felt::from(min_tick.abs()) - tick_spacing,
                    Felt::from(true),
                    Felt::from(max_tick) - tick_spacing,
                    Felt::from(false),
                    Felt::from(0u128)
                ),
            );

            assert_eq!(result, s_calldata_felt!("amount must be greater than 0"));
        }
    }

    mod success_cases {
        use super::*;

        #[test]
        fn test_initial_balances() {
            let mut context = setup();

            init_pool(&mut context);

            let pool_address = context.contract_address(&yas_pool());
            let balance_token_0 = context.call_entry_point(
                &token_0_name(),
                "balanceOf",
                s_calldata_starkfelt!(pool_address),
            );
            let balance_token_1 = context.call_entry_point(
                &token_1_name(),
                "balanceOf",
                s_calldata_starkfelt!(pool_address),
            );

            println!("{:?}", balance_token_0.first().unwrap().to_hex_string());
            println!("{:?}", balance_token_1.first().unwrap().to_hex_string());

            assert_eq!(balance_token_0, s_calldata_felt!(YasU256::from_u128(996)));
            assert_eq!(balance_token_1, s_calldata_felt!(YasU256::from_u128(1000)));
        }

        #[test]
        fn test_initial_tick() {
            let mut context = setup();

            init_pool(&mut context);

            let slot_0 = context.call_entry_point(&yas_pool(), "get_slot_0", vec![]);

            let tick = slot_0[3];
            let tick_sign = slot_0[4];

            assert_eq!(vec![tick, tick_sign], s_calldata_felt!(YasI32::from_i32(-23028)));
        }

        mod above_current_price {
            use std::cmp::max;

            use super::*;

            #[test]
            fn test_transfers_token_0_only() {
                let mut context = setup();

                init_pool(&mut context);

                let pool_address = context.contract_address(&yas_pool());

                let result = context.call_entry_point(
                    &yas_router(),
                    "mint",
                    s_calldata_starkfelt!(
                        pool_address,
                        WALLET(),
                        YasI32::from_i32(-22980),
                        YasI32::from_i32(0),
                        10000u128
                    ),
                );

                let balance_token_0 = context.call_entry_point(
                    &token_0_name(),
                    "balanceOf",
                    s_calldata_starkfelt!(pool_address),
                );

                let balance_token_1 = context.call_entry_point(
                    &token_1_name(),
                    "balanceOf",
                    s_calldata_starkfelt!(pool_address),
                );

                assert_eq!(result, vec![]);

                assert_eq!(balance_token_0, s_calldata_felt!(YasU256::from_u128(9996 + 21549)));

                assert_eq!(balance_token_1, s_calldata_felt!(YasU256::from_u128(1000)));
            }

            #[test]
            fn test_max_tick_max_lvrg() {
                let mut context = setup();

                init_pool(&mut context);

                let pool_address = context.contract_address(&yas_pool());

                let big_number = 2u128.pow(102);

                let (_, max_tick) = get_min_tick_and_max_tick();
                let max_tick = Felt::from(max_tick);

                let tick_spacing =
                    context.call_entry_point(&yas_pool(), "get_tick_spacing", vec![]);
                let tick_spacing = tick_spacing.first().unwrap();

                let (tick_spacing, tick_spacing_sign) = if max_tick < *tick_spacing {
                    (tick_spacing - max_tick, true)
                } else {
                    (max_tick - tick_spacing, false)
                };

                let result = context.call_entry_point(
                    &yas_router(),
                    "mint",
                    s_calldata_starkfelt!(
                        pool_address,
                        WALLET(),
                        tick_spacing,
                        tick_spacing_sign,
                        Felt::from(max_tick),
                        Felt::from(false),
                        Felt::from(big_number)
                    ),
                );

                let balance_token_0 = context.call_entry_point(
                    &token_0_name(),
                    "balanceOf",
                    s_calldata_starkfelt!(pool_address),
                );

                let balance_token_1 = context.call_entry_point(
                    &token_1_name(),
                    "balanceOf",
                    s_calldata_starkfelt!(pool_address),
                );

                assert_eq!(result, vec![]);

                assert_eq!(balance_token_0, s_calldata_felt!(YasU256::from_u128(9996 + 828011525)));
                assert_eq!(balance_token_1, s_calldata_felt!(YasU256::from_u128(1000)));
            }

            #[test]
            fn test_max_tick() {
                let mut context = setup();

                init_pool(&mut context);

                let pool_address = context.contract_address(&yas_pool());

                let (_, max_tick) = get_min_tick_and_max_tick();

                let result = context.call_entry_point(
                    &yas_router(),
                    "mint",
                    s_calldata_starkfelt!(
                        pool_address,
                        WALLET(),
                        YasI32::from_i32(-22980),
                        YasI32::from_i32(max_tick),
                        10000u128
                    ),
                );

                let balance_token_0 = context.call_entry_point(
                    &token_0_name(),
                    "balanceOf",
                    s_calldata_starkfelt!(pool_address),
                );

                let balance_token_1 = context.call_entry_point(
                    &token_1_name(),
                    "balanceOf",
                    s_calldata_starkfelt!(pool_address),
                );

                assert_eq!(result, vec![]);
                assert_eq!(balance_token_0, s_calldata_felt!(YasU256::from_u128(9996 + 31549)));
                assert_eq!(balance_token_1, s_calldata_felt!(YasU256::from_u128(1000)));
            }

            #[test]
            fn test_add_liquidity_gross() {
                let mut context = setup();

                init_pool(&mut context);

                let pool_address = context.contract_address(&yas_pool());

                let result = context.call_entry_point(
                    &yas_router(),
                    "mint",
                    s_calldata_starkfelt!(
                        pool_address,
                        WALLET(),
                        YasI32::from_i32(-240),
                        YasI32::from_i32(0),
                        100u128
                    ),
                );

                assert_eq!(result, vec![]);

                let get_tick = context.call_entry_point(
                    &yas_pool(),
                    "get_tick",
                    s_calldata_starkfelt!(YasI32::from_i32(-240)),
                );
                let get_tick = get_tick.first().unwrap();

                assert_eq!(vec![*get_tick], s_calldata_felt!(100));

                let get_tick = context.call_entry_point(
                    &yas_pool(),
                    "get_tick",
                    s_calldata_starkfelt!(YasI32::from_i32(0)),
                );
                let get_tick = get_tick.first().unwrap();

                assert_eq!(vec![*get_tick], s_calldata_felt!(100));

                let tick_spacing =
                    context.call_entry_point(&yas_pool(), "get_tick_spacing", vec![]);
                let tick_spacing = tick_spacing.first().unwrap();

                let get_tick = context.call_entry_point(
                    &yas_pool(),
                    "get_tick",
                    s_calldata_starkfelt!(tick_spacing),
                );
                let get_tick = get_tick.first().unwrap();

                assert_eq!(vec![*get_tick], s_calldata_felt!(0));

                let get_tick = context.call_entry_point(
                    &yas_pool(),
                    "get_tick",
                    s_calldata_starkfelt!(YasI32::from_u32(FeeAmount::Medium.tick_spacing() * 2)),
                );
                let get_tick = get_tick.first().unwrap();

                assert_eq!(vec![*get_tick], s_calldata_felt!(0));

                let result = context.call_entry_point(
                    &yas_router(),
                    "mint",
                    s_calldata_starkfelt!(
                        pool_address,
                        WALLET(),
                        YasI32::from_i32(-240),
                        tick_spacing,
                        *tick_spacing < Felt::from(0),
                        150u128
                    ),
                );

                assert_eq!(result, vec![]);

                let get_tick = context.call_entry_point(
                    &yas_pool(),
                    "get_tick",
                    s_calldata_starkfelt!(YasI32::from_i32(-240)),
                );
                let get_tick = get_tick.first().unwrap();

                assert_eq!(vec![*get_tick], s_calldata_felt!(250));

                let get_tick = context.call_entry_point(
                    &yas_pool(),
                    "get_tick",
                    s_calldata_starkfelt!(YasI32::from_i32(0)),
                );
                let get_tick = get_tick.first().unwrap();

                assert_eq!(vec![*get_tick], s_calldata_felt!(100));

                let get_tick = context.call_entry_point(
                    &yas_pool(),
                    "get_tick",
                    s_calldata_starkfelt!(tick_spacing),
                );
                let get_tick = get_tick.first().unwrap();

                assert_eq!(vec![*get_tick], s_calldata_felt!(150));

                let get_tick = context.call_entry_point(
                    &yas_pool(),
                    "get_tick",
                    s_calldata_starkfelt!(YasI32::from_u32(FeeAmount::Medium.tick_spacing() * 2)),
                );
                let get_tick = get_tick.first().unwrap();

                assert_eq!(vec![*get_tick], s_calldata_felt!(0));

                let result = context.call_entry_point(
                    &yas_router(),
                    "mint",
                    s_calldata_starkfelt!(
                        pool_address,
                        WALLET(),
                        YasI32::from_i32(0),
                        FeeAmount::Medium.tick_spacing() * 2,
                        false,
                        150u128
                    ),
                );

                assert_eq!(result, vec![]);

                let get_tick = context.call_entry_point(
                    &yas_pool(),
                    "get_tick",
                    s_calldata_starkfelt!(YasI32::from_i32(-240)),
                );
                let get_tick = get_tick.first().unwrap();

                assert_eq!(vec![*get_tick], s_calldata_felt!(250));

                let get_tick = context.call_entry_point(
                    &yas_pool(),
                    "get_tick",
                    s_calldata_starkfelt!(YasI32::from_i32(0)),
                );
                let get_tick = get_tick.first().unwrap();

                assert_eq!(vec![*get_tick], s_calldata_felt!(160));

                let tick_spacing =
                    context.call_entry_point(&yas_pool(), "get_tick_spacing", vec![]);
                let tick_spacing = tick_spacing.first().unwrap();

                let get_tick = context.call_entry_point(
                    &yas_pool(),
                    "get_tick",
                    s_calldata_starkfelt!(tick_spacing),
                );
                let get_tick = get_tick.first().unwrap();

                assert_eq!(vec![*get_tick], s_calldata_felt!(150));

                let get_tick = context.call_entry_point(
                    &yas_pool(),
                    "get_tick",
                    s_calldata_starkfelt!(YasI32::from_u32(FeeAmount::Medium.tick_spacing() * 2)),
                );
                let get_tick = get_tick.first().unwrap();

                assert_eq!(vec![*get_tick], s_calldata_felt!(60));
            }
        }

        mod below_current_price {
            use super::*;

            #[test]
            fn test_below_only_token_1() {
                let mut context = setup();

                init_pool(&mut context);

                let pool_address = context.contract_address(&yas_pool());

                let result = context.call_entry_point(
                    &yas_router(),
                    "mint",
                    s_calldata_starkfelt!(
                        pool_address,
                        WALLET(),
                        YasI32::from_i32(46080),
                        YasI32::from_i32(23040),
                        10000u128
                    ),
                );

                let balance_token_0 = context.call_entry_point(
                    &token_0_name(),
                    "balanceOf",
                    s_calldata_starkfelt!(pool_address),
                );

                let balance_token_1 = context.call_entry_point(
                    &token_1_name(),
                    "balanceOf",
                    s_calldata_starkfelt!(pool_address),
                );

                assert_eq!(result, vec![]);
                assert_eq!(balance_token_0, s_calldata_felt!(YasU256::from_u128(9996)));
                assert_eq!(balance_token_1, s_calldata_felt!(YasU256::from_u128(1000 + 2162)));
            }

            #[test]
            fn test_below_max_tick_max_lvrg() {
                let mut context = setup();

                init_pool(&mut context);

                let pool_address = context.contract_address(&yas_pool());
                let big_number = 2u128.pow(102);

                let (min_tick, _) = get_min_tick_and_max_tick();

                let result = context.call_entry_point(
                    &yas_router(),
                    "mint",
                    s_calldata_starkfelt!(
                        pool_address,
                        WALLET(),
                        YasI32::from_i32(min_tick),
                        YasI32::from_i32(min_tick + (FeeAmount::Medium.tick_spacing() as i32)),
                        big_number
                    ),
                );

                let balance_token_0 = context.call_entry_point(
                    &token_0_name(),
                    "balanceOf",
                    s_calldata_starkfelt!(pool_address),
                );

                let balance_token_1 = context.call_entry_point(
                    &token_1_name(),
                    "balanceOf",
                    s_calldata_starkfelt!(pool_address),
                );

                assert_eq!(result, vec![]);
                assert_eq!(balance_token_0, s_calldata_felt!(YasU256::from_u128(9996)));
                assert_eq!(balance_token_1, s_calldata_felt!(YasU256::from_u128(1000 + 828011520)));
            }

            #[test]
            fn test_below_min_tick() {
                let mut context = setup();

                init_pool(&mut context);

                let pool_address = context.contract_address(&yas_pool());
                let (min_tick, _) = get_min_tick_and_max_tick();

                let result = context.call_entry_point(
                    &yas_router(),
                    "mint",
                    s_calldata_starkfelt!(
                        pool_address,
                        WALLET(),
                        YasI32::from_i32(min_tick),
                        YasI32::from_i32(23040),
                        10000u128
                    ),
                );

                let balance_token_0 = context.call_entry_point(
                    &token_0_name(),
                    "balanceOf",
                    s_calldata_starkfelt!(pool_address),
                );

                let balance_token_1 = context.call_entry_point(
                    &token_1_name(),
                    "balanceOf",
                    s_calldata_starkfelt!(pool_address),
                );

                assert_eq!(result, vec![]);
                assert_eq!(balance_token_0, s_calldata_felt!(YasU256::from_u128(9996)));
                assert_eq!(balance_token_1, s_calldata_felt!(YasU256::from_u128(1000 + 3161)));
            }
        }
    }
}

#[cfg(test)]
pub mod swap_tests {
    use super::*;

    #[test]
    fn test_swap_token_1_for_token_0() {
        let initial_price = 45584610003121481572705762227159u128;

        let mut context = setup_with(
            FixedType::from_u128(initial_price),
            300000000000000u128,
            10000000000u128,
            YasU256::from_u128(100000000000000000000000u128),
        );

        init_pool(&mut context);

        let yas_pool_address = context.contract_address(&yas_pool());

        // 1 ETH
        let eth_amount = FixedType::from_u128(1000000000000000000u128);

        // 3019294,467836 USDC
        let usdc_swapped_expected = 3019294467836u128;

        // 1 ETH
        let eth_swapped_expected = 1000000000000000000u128;

        // will trade ETH for USDC (USDC token_0, ETH token_1) so, ZFO false
        let zero_for_one = false;

        // When selling token 0 (zeroForOne is true) sqrtPriceLimitX96 must be
        // between the current price and the minimal sqrt(P) since selling token 0
        // moves the price down. Likewise, when selling token 1, sqrtPriceLimitX96
        // must be between the current price and the maximal sqrt(P) because price moves up.

        // In the while loop, we want to satisfy two conditions: full swap amount has not
        // been filled and current price isn’t equal to sqrtPriceLimitX96:
        let price_limit = FixedType::from_u128(initial_price * 1000u128);

        // Check balance before swap
        let user_token_0_balance_bf =
            context.call_entry_point(&token_0_name(), "balanceOf", s_calldata_starkfelt!(WALLET()));
        let user_token_1_balance_bf =
            context.call_entry_point(&token_1_name(), "balanceOf", s_calldata_starkfelt!(WALLET()));

        // Execute swap
        let result = context.call_entry_point(
            &yas_router(),
            "swap",
            s_calldata_starkfelt!(
                yas_pool_address,
                WALLET(),
                zero_for_one,
                eth_amount,
                price_limit
            ),
        );

        // Check balance after swap
        let user_token_0_balance_af =
            context.call_entry_point(&token_0_name(), "balanceOf", s_calldata_starkfelt!(WALLET()));
        let user_token_1_balance_af =
            context.call_entry_point(&token_1_name(), "balanceOf", s_calldata_starkfelt!(WALLET()));

        assert_eq!(result, vec![]);
        assert_eq!(
            user_token_0_balance_af[0] - user_token_0_balance_bf[0],
            Felt::from(usdc_swapped_expected)
        );
        assert_eq!(
            user_token_1_balance_bf[0] - user_token_1_balance_af[0],
            Felt::from(eth_swapped_expected)
        );
    }

    #[test]
    fn test_swap_token_0_for_token_1() {
        let initial_price = 45584610003121481572705762227159u128;

        let mut context = setup_with(
            FixedType::from_u128(initial_price),
            300000000000000u128,
            10000000000u128,
            YasU256::from_u128(100000000000000000000000u128),
        );

        init_pool(&mut context);

        let yas_pool_address = context.contract_address(&yas_pool());

        // 3019294,467836 USDC
        let usdc_amount = YasU256::from_u128(3019293995782u128);

        // 3019294,467836 USDC
        let usdc_swapped_expected = 3019293995782u128;

        // 0,999000059110060056 ETH
        let eth_swapped_expected = 999000059110060056u128;

        // will trade USDC for ETH (USDC token_0, ETH token_1) so, ZFO true
        let zero_for_one = true;

        // When selling token 0 (zeroForOne is true) sqrtPriceLimitX96 must be
        // between the current price and the minimal sqrt(P) since selling token 0
        // moves the price down. Likewise, when selling token 1, sqrtPriceLimitX96
        // must be between the current price and the maximal sqrt(P) ​because price moves up.

        // In the while loop, we want to satisfy two conditions: full swap amount has not
        // been filled and current price isn’t equal to sqrtPriceLimitX96:
        let price_limit = FixedType::from_u128(initial_price / 1000u128);

        // Check balance before swap
        let user_token_0_balance_bf =
            context.call_entry_point(&token_0_name(), "balanceOf", s_calldata_starkfelt!(WALLET()));
        let user_token_1_balance_bf =
            context.call_entry_point(&token_1_name(), "balanceOf", s_calldata_starkfelt!(WALLET()));

        // Execute swap
        let result = context.call_entry_point(
            &yas_router(),
            "swap",
            s_calldata_starkfelt!(
                yas_pool_address,
                WALLET(),
                zero_for_one,
                usdc_amount,
                price_limit
            ),
        );

        // Check balance after swap
        let user_token_0_balance_af =
            context.call_entry_point(&token_0_name(), "balanceOf", s_calldata_starkfelt!(WALLET()));
        let user_token_1_balance_af =
            context.call_entry_point(&token_1_name(), "balanceOf", s_calldata_starkfelt!(WALLET()));

        assert_eq!(result, vec![]);
        assert_eq!(
            user_token_0_balance_bf[0] - user_token_0_balance_af[0],
            Felt::from(usdc_swapped_expected)
        );
        assert_eq!(
            user_token_1_balance_af[0] - user_token_1_balance_bf[0],
            Felt::from(eth_swapped_expected)
        );
    }
}

// mod PoolCase1 {
//             use super::test_pool;
//             use yas_core::tests::utils::pool_1::{SWAP_CASES_POOL_1,
// SWAP_EXPECTED_RESULTS_POOL_1};             use
// yas_core::tests::utils::swap_cases::SwapTestHelper::{POOL_CASES};
//
//             #[test]
//             #[available_gas(200000000000)]
//             fn test_pool_1_success_cases() {
//                 let pool_case = POOL_CASES()[1];
//                 let expected_cases = SWAP_EXPECTED_RESULTS_POOL_1();
//                 let (success_swap_cases, _) = SWAP_CASES_POOL_1();
//                 test_pool(pool_case, expected_cases, success_swap_cases);
//             }
//
//             #[test]
//             #[available_gas(200000000000)]
//             #[should_panic(expected: ('SPL', 'ENTRYPOINT_FAILED', 'ENTRYPOINT_FAILED'))]
//             fn test_pool_1_panics_0() {
//                 let PANIC_CASE = 0;
//                 let pool_case = POOL_CASES()[1];
//                 let (success_swap_cases, panic_swap_cases) = SWAP_CASES_POOL_1();
//                 let expected_cases =
//                     SWAP_EXPECTED_RESULTS_POOL_1(); //get random case, is never executed
//                 test_pool(
//                     pool_case,
//                     array![*expected_cases[PANIC_CASE]],
//                     array![*panic_swap_cases[PANIC_CASE]]
//                 );
//             }
//
//             #[test]
//             #[available_gas(200000000000)]
//             #[should_panic(expected: ('SPL', 'ENTRYPOINT_FAILED', 'ENTRYPOINT_FAILED'))]
//             fn test_pool_1_panics_1() {
//                 let PANIC_CASE = 1;
//                 let pool_case = POOL_CASES()[1];
//                 let (success_swap_cases, panic_swap_cases) = SWAP_CASES_POOL_1();
//                 let expected_cases =
//                     SWAP_EXPECTED_RESULTS_POOL_1(); //get random case, is never executed
//                 test_pool(
//                     pool_case,
//                     array![*expected_cases[PANIC_CASE]],
//                     array![*panic_swap_cases[PANIC_CASE]]
//                 );
//             }
//         }
//
//         fn test_pool(
//             pool_case: @PoolTestCase,
//             expected_cases: Array<SwapExpectedResults>,
//             swap_cases: Array<SwapTestCase>
//         ) {
//             let mut i = 0;
//             assert(expected_cases.len() == swap_cases.len(), 'wrong amount of expected cases');
//             loop {
//                 if i == expected_cases.len() {
//                     break;
//                 }
//                 // restart Pool
//                 let (yas_pool, yas_router, token_0, token_1) = setup_pool_for_swap_test(
//                     initial_price: *pool_case.starting_price,
//                     fee_amount: *pool_case.fee_amount,
//                     mint_positions: pool_case.mint_positions
//                 );
//                 let swap_case = swap_cases[i];
//                 let expected = expected_cases[i];
//
//                 // Save values before swap for compare
//                 let user_token_0_balance_bf = token_0.balanceOf(WALLET());
//                 let user_token_1_balance_bf = token_1.balanceOf(WALLET());
//                 let (fee_growth_global_0_X128_bf, fee_growth_global_1_X128_bf) = yas_pool
//                     .get_fee_growth_globals();
//
//                 let pool_balance_0_bf = token_0.balanceOf(yas_pool.contract_address);
//                 let pool_balance_1_bf = token_1.balanceOf(yas_pool.contract_address);
//                 let slot0_bf = yas_pool.get_slot_0();
//
//                 let mut amount_to_swap = IntegerTrait::<i256>::new(0, false); //Zeroable::zero();
//                 if *swap_case.has_exact_out {
//                     if *swap_case.exact_out { //exact OUT
//                         if *swap_case
//                             .zero_for_one { //so i check how much i should put swap IN in order
// to get those OUT tokens, the Asserts will still verify everything else
// amount_to_swap = *expected.amount_0_delta;                         } else {
//                             amount_to_swap = *expected.amount_1_delta;
//                         }
//                     } else { //exact IN, normal swap.
//                         amount_to_swap = *swap_case.amount_specified;
//                     }
//                 } else {
//                     amount_to_swap = IntegerTrait::<i256>::new((BoundedInt::max() / 2) - 1,
// false);                 }
//                 // Execute swap
//                 let (token_0_swapped_amount, token_1_swapped_amount) = swap_test_case(
//                     yas_router,
//                     yas_pool,
//                     token_0,
//                     token_1,
//                     *swap_case.zero_for_one,
//                     amount_to_swap,
//                     *swap_case.sqrt_price_limit
//                 );
//
//                 // Save values after swap to get deltas
//                 let (fee_growth_global_0_X128_af, fee_growth_global_1_X128_af) = yas_pool
//                     .get_fee_growth_globals();
//
//                 let user_token_0_balance_af = token_0.balanceOf(WALLET());
//                 let user_token_1_balance_af = token_1.balanceOf(WALLET());
//                 let (fee_growth_global_0_X128_af, fee_growth_global_1_X128_af) = yas_pool
//                     .get_fee_growth_globals();
//                 let (fee_growth_global_0_X128_delta, fee_growth_global_1_X128_delta) = (
//                     fee_growth_global_0_X128_af - fee_growth_global_0_X128_bf,
//                     fee_growth_global_1_X128_af - fee_growth_global_1_X128_bf
//                 );
//                 let slot0_af = yas_pool.get_slot_0();
//
//                 // Generate swap result values to compare with expected
//                 let (fee_growth_global_0_X128_delta, fee_growth_global_1_X128_delta) = (
//                     fee_growth_global_0_X128_af - fee_growth_global_0_X128_bf,
//                     fee_growth_global_1_X128_af - fee_growth_global_1_X128_bf
//                 );
//                 let execution_price = calculate_execution_price(
//                     token_0_swapped_amount, token_1_swapped_amount
//                 );
//
//                 let pool_balance_0_af = token_0.balanceOf(yas_pool.contract_address);
//                 let pool_balance_1_af = token_1.balanceOf(yas_pool.contract_address);
//
//                 let pool_price_bf = round_for_price_comparison(slot0_bf.sqrt_price_X96.mag);
//                 let pool_price_af = round_for_price_comparison(slot0_af.sqrt_price_X96.mag);
//
//                 let tick_bf = slot0_bf.tick;
//                 let tick_af = slot0_af.tick;
//
//                 let actual = SwapExpectedResults {
//                     amount_0_before: pool_balance_0_bf,
//                     amount_0_delta: IntegerTrait::<i256>::new(pool_balance_0_af, false)
//                         - IntegerTrait::<i256>::new(pool_balance_0_bf, false),
//                     amount_1_before: pool_balance_1_bf,
//                     amount_1_delta: IntegerTrait::<i256>::new(pool_balance_1_af, false)
//                         - IntegerTrait::<i256>::new(pool_balance_1_bf, false),
//                     execution_price: execution_price,
//                     fee_growth_global_0_X128_delta: fee_growth_global_0_X128_delta,
//                     fee_growth_global_1_X128_delta: fee_growth_global_1_X128_delta,
//                     pool_price_after: pool_price_af,
//                     pool_price_before: pool_price_bf,
//                     tick_after: tick_af,
//                     tick_before: tick_bf,
//                 };
//
//                 assert_swap_result_equals(actual, expected);
//                 i += 1;
//             };
//         }
//
//         fn assert_swap_result_equals(actual: SwapExpectedResults, expected: @SwapExpectedResults)
// {             assert(actual.amount_0_before == *expected.amount_0_before, 'wrong
// amount_0_before');             assert(actual.amount_0_delta == *expected.amount_0_delta, 'wrong
// amount_0_delta');             assert(actual.amount_1_before == *expected.amount_1_before, 'wrong
// amount_1_before');             assert(actual.amount_1_delta == *expected.amount_1_delta, 'wrong
// amount_1_delta');             assert(actual.execution_price == *expected.execution_price, 'wrong
// execution_price');             assert(
//                 actual.fee_growth_global_0_X128_delta ==
// *expected.fee_growth_global_0_X128_delta,                 'wrong fee_growth_global_0_X128'
//             );
//             assert(
//                 actual.fee_growth_global_1_X128_delta ==
// *expected.fee_growth_global_1_X128_delta,                 'wrong fee_growth_global_1_X128'
//             );
//             assert(
//                 actual.pool_price_before == *expected.pool_price_before, 'wrong
// pool_price_before'             );
//             assert(actual.pool_price_after == *expected.pool_price_after, 'wrong
// pool_price_after');
//
//             assert(actual.tick_after == *expected.tick_after, 'wrong tick_after');
//             assert(actual.tick_before == *expected.tick_before, 'wrong tick_before');
//         }
//     }
#[cfg(test)]
pub mod pool_case_1 {
    use super::*;

    #[test]
    fn test_pool_1_success_cases() {
        let mut context = setup_with(
            FixedType::from_u128(45584610003121481572705762227159u128),
            300000000000000u128,
            10000000000u128,
            YasU256::from_u128(100000000000000000000000u128),
        );

        init_pool(&mut context);

        let yas_pool_address = context.contract_address(&yas_pool());

        let result = context.call_entry_point(
            &yas_router(),
            "mint",
            s_calldata_starkfelt!(
                yas_pool_address,
                WALLET(),
                YasI32::from_i32(-22980),
                YasI32::from_i32(0),
                10000u128
            ),
        );

        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_pool_1_panics_0() {
        let mut context = setup_with(
            FixedType::from_u128(45584610003121481572705762227159u128),
            300000000000000u128,
            10000000000u128,
            YasU256::from_u128(100000000000000000000000u128),
        );

        init_pool(&mut context);

        let yas_pool_address = context.contract_address(&yas_pool());

        let result = context.call_entry_point(
            &yas_router(),
            "mint",
            s_calldata_starkfelt!(
                yas_pool_address,
                WALLET(),
                YasI32::from_i32(-22980),
                YasI32::from_i32(0),
                10000u128
            ),
        );

        assert_eq!(result, vec![]);
    }

    fn test_pool(
        pool_case: PoolCase,
        expected_cases: Vec<SwapExpectedResults>,
        swap_cases: Vec<SwapTestCase>,
    ) {
        let mut i = 0;
        assert_eq!(expected_cases.len(), swap_cases.len());
        loop {
            if i == expected_cases.len() {
                break;
            }

            // restart Pool
            let mut context = setup_pool_for_swap_test(
                pool_case.starting_price,
                pool_case.fee_amount,
                pool_case.mint_positions,
            );
        }
    }
}

pub struct MintPosition {
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub amount: u128,
}
pub struct PoolCase {
    pub starting_price: FixedType,
    pub fee_amount: u128,
    pub mint_positions: Vec<MintPosition>,
}

pub struct SwapExpectedResults {
    pub amount_0_before: YasU256,
    pub amount_0_delta: YasU256,
    pub amount_1_before: YasU256,
    pub amount_1_delta: YasI256,
    pub execution_price: YasU256,
    pub fee_growth_global_0_x128_delta: YasU256,
    pub fee_growth_global_1_x128_delta: YasU256,
    pub pool_price_after: YasU256,
    pub pool_price_before: YasU256,
    pub tick_after: YasI32,
    pub tick_before: YasI32,
}

struct SwapTestCase {
    pub zero_for_one: bool,
    pub has_exact_out: bool,
    pub exact_out: bool,
    pub amount_specified: YasI256,
    pub sqrt_price_limit: FixedType,
}
