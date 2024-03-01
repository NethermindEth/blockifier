use blockifier::execution::contract_class::SierraContractClassV1;
use blockifier::execution::sierra_utils::{contract_address_to_starkfelt, felt_to_starkfelt};
use blockifier::test_utils::testing_context::{
    FeeAmount, Signers, StateFactory, TestContext, YASERC20Factory, YASFactory, YASPoolFactory,
    YASRouterFactory, FACTORY_NAME, OWNER, WALLET,
};
use blockifier::test_utils::{TEST_YAS_POOL_CONTRACT_CLASS_HASH, YAS_POOL_CONTRACT_PATH};
use cairo_felt::Felt252;
use num_integer::Integer;
use starknet_api::class_hash;
use starknet_api::core::{ClassHash, ContractAddress, PatriciaKey};
use starknet_api::hash::{StarkFelt, StarkHash};
use starknet_types_core::felt::Felt;

const MINT_AMOUNT: u128 = 1000000000000000000000000u128;
// 1461446703485210103287273052203988822378723970342
// const MAX_SQRT_RATIO_LO = 1461446703485210103287273052203988822378723970342u128;
// 4295128739
// const MAX_SQRT_RATIO_HI = 4295128739u128;

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
        vec![encode_price_sqrt_1_10(), StarkFelt::from(0u8), StarkFelt::from(0u8)],
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

fn setup_with_pool() -> TestContext {
    let mut context = setup();
    init_pool(&mut context);
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

fn deploy_only_pool() -> TestContext {
    let context = TestContext::new(YASPoolFactory::new(vec![
        MOCK_FACTORY_ADDRESS().into(),
        MOCK_TOKEN_1_ADDRESS().into(),
        MOCK_TOKEN_2_ADDRESS().into(),
        Felt::from(FeeAmount::Medium.fee_amount()),
        Felt::from(FeeAmount::Medium.tick_spacing()),
        Felt::from(0u8),
    ]));

    context
}

fn encode_price_sqrt_1_10() -> StarkFelt {
    // // returns result of encode_price_sqrt(1, 10) on v3-core typescript impl.
    // fn encode_price_sqrt_1_10() -> FixedType {
    //     FP64x96Impl::new(25054144837504793118641380156, false)
    // }

    StarkFelt::from_u128(25054144837504793118641380156)
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

pub const MIN_TICK: i32 = -887272;
pub const MAX_TICK: i32 = 887272;

#[cfg(test)]
mod constructor_tests {
    use super::*;

    #[test]
    fn deploys() {
        let _ = setup_with_pool();
    }
}
pub const MIN_SQRT_RATIO: u128 = 4295128739;

#[cfg(test)]
mod initialize_tests {
    use blockifier::test_utils::testing_context::string_to_felt;

    use super::*;

    #[test]
    fn test_fails_if_already_initialized() {
        let mut context = deploy_only_pool();

        let result = context.call_entry_point(
            &yas_pool(),
            "initialize",
            vec![encode_price_sqrt_1_10(), StarkFelt::from(0u8), StarkFelt::from(0u8)],
        );

        assert_eq!(result, vec![]);

        let result = context.call_entry_point(
            &yas_pool(),
            "initialize",
            vec![encode_price_sqrt_1_10(), StarkFelt::from(0u8), StarkFelt::from(0u8)],
        );

        assert_eq!(result, vec![string_to_felt("AI").unwrap()]);
    }

    #[test]
    fn test_fails_if_price_is_too_low() {
        let mut context = deploy_only_pool();

        let result = context.call_entry_point(
            &yas_pool(),
            "initialize",
            vec![StarkFelt::from(1u128), StarkFelt::from(0u8), StarkFelt::from(0u8)],
        );

        assert_eq!(result, vec![string_to_felt("R").unwrap()]);
    }

    #[test]
    fn test_fails_if_price_is_min_sqrt_ratio_minus_1() {
        let mut context = deploy_only_pool();

        let result = context.call_entry_point(
            &yas_pool(),
            "initialize",
            vec![StarkFelt::from(MIN_SQRT_RATIO - 1), StarkFelt::from(0u8), StarkFelt::from(0u8)],
        );

        assert_eq!(result, vec![string_to_felt("R").unwrap()]);
    }

    // #[test]
    // #[available_gas(200000000)]
    // #[should_panic(expected: ('R', 'ENTRYPOINT_FAILED'))]
    // fn test_fails_if_price_is_too_high() {
    //     let yas_pool = deploy(
    //         FACTORY_ADDRESS(), TOKEN_A(), TOKEN_B(), 5, IntegerTrait::<i32>::new(1, false)
    //     );
    //
    //     let sqrt_price_X96 = FixedTrait::new(pow(2, 160) - 1, false);
    //     yas_pool.initialize(sqrt_price_X96);
    // }
    #[test]
    fn test_fails_if_price_is_too_high() {
        let mut context = deploy_only_pool();

        let result = context.call_entry_point(
            &yas_pool(),
            "initialize",
            vec![StarkFelt::from(0u8), StarkFelt::from(2u128.pow(160 - 128)), StarkFelt::from(0u8)],
        );

        assert_eq!(result, vec![string_to_felt("R").unwrap()]);
    }

    // #[test]
    // #[available_gas(200000000)]
    // #[should_panic(expected: ('R', 'ENTRYPOINT_FAILED'))]
    // fn test_fails_if_price_is_max_sqrt_ratio() {
    //     let yas_pool = deploy(
    //         FACTORY_ADDRESS(), TOKEN_A(), TOKEN_B(), 5, IntegerTrait::<i32>::new(1, false)
    //     );
    //
    //     let sqrt_price_X96 = FixedTrait::new(MAX_SQRT_RATIO, false);
    //     yas_pool.initialize(sqrt_price_X96);
    // }
    #[test]
    fn test_fails_if_price_is_max_sqrt_ratio() {
        todo!("Implement test_fails_if_price_is_max_sqrt_ratio");
        // let mut context = deploy_only_pool();
        //
        // let result = context.call_entry_point(
        //     &yas_pool(),
        //     "initialize",
        //     vec![StarkFelt::from(MAX_SQRT_RATIO), StarkFelt::from(0u8), StarkFelt::from(0u8)],
        // )
        //;
    }

    // #[test]
    // #[available_gas(200000000)]
    // fn test_can_be_initialized_at_min_sqrt_ratio() {
    //     let mut state = STATE();
    //
    //     let sqrt_price_X96 = FixedTrait::new(MIN_SQRT_RATIO, false);
    //     YASPoolImpl::initialize(ref state, sqrt_price_X96);
    //
    //     let expected = Slot0 {
    //         sqrt_price_X96: FixedTrait::new(MIN_SQRT_RATIO, false),
    //         tick: min_tick(IntegerTrait::<i32>::new(1, false)),
    //         fee_protocol: 0
    //     };
    //
    //     assert(InternalImpl::get_slot_0(@state) == expected, 'slot 0 wrong initialization');
    // }
    #[test]
    fn test_can_be_initialized_at_min_sqrt_ratio() {
        let mut context = deploy_only_pool();

        let result = context.call_entry_point(
            &yas_pool(),
            "initialize",
            vec![StarkFelt::from(MIN_SQRT_RATIO), StarkFelt::from(0u8), StarkFelt::from(0u8)],
        );

        assert_eq!(result, vec![]);

        let result = context.call_entry_point(&yas_pool(), "get_slot_0", vec![]);

        println!("Result : {:?}", result);

        assert_eq!(
            result,
            vec![
                Felt::from(MIN_SQRT_RATIO),
                Felt::from(0u8),
                Felt::from(0u8),
                // Felt::from(0u8),
                Felt::from(MIN_TICK),
                Felt::from(0u8),
                Felt::from(0u8),
            ]
        );
    }
}
pub fn min_tick(tick_spacing: i32) -> i32 {
    (MIN_TICK / tick_spacing) * tick_spacing
}
