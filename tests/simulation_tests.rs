mod utils;
use crate::utils::{ExternalUser, MAX_GAS, FUNGIBLE_TOKEN_ACCOUNT_ID, COUNTER_ACCOUNT_ID, SIMULATION_ACCOUNT_ID, ALICE_ACCOUNT_ID};
use near_primitives::transaction::ExecutionStatus;
use near_runtime_standalone::RuntimeStandalone;
use near_sdk::json_types::{U128, U64};
use serde_json::json;
use utils::{near_view, near_call, new_root, ntoy, NewFungibleTokenArgs, deploy_and_init_fungible_token, deploy_and_init_counter, deploy_simulation_example};

#[test]
fn deploy_fungible_check_total_supply() {
    let (mut r, _, fungible_token, _, _, _) = basic_setup();
    let total_supply = 1_000_000;

    let args = NewFungibleTokenArgs {
        owner_id: FUNGIBLE_TOKEN_ACCOUNT_ID.into(),
        total_supply: U128(total_supply.clone())
    };

    deploy_and_init_fungible_token(&mut r,
        &fungible_token,
        "new",
        U64(MAX_GAS),
        &args).unwrap();

    let returned_supply: U128 = near_view(&r, &FUNGIBLE_TOKEN_ACCOUNT_ID.into(), "get_total_supply", "");
    assert_eq!(returned_supply.0, total_supply);
    println!("Note that we can use println! instead of env::log in simulation tests.");
    let demo_variable = "-- --nocapture".to_string();
    println!("Just remember to to add this after 'cargo test': '{}'", demo_variable);
}

#[test]
fn deploy_fungible_send_alice_tokens() {
    let (mut r, _, fungible_token, _, _, _)= basic_setup();

    let args = NewFungibleTokenArgs {
        owner_id: FUNGIBLE_TOKEN_ACCOUNT_ID.into(),
        total_supply: U128(1_000_000)
    };

    deploy_and_init_fungible_token(&mut r,
        &fungible_token,
        "new",
        U64(MAX_GAS),
        &args).unwrap();

    let alice_balance: U128 = near_view(
        &r,
        &FUNGIBLE_TOKEN_ACCOUNT_ID.into(),
        "get_balance",
        &json!({
            "owner_id": ALICE_ACCOUNT_ID,
        })
    );
    // Confirm Alice's initial balance is 0
    assert_eq!(alice_balance.0, 0);
    // send some to Alice
    near_call(&mut r,
              &fungible_token,
              &FUNGIBLE_TOKEN_ACCOUNT_ID,
              "transfer",
              &serde_json::to_vec(&json!({
            "new_owner_id": ALICE_ACCOUNT_ID,
            "amount": "191919",
        }),).unwrap(),
              U64(MAX_GAS),
              36_500_000_000_000_000_000_000
    ).unwrap();

    let alice_balance: U128 = near_view(
        &r,
        &FUNGIBLE_TOKEN_ACCOUNT_ID.into(),
        "get_balance",
        &json!({
            "owner_id": ALICE_ACCOUNT_ID,
        })
    );
    // Confirm Alice's initial balance has increased to set amount
    assert_eq!(alice_balance.0, 191_919);
}

#[test]
fn deploy_all_check_allowance_before_increment() {
    let (mut r, _, fungible_token, counter, simulation_example, alice)= basic_setup();

    let args = NewFungibleTokenArgs {
        owner_id: FUNGIBLE_TOKEN_ACCOUNT_ID.into(),
        total_supply: U128(1_000_000)
    };

    deploy_and_init_fungible_token(&mut r,
        &fungible_token,
        "new",
        U64(MAX_GAS),
        &args).unwrap();

    deploy_and_init_counter(&mut r,
        &counter,
        "new",
        U64(MAX_GAS)).unwrap();

    deploy_simulation_example(&mut r,
        &simulation_example,
        "new",
        U64(MAX_GAS)).unwrap();

    let mut alice_counter: u8 = near_view(
        &r,
        &COUNTER_ACCOUNT_ID.into(),
        "get_num",
        &json!({
            "account": ALICE_ACCOUNT_ID
        })
    );

    assert_eq!(alice_counter.clone(), 0);

    let mut execution_outcome = near_call(&mut r,
        &alice,
        &COUNTER_ACCOUNT_ID,
        "increment",
        &[],
        U64(MAX_GAS),
        0
    ).unwrap();

    println!("Log(s) {:?}", execution_outcome.logs);

    // Make sure it was successful
    assert_eq!(execution_outcome.status, ExecutionStatus::SuccessValue(vec![]));

    alice_counter = near_view(
        &r,
        &COUNTER_ACCOUNT_ID.into(),
        "get_num",
        &json!({
            "account": ALICE_ACCOUNT_ID
        })
    );

    assert_eq!(alice_counter.clone(), 1);

    // Now we expect that when we increment again, the number will be two, which will move a fungible token
    // Before we can move over the fungible token, though, we need to

    // Check Alice's fungible token balance before, which should be zero.
    let alice_tokens: U128 = near_view(
        &r,
        &FUNGIBLE_TOKEN_ACCOUNT_ID.into(),
        "get_balance",
        &json!({
            "owner_id": ALICE_ACCOUNT_ID
        })
    );

    assert_eq!(alice_tokens.clone().0, 0);

    // Now increment again
    execution_outcome = near_call(&mut r,
        &alice,
        &SIMULATION_ACCOUNT_ID,
        "cross_contract_increment",
        &serde_json::to_vec(&json!({
            "counter_account": COUNTER_ACCOUNT_ID,
            "token_account": FUNGIBLE_TOKEN_ACCOUNT_ID,
        }),).unwrap(),
        U64(MAX_GAS),
        0
    ).unwrap();

    println!("Log(s) {:?}", execution_outcome.logs);
    // Make sure it was successful
    assert_eq!(execution_outcome.status, ExecutionStatus::SuccessValue(vec![]));

    // Check that the number has increased to 2
    alice_counter = near_view(
        &r,
        &COUNTER_ACCOUNT_ID.into(),
        "get_num",
        &json!({
            "account": ALICE_ACCOUNT_ID
        })
    );

    assert_eq!(alice_counter.clone(), 2);

    // Check that the fungible token has been given to Alice since 2 is an even number
    // Note: this is a current limitation with simulation tests.
    // At this time you cannot send more cross-contract calls inside of a cross-contract callback
    // Intentionally commented out the final assertion that would reasonably succeed
    /*
    let alice_new_tokens: U128 = near_view(
        &r,
        &FUNGIBLE_TOKEN_ACCOUNT_ID.into(),
        "get_balance",
        &json!({
            "owner_id": ALICE_ACCOUNT_ID
        })
    );

    assert_eq!(alice_new_tokens.clone().0, 1);
    */
}

fn basic_setup() -> (RuntimeStandalone, ExternalUser, ExternalUser, ExternalUser, ExternalUser, ExternalUser) {
    let (mut r, main) = new_root("main.testnet".into());

    let fungible_token = main
        .create_external(&mut r, FUNGIBLE_TOKEN_ACCOUNT_ID.into(), ntoy(1_000_000))
        .unwrap();
    let counter = main
        .create_external(&mut r, COUNTER_ACCOUNT_ID.into(), ntoy(1_000_000))
        .unwrap();
    let simulation = main
        .create_external(&mut r, SIMULATION_ACCOUNT_ID.into(), ntoy(1_000_000))
        .unwrap();
    let alice = main
        .create_external(&mut r, ALICE_ACCOUNT_ID.into(), ntoy(1_000_000))
        .unwrap();
    (r, main, fungible_token, counter, simulation, alice)
}
