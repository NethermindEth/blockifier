use blockifier::execution::sierra_utils::{contract_address_to_felt, felt_to_starkfelt};
use blockifier::test_utils::testing_context::{
    StateFactory, TestContext, YASERC20Factory, YASFaucetFactory, OTHER, OWNER, WALLET,
};
use starknet_api::hash::StarkFelt;
use starknet_types_core::felt::Felt;

pub const NOT_ALLOWED_TO_WITHDRAW: &str = "0x4e6f7420616c6c6f77656420746f207769746864726177";
pub const THERE_IS_NOT_ENOUGH_BALANCE: &str =
    "0x5468657265206973206e6f7420656e6f7567682062616c616e6365";
pub const CALLER_IS_NOT_THE_OWNER: &str = "0x43616c6c6572206973206e6f7420746865206f776e6572";

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

#[test]
fn deploys_yas_faucet() {
    let _ = setup();
}

#[test]
fn test_happy_path() {
    let mut context = setup();

    assert_eq!(
        context.call_entry_point(YASERC20Factory::name(), "balanceOf", vec![WALLET().into()]),
        vec![Felt::from(0), Felt::from(0)]
    );

    context = context.with_caller(WALLET().into());

    assert_eq!(context.call_entry_point(YASFaucetFactory::name(), "faucet_mint", vec![]), vec![]);

    assert_eq!(
        context.call_entry_point(YASERC20Factory::name(), "balanceOf", vec![WALLET().into()]),
        vec![Felt::from(1000), Felt::from(0)]
    );
}

#[test]
fn test_double_faucet_mint() {
    let mut context = setup();

    assert_eq!(
        context.call_entry_point(YASERC20Factory::name(), "balanceOf", vec![WALLET().into()]),
        vec![Felt::from(0), Felt::from(0)]
    );

    context = context.with_caller(WALLET().into());

    assert_eq!(context.call_entry_point(YASFaucetFactory::name(), "faucet_mint", vec![]), vec![]);

    assert_eq!(
        context.call_entry_point(YASERC20Factory::name(), "balanceOf", vec![WALLET().into()]),
        vec![Felt::from(1000), Felt::from(0)]
    );

    context.set_timestamp(context.get_timestamp() + 86400 + 1);

    assert_eq!(context.call_entry_point(YASFaucetFactory::name(), "faucet_mint", vec![]), vec![]);

    assert_eq!(
        context.call_entry_point(YASERC20Factory::name(), "balanceOf", vec![WALLET().into()]),
        vec![Felt::from(2000), Felt::from(0)]
    );
}

#[test]
fn test_withdraw_all_balance() {
    let mut context = setup();

    assert_eq!(
        context.call_entry_point(YASERC20Factory::name(), "balanceOf", vec![OTHER().into()]),
        vec![Felt::from(0), Felt::from(0)]
    );

    assert_eq!(
        context.call_entry_point(
            YASERC20Factory::name(),
            "balanceOf",
            vec![felt_to_starkfelt(contract_address_to_felt(
                context.contract_address(YASFaucetFactory::name())
            ))]
        ),
        vec![Felt::from(4000000000000000000u128), Felt::from(0)]
    );

    context = context.with_caller(OWNER().into());

    assert_eq!(
        context.call_entry_point(
            YASFaucetFactory::name(),
            "withdraw_all_balance",
            vec![OTHER().into()]
        ),
        vec![]
    );

    assert_eq!(
        context.call_entry_point(YASERC20Factory::name(), "balanceOf", vec![OTHER().into()]),
        vec![Felt::from(4000000000000000000u128), Felt::from(0)]
    );

    assert_eq!(
        context.call_entry_point(YASERC20Factory::name(), "balanceOf", vec![OWNER().into()]),
        vec![Felt::from(0), Felt::from(0)]
    );
}

#[test]
fn test_faucet_constructor() {
    let mut context = setup();

    assert_eq!(
        context.call_entry_point(YASFaucetFactory::name(), "get_amount_faucet", vec![]),
        vec![Felt::from(4000000000000000000u128), Felt::from(0)]
    );

    assert_eq!(
        context.call_entry_point(YASFaucetFactory::name(), "get_token_address", vec![]),
        vec![contract_address_to_felt(context.contract_address(YASERC20Factory::name()))]
    );

    assert_eq!(
        context.call_entry_point(YASFaucetFactory::name(), "get_withdrawal_amount", vec![]),
        vec![Felt::from(1000u128), Felt::from(0)]
    );

    assert_eq!(
        context.call_entry_point(YASFaucetFactory::name(), "get_wait_time", vec![]),
        vec![Felt::from(86400u128)]
    );
}

#[test]
fn test_withdrawal_not_allowed_panic() {
    let mut context = setup();

    assert_eq!(
        context.call_entry_point(YASERC20Factory::name(), "balanceOf", vec![WALLET().into()]),
        vec![Felt::from(0), Felt::from(0)]
    );

    context = context.with_caller(WALLET().into());

    assert_eq!(context.call_entry_point(YASFaucetFactory::name(), "faucet_mint", vec![]), vec![]);

    assert_eq!(
        context.call_entry_point(YASERC20Factory::name(), "balanceOf", vec![WALLET().into()]),
        vec![Felt::from(1000), Felt::from(0)]
    );

    context.set_timestamp(context.get_timestamp() + 86400);

    assert_eq!(
        context.call_entry_point(YASFaucetFactory::name(), "faucet_mint", vec![]),
        vec![Felt::from_hex(NOT_ALLOWED_TO_WITHDRAW).unwrap()]
    );

    assert_eq!(
        context.call_entry_point(YASERC20Factory::name(), "balanceOf", vec![WALLET().into()]),
        vec![Felt::from(1000), Felt::from(0)]
    );
}

#[test]
fn test_insufficient_balance_panic() {
    let mut context = TestContext::new(YASERC20Factory::new()).with_caller(OWNER().into());

    context.patch_with_factory(YASFaucetFactory::new(
        context.contract_address(YASERC20Factory::name()),
    ));

    context = context.with_caller(WALLET().into());

    assert_eq!(
        context.call_entry_point(YASFaucetFactory::name(), "faucet_mint", vec![]),
        vec![Felt::from_hex(THERE_IS_NOT_ENOUGH_BALANCE).unwrap()]
    );
}

#[test]
fn test_owner_withdrawal_panic() {
    let mut context = setup();

    assert_eq!(
        context.call_entry_point(YASERC20Factory::name(), "balanceOf", vec![OTHER().into()]),
        vec![Felt::from(0), Felt::from(0)]
    );

    assert_eq!(
        context.call_entry_point(
            YASERC20Factory::name(),
            "balanceOf",
            vec![felt_to_starkfelt(contract_address_to_felt(
                context.contract_address(YASFaucetFactory::name())
            ))]
        ),
        vec![Felt::from(4000000000000000000u128), Felt::from(0)]
    );

    context = context.with_caller(WALLET().into());

    assert_eq!(
        context.call_entry_point(
            YASFaucetFactory::name(),
            "withdraw_all_balance",
            vec![OTHER().into()]
        ),
        vec![Felt::from_hex(CALLER_IS_NOT_THE_OWNER).unwrap()]
    );

    assert_eq!(
        context.call_entry_point(YASERC20Factory::name(), "balanceOf", vec![OTHER().into()]),
        vec![Felt::from(0), Felt::from(0)]
    );

    assert_eq!(
        context.call_entry_point(
            YASERC20Factory::name(),
            "balanceOf",
            vec![felt_to_starkfelt(contract_address_to_felt(
                context.contract_address(YASFaucetFactory::name())
            ))]
        ),
        vec![Felt::from(4000000000000000000u128), Felt::from(0)]
    );
}
