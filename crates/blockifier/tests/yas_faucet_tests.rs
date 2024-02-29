use blockifier::execution::sierra_utils::{contract_address_to_felt, felt_to_starkfelt};
use blockifier::test_utils::testing_context::{
    string_to_felt, TestContext, YASERC20Factory, YASFaucetFactory, FAUCET_NAME, OTHER, OWNER,
    WALLET, YASERC20_NAME,
};
use starknet_api::hash::StarkFelt;
use starknet_types_core::felt::Felt;

fn setup() -> TestContext {
    let mut context = TestContext::new(YASERC20Factory::default()).with_caller(OWNER().into());

    context.patch_with_factory(YASFaucetFactory::new(context.contract_address(&YASERC20_NAME())));

    assert_eq!(
        context.call_entry_point(
            &YASERC20_NAME(),
            "transfer",
            vec![
                felt_to_starkfelt(contract_address_to_felt(
                    context.contract_address(&FAUCET_NAME()),
                )),
                StarkFelt::from(4000000000000000000u128),
                StarkFelt::from(0u128),
            ],
        ),
        vec![Felt::from(true)]
    );

    context
}

#[test]
fn deploys_yas_faucet() {
    let _ = setup();
}

#[test]
fn test_happy_path() {
    let mut context = setup();

    assert_eq!(
        context.call_entry_point(&YASERC20_NAME(), "balanceOf", vec![WALLET().into()]),
        vec![Felt::from(0), Felt::from(0)]
    );

    context = context.with_caller(WALLET().into());

    assert_eq!(context.call_entry_point(&FAUCET_NAME(), "faucet_mint", vec![]), vec![]);

    assert_eq!(
        context.call_entry_point(&YASERC20_NAME(), "balanceOf", vec![WALLET().into()]),
        vec![Felt::from(1000), Felt::from(0)]
    );
}

#[test]
fn test_double_faucet_mint() {
    let mut context = setup();

    assert_eq!(
        context.call_entry_point(&YASERC20_NAME(), "balanceOf", vec![WALLET().into()]),
        vec![Felt::from(0), Felt::from(0)]
    );

    context = context.with_caller(WALLET().into());

    assert_eq!(context.call_entry_point(&FAUCET_NAME(), "faucet_mint", vec![]), vec![]);

    assert_eq!(
        context.call_entry_point(&YASERC20_NAME(), "balanceOf", vec![WALLET().into()]),
        vec![Felt::from(1000), Felt::from(0)]
    );

    context.set_timestamp(context.get_timestamp() + 86400 + 1);

    assert_eq!(context.call_entry_point(&FAUCET_NAME(), "faucet_mint", vec![]), vec![]);

    assert_eq!(
        context.call_entry_point(&YASERC20_NAME(), "balanceOf", vec![WALLET().into()]),
        vec![Felt::from(2000), Felt::from(0)]
    );
}

#[test]
fn test_withdraw_all_balance() {
    let mut context = setup();

    assert_eq!(
        context.call_entry_point(&YASERC20_NAME(), "balanceOf", vec![OTHER().into()]),
        vec![Felt::from(0), Felt::from(0)]
    );

    assert_eq!(
        context.call_entry_point(
            &YASERC20_NAME(),
            "balanceOf",
            vec![felt_to_starkfelt(contract_address_to_felt(
                context.contract_address(&FAUCET_NAME())
            ))]
        ),
        vec![Felt::from(4000000000000000000u128), Felt::from(0)]
    );

    context = context.with_caller(OWNER().into());

    assert_eq!(
        context.call_entry_point(&FAUCET_NAME(), "withdraw_all_balance", vec![OTHER().into()]),
        vec![]
    );

    assert_eq!(
        context.call_entry_point(&YASERC20_NAME(), "balanceOf", vec![OTHER().into()]),
        vec![Felt::from(4000000000000000000u128), Felt::from(0)]
    );

    assert_eq!(
        context.call_entry_point(&YASERC20_NAME(), "balanceOf", vec![OWNER().into()]),
        vec![Felt::from(0), Felt::from(0)]
    );
}

#[test]
fn test_faucet_constructor() {
    let mut context = setup();

    assert_eq!(
        context.call_entry_point(&FAUCET_NAME(), "get_amount_faucet", vec![]),
        vec![Felt::from(4000000000000000000u128), Felt::from(0)]
    );

    assert_eq!(
        context.call_entry_point(&FAUCET_NAME(), "get_token_address", vec![]),
        vec![contract_address_to_felt(context.contract_address(&YASERC20_NAME()))]
    );

    assert_eq!(
        context.call_entry_point(&FAUCET_NAME(), "get_withdrawal_amount", vec![]),
        vec![Felt::from(1000u128), Felt::from(0)]
    );

    assert_eq!(
        context.call_entry_point(&FAUCET_NAME(), "get_wait_time", vec![]),
        vec![Felt::from(86400u128)]
    );
}

#[test]
fn test_withdrawal_not_allowed_panic() {
    let mut context = setup();

    assert_eq!(
        context.call_entry_point(&YASERC20_NAME(), "balanceOf", vec![WALLET().into()]),
        vec![Felt::from(0), Felt::from(0)]
    );

    context = context.with_caller(WALLET().into());

    assert_eq!(context.call_entry_point(&FAUCET_NAME(), "faucet_mint", vec![]), vec![]);

    assert_eq!(
        context.call_entry_point(&YASERC20_NAME(), "balanceOf", vec![WALLET().into()]),
        vec![Felt::from(1000), Felt::from(0)]
    );

    context.set_timestamp(context.get_timestamp() + 86400);

    assert_eq!(
        context.call_entry_point(&FAUCET_NAME(), "faucet_mint", vec![]),
        vec![string_to_felt("Not allowed to withdraw").unwrap()]
    );

    assert_eq!(
        context.call_entry_point(&YASERC20_NAME(), "balanceOf", vec![WALLET().into()]),
        vec![Felt::from(1000), Felt::from(0)]
    );
}

#[test]
fn test_insufficient_balance_panic() {
    let mut context = TestContext::new(YASERC20Factory::default()).with_caller(OWNER().into());

    context.patch_with_factory(YASFaucetFactory::new(context.contract_address(&YASERC20_NAME())));

    context = context.with_caller(WALLET().into());

    assert_eq!(
        context.call_entry_point(&FAUCET_NAME(), "faucet_mint", vec![]),
        vec![string_to_felt("There is not enough balance").unwrap()]
    );
}

#[test]
fn test_owner_withdrawal_panic() {
    let mut context = setup();

    assert_eq!(
        context.call_entry_point(&YASERC20_NAME(), "balanceOf", vec![OTHER().into()]),
        vec![Felt::from(0), Felt::from(0)]
    );

    assert_eq!(
        context.call_entry_point(
            &YASERC20_NAME(),
            "balanceOf",
            vec![felt_to_starkfelt(contract_address_to_felt(
                context.contract_address(&FAUCET_NAME())
            ))]
        ),
        vec![Felt::from(4000000000000000000u128), Felt::from(0)]
    );

    context = context.with_caller(WALLET().into());

    assert_eq!(
        context.call_entry_point(&FAUCET_NAME(), "withdraw_all_balance", vec![OTHER().into()]),
        vec![string_to_felt("Caller is not the owner").unwrap()]
    );

    assert_eq!(
        context.call_entry_point(&YASERC20_NAME(), "balanceOf", vec![OTHER().into()]),
        vec![Felt::from(0), Felt::from(0)]
    );

    assert_eq!(
        context.call_entry_point(
            &YASERC20_NAME(),
            "balanceOf",
            vec![felt_to_starkfelt(contract_address_to_felt(
                context.contract_address(&FAUCET_NAME())
            ))]
        ),
        vec![Felt::from(4000000000000000000u128), Felt::from(0)]
    );
}
