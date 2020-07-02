use near_crypto::{InMemorySigner, KeyType, Signer};
use near_primitives::{
    account::{AccessKey, Account},
    errors::{RuntimeError, TxExecutionError},
    hash::CryptoHash,
    transaction::{ExecutionOutcome, ExecutionStatus, Transaction},
    types::{AccountId, Balance},
};
use near_runtime_standalone::{init_runtime_and_signer, RuntimeStandalone};
use near_sdk::json_types::{U64, U128};
use serde::de::DeserializeOwned;
use serde::Serialize;

pub const MAX_GAS: u64 = 300_000_000_000_000;

pub const COUNTER_ACCOUNT_ID: &str = "counter";
pub const FUNGIBLE_TOKEN_ACCOUNT_ID: &str = "fungible_token";
pub const SIMULATION_ACCOUNT_ID: &str = "simulation";
pub const ALICE_ACCOUNT_ID: &str = "alice";

/// NEAR to yoctoNEAR
pub fn ntoy(near_amount: Balance) -> Balance {
    near_amount * 10u128.pow(24)
}

lazy_static::lazy_static! {
    static ref FUNGIBLE_TOKEN_BYTES: &'static [u8] = include_bytes!("res/fungible_token.wasm").as_ref();
    static ref COUNTER_BYTES: &'static [u8] = include_bytes!("res/counter.wasm").as_ref();
    static ref SIMULATION_EXAMPLE_BYTES: &'static [u8] = include_bytes!("../res/simulation_example.wasm").as_ref();
}

type TxResult = Result<ExecutionOutcome, ExecutionOutcome>;

fn outcome_into_result(outcome: ExecutionOutcome) -> TxResult {
    match outcome.status {
        ExecutionStatus::SuccessValue(_) => Ok(outcome),
        ExecutionStatus::Failure(_) => Err(outcome),
        ExecutionStatus::SuccessReceiptId(_) => panic!("Unresolved ExecutionOutcome run runitme.resolve(tx) to resolve the filnal outcome of tx"),
        ExecutionStatus::Unknown => unreachable!()
    }
}

/// Specific to fungible token contract's `new` method
#[derive(Serialize)]
pub struct NewFungibleTokenArgs {
    pub owner_id: AccountId,
    pub total_supply: U128,
}

#[derive(Clone)]
pub struct ExternalUser {
    pub account_id: AccountId,
    pub signer: InMemorySigner,
}

impl ExternalUser {
    #[allow(dead_code)]
    pub fn new(account_id: AccountId, signer: InMemorySigner) -> Self {
        Self { account_id, signer }
    }

    #[allow(dead_code)]
    pub fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    #[allow(dead_code)]
    pub fn signer(&self) -> &InMemorySigner {
        &self.signer
    }

    #[allow(dead_code)]
    pub fn account(&self, runtime: &mut RuntimeStandalone) -> Account {
        runtime
            .view_account(&self.account_id)
            .expect("Account should be there")
    }

    pub fn create_external(
        &self,
        runtime: &mut RuntimeStandalone,
        new_account_id: AccountId,
        amount: Balance,
    ) -> Result<ExternalUser, ExecutionOutcome> {
        let new_signer =
            InMemorySigner::from_seed(&new_account_id, KeyType::ED25519, &new_account_id);
        let tx = self
            .new_tx(runtime, new_account_id.clone())
            .create_account()
            .add_key(new_signer.public_key(), AccessKey::full_access())
            .transfer(amount)
            .sign(&self.signer);
        let res = runtime.resolve_tx(tx);

        // This logic be rewritten, FYI
        if let Err(err) = res.clone() {
            if let RuntimeError::InvalidTxError(tx_err) = err {
                let mut out = ExecutionOutcome::default();
                out.status = ExecutionStatus::Failure(TxExecutionError::InvalidTxError(tx_err));
                return Err(out);
            } else {
                unreachable!();
            }
        } else {
            outcome_into_result(res.unwrap())?;
            runtime.process_all().unwrap();
            Ok(ExternalUser {
                account_id: new_account_id,
                signer: new_signer,
            })
        }
    }

    fn new_tx(&self, runtime: &RuntimeStandalone, receiver_id: AccountId) -> Transaction {
        let nonce = runtime
            .view_access_key(&self.account_id, &self.signer.public_key())
            .unwrap()
            .nonce
            + 1;
        Transaction::new(
            self.account_id.clone(),
            self.signer.public_key(),
            receiver_id,
            nonce,
            CryptoHash::default(),
        )
    }
}

pub fn near_view<I: ToString, O: DeserializeOwned>(
    runtime: &RuntimeStandalone,
    account_id: &AccountId,
    method: &str,
    args: I,
) -> O {
    let args = args.to_string();
    let result = runtime
        .view_method_call(account_id, method, args.as_bytes())
        .unwrap()
        .0;
    let output: O = serde_json::from_reader(result.as_slice()).unwrap();
    output
}

pub fn near_call(
    runtime: &mut RuntimeStandalone,
    account: &ExternalUser,
    receiver_id: &str,
    method: &str,
    args: &[u8],
    gas: U64,
    deposit: Balance
) -> TxResult {
    let tx = account
        .new_tx(runtime, receiver_id.to_string())
        .function_call(method.into(), args.to_vec(), gas.into(), deposit)
        .sign(&account.signer);
    let res = runtime.resolve_tx(tx).unwrap();
    runtime.process_all().unwrap();
    outcome_into_result(res)
}

pub fn deploy_and_init_fungible_token(
    runtime: &mut RuntimeStandalone,
    account: &ExternalUser,
    init_method: &str,
    gas: U64,
    args: &NewFungibleTokenArgs,
) -> TxResult {
    let tx = account
        .new_tx(runtime, account.clone().account_id)
        // transfer tokens otherwise "wouldn't have enough balance to cover storage"
        .transfer(ntoy(50))
        .deploy_contract(FUNGIBLE_TOKEN_BYTES.to_vec())
        .function_call(init_method.into(), serde_json::to_vec(args).unwrap(), gas.into(), 0)
        .sign(&account.signer);
    let res = runtime.resolve_tx(tx).unwrap();
    runtime.process_all().unwrap();
    outcome_into_result(res)
}

pub fn deploy_and_init_counter(
    runtime: &mut RuntimeStandalone,
    account: &ExternalUser,
    init_method: &str,
    gas: U64,
) -> TxResult {
    let tx = account
        .new_tx(runtime, account.clone().account_id)
        .transfer(ntoy(50))
        .deploy_contract(COUNTER_BYTES.to_vec())
        .function_call(init_method.into(), vec!(), gas.into(), 0)
        .sign(&account.signer);
    let res = runtime.resolve_tx(tx).unwrap();
    runtime.process_all().unwrap();
    outcome_into_result(res)
}


pub fn deploy_simulation_example(
    runtime: &mut RuntimeStandalone,
    account: &ExternalUser,
    init_method: &str,
    gas: U64,
) -> TxResult {
    let tx = account
        .new_tx(runtime, account.clone().account_id)
        .transfer(ntoy(50))
        .deploy_contract(SIMULATION_EXAMPLE_BYTES.to_vec())
        .function_call(init_method.into(), vec!(), gas.into(), 0)
        .sign(&account.signer);
    let res = runtime.resolve_tx(tx).unwrap();
    runtime.process_all().unwrap();
    outcome_into_result(res)
}

pub fn new_root(account_id: AccountId) -> (RuntimeStandalone, ExternalUser) {
    let (runtime, signer) = init_runtime_and_signer(&account_id);
    (runtime, ExternalUser { account_id, signer })
}
