use near_sdk::{env, PromiseResult, ext_contract, near_bindgen, AccountId, Promise};
use near_sdk::json_types::{U128};
use borsh::{BorshDeserialize, BorshSerialize};
use serde_json::json;
use std::str;

const SINGLE_CALL_GAS: u64 = 20_000_000_000_000; // 2 x 10^14
const TRANSFER_FROM_NEAR_COST: u128 = 36_500_000_000_000_000_000_000; // 365 x 10^20

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Simulation {}

impl Default for Simulation {
    fn default() -> Self {
        env::panic(b"Contract must be initialized before usage with 'new' function call.")
    }
}

#[ext_contract(ext_fungible_token)]
pub trait ExtFunToken {
    fn transfer(&mut self, new_owner_id: AccountId, amount: U128);
}

#[ext_contract(ext_this_contract)]
pub trait ExtSimulation {
    fn post_transfer(&mut self);
}

#[near_bindgen]
impl Simulation {
    #[init]
    pub fn new() -> Self {
        assert!(env::state_read::<Self>().is_none(), "Already initialized");
        Self {}
    }

    /// This is a silly example that will increment a counter
    /// and send a fungible token if the number is even
    pub fn cross_contract_increment(&mut self, counter_account: AccountId, token_account: AccountId) {
        let promise_increment = env::promise_create(
            counter_account.clone(),
            b"increment",
            json!({}).to_string().as_bytes(),
            0,
            SINGLE_CALL_GAS,
        );

        let promise_get_num = env::promise_then(
            promise_increment,
            counter_account.clone(),
            b"get_num",
            json!({
                "account": env::predecessor_account_id()
            }).to_string().as_bytes(),
            0,
            SINGLE_CALL_GAS,
        );

        // call this contract's request function after the transfer
        let promise_increment_callback = env::promise_then(
            promise_get_num,
            env::current_account_id(),
            b"post_increment",
            json!({
                "token_account": token_account
            }).to_string().as_bytes(),
            0,
            SINGLE_CALL_GAS * 3
        );

        env::promise_return(promise_increment_callback);
    }

    pub fn post_increment(&mut self, token_account: AccountId) {
        env::log(b"Top of post_increment");
        // this method should only ever be called from this contract
        self._only_owner_predecessor();
        assert_eq!(env::promise_results_count(), 1);
        // ensure successful promise, meaning tokens are transferred
        let new_num_bytes = match env::promise_result(0) {
            PromiseResult::Successful(x) => x,
            PromiseResult::Failed => env::panic(b"(post_increment) The promise failed. See receipt failures."),
            PromiseResult::NotReady => env::panic(b"The promise was not ready."),
        };

        let new_num_str = str::from_utf8(new_num_bytes.as_slice()).expect("Issue turning Vec<u8> into &str apparently");
        let new_num = u8::from_str_radix(new_num_str, 10).expect("Issue turning &str into u8 apparently");
        env::log(format!("new_num came back: {:?}", new_num).as_bytes());

        self.send_token_if_counter_even(new_num, token_account, env::signer_account_id());
    }

    pub fn send_token_if_counter_even(&mut self, new_num: u8, token_account: AccountId, recipient_account: AccountId) -> Promise {
        self._only_owner_predecessor();
        assert!(self._is_even(new_num), "Number is not even");
        ext_fungible_token::transfer(recipient_account, U128(1), &token_account, TRANSFER_FROM_NEAR_COST, SINGLE_CALL_GAS)
        .then(ext_this_contract::post_transfer(&env::current_account_id(), 0, SINGLE_CALL_GAS))
    }

    // Note that currently, the simulation tests will not make it this far
    // This is a callback in a callback which is not (yet) supported
    pub fn post_transfer(&mut self) {
        self._only_owner_predecessor();
        assert_eq!(env::promise_results_count(), 1);
        match env::promise_result(0) {
            // We don't care about the result this time, hence the underscore
            // This is how we'll get the number soon, though.
            PromiseResult::Successful(_) => {},
            PromiseResult::Failed => env::panic(b"(post_transfer) The promise failed. See receipt failures."),
            PromiseResult::NotReady => env::panic(b"The promise was not ready."),
        };
        env::log(b"You've received a fungible token for incrementing to an even number.")
    }

    fn _is_even(&self, num: u8) -> bool {
        num & 1 == 0
    }

    /// This is a helper function with the promises happening.
    /// The predecessor will be this account calling itself after transferring
    /// fungible tokens. Used for functions called via promises where we
    /// do not want end user accounts calling them directly.
    fn _only_owner_predecessor(&mut self) {
        assert_eq!(env::predecessor_account_id(), env::current_account_id(), "Only contract owner can sign transactions for this method.");
    }
}
