use blockifier::execution::sierra_utils::{contract_address_to_felt, felt_to_starkfelt};
use blockifier::test_utils::testing_context::{
    Signers, StateFactory, TestContext, YASERC20Factory, YASFaucetFactory,
};
use starknet_api::hash::StarkFelt;
use starknet_types_core::felt::Felt;

fn setup() -> TestContext {
    let mut context = TestContext::new(YASERC20Factory::new()).with_caller(OWNER().into());

    context.patch_with_factory(YASFaucetFactory::new(
        context.contract_address(YASERC20Factory::name()),
    ));

    assert_eq!(
        context.call_entry_point(
            YASERC20Factory::name(),
            "transfer",
            vec![
                felt_to_starkfelt(contract_address_to_felt(
                    context.contract_address(YASFaucetFactory::name()),
                )),
                StarkFelt::from(4000000000000000000u128),
                StarkFelt::from(0u128),
            ],
        ),
        vec![Felt::from(true)]
    );

    context
}

#[allow(non_snake_case)]
fn OWNER() -> Signers {
    Signers::Alice.into()
}
#[allow(non_snake_case)]
fn WALLET() -> Signers {
    Signers::Bob.into()
}

#[allow(non_snake_case)]
fn OTHER() -> Signers {
    Signers::Charlie.into()
}

#[test]
fn deploys_yas_faucet() {
    let _ = setup();
}

#[test]
fn test_happy_path() {
    let mut context = setup();

    // assert(yas_erc_20.balanceOf(WALLET()) == 0, 'wrong balance');
    assert_eq!(
        context.call_entry_point(YASERC20Factory::name(), "balanceOf", vec![WALLET().into()]),
        vec![Felt::from(0), Felt::from(0)]
    );

    context = context.with_caller(WALLET().into());

    let result = context.call_entry_point(
        YASERC20Factory::name(),
        "balanceOf",
        vec![felt_to_starkfelt(contract_address_to_felt(
            context.contract_address(YASFaucetFactory::name()),
        ))],
    );

    println!("{:?}", result.first().unwrap().to_hex_string());

    let result = context.call_entry_point(YASFaucetFactory::name(), "faucet_mint", vec![]);

    println!("{:?}", result.first().unwrap().to_hex_string());
    assert_eq!(result, vec![]);

    assert_eq!(
        context.call_entry_point(YASERC20Factory::name(), "balanceOf", vec![WALLET().into()]),
        vec![Felt::from(1000), Felt::from(0)]
    );
}
