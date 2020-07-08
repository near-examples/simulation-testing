# Simulation tests

The purpose of this repository is to illustrate how to write simulation tests for smart contracts. This is particularly useful for decentralized apps that make use of cross-contract calls.

Typically, a smart contract will have unit tests that work differently than simulation tests. Simulation tests are not a replacement for unit tests, but can offer more extensive tests.

### Unit tests

Unit tests are often found at the end of the `src/lib.rs` file in the [NEAR examples](https://near.dev). They'll often begin with something like:

```
use super::*;
use near_sdk::MockedBlockchain;
use near_sdk::{testing_env, VMContext};

fn get_context(input: Vec<u8>, is_view: bool, signer: AccountId) -> VMContext {
    VMContext {
        current_account_id: "alice.testnet".to_string(),
        signer_account_id: signer,
…
```

These unit tests can be run from the command line with:

    cargo test -- --nocapture
    
You do *not* have to build a project before running these types of tests.
One limitation of unit tests is their inability to perform cross-contract calls.

### Simulation tests

Simulation tests are able to perform cross-contract call and are quite different than unit tests.

First, simulation tests run on the compiled WebAssembly files themselves. This mean that, unlike unit tests, projects must be built before they can be tested. (Built meaning the typical `cargo build…` command with flags. In this project there are flags in the `.cargo/config` file as well as in the `build.sh` script. See those for details.)

Second, like [integration tests in Rust](https://doc.rust-lang.org/rust-by-example/testing/integration_testing.html), simulation tests live in the `tests` folder instead of in the `src/lib.rs`. At the time of this writing, it's convention to have a `utils.rs` file to:

- Abstract some common calls
- Deploy compiled contracts using the [RuntimeStandalone](https://github.com/nearprotocol/nearcore/tree/master/runtime/runtime-standalone)

Besides the `utils.rs` file, there will be additional file(s) with tests that use it. The name of the file(s) is flexible. In this project, the tests are found in `tests/simulation_tests.rs` with the tests following the macro `#[test]`.

Simulation tests will also be run with the same `cargo test` command shown for unit tets.

### Gotcha(s)

At the time of this writing, simulation tests need to use a Rust nightly version. There's a file in this project called `rust-toolchain` which specifies a particular nightly build. Note that in the `build.sh` file, there's an extra flag for `+stable`.

This means that the simulation tests are running with Rust nightly but the project is being built by Rust stable. Please remember this distinction if copy/pasting pieces from this example.

### `tests` directory structure

```bash
├── res
│  ├── counter.wasm         ⟵ Compiled Rust Counter example
│  └── fungible_token.wasm  ⟵ Compiled Rust Fungible Token
├── simulation_tests.rs     ⟵ Contains simulation tests
└── utils.rs                ⟵ Contains helper functions for simulation tests
```

### Overview of this example

In this example, three compiled WebAssembly smart contracts are used.

1. [Counter](https://github.com/near-examples/rust-counter/blob/01-use-hashmap/contract/src/lib.rs) (simple contract that increments/decrements a number per signer account)
2. [Fungible token](https://github.com/near/near-sdk-rs/blob/master/examples/fungible-token/src/lib.rs)
3. Simulation Example (found in `src/lib.rs` of this project) that does cross-contract calls to the other two contracts.

When tests are run, mock accounts will be created, contracts deployed (from their respective `.wasm` files), contract functions called, methods returned, and logs will be printed.

#### What happens

This example is contrived, but shows some useful patterns. A counter can be incremented and when the number becomes even, a fungible token is then sent to the individual that incremented/decremented it. 

The final test in `tests/simulation_tets.rs` will deploy multiple contracts. One of them is a simple counter. First it will increment the counter directly. 

Then it will call the simulation contract (the contract source code which lives in `src/lib.rs`) to make a cross-contract call to increment the counter.

Then it will transfer from fungible tokens to contracts directly. Then it will call the simulation contract to transfer via a cross-contract call.

### Limitations

#### Cross-contract call within a callback

There's an example of a deep cross-contract call that will *not* work with simulation testing in this project. In this case, the cross-contract call is essentially "nested" in a callback. In this case, a cross contract call is made, then a callback is called, and in the callback another cross-contract call happens. This is not possible with simulation tests at the current time. There is some deliberately commented-out code in `tests/simulation_tests.rs` that shows how this might be confusing. Please search for "Cross-contract call within a callback" in that file to see.

#### Error messages early in promise execution

This project illustrates another issue when writing tests with promises for cross-contract calls. In `tests/simulation_tests.rs`, there is a cross-contract call that is intended the fail. The error is supposed to alert the user that they cannot transfer fungible tokens to themselves. Instead, we only see the error message from the callback. Put another way, it's impossible to capture the error from the previous cross-contract call.

At the time of this writing, this is a limitation of simulation tests. If a developer is writing simulation tests and can't determine the cause of preceding failure, it may be helpful to remove the callbacks one by one.

For example, in `src/lib.rs` there's a function `send_token_if_counter_even` containing this high-level cross-contract call:

    ext_fungible_token::transfer(recipient_account, U128(1), &token_account, TRANSFER_FROM_NEAR_COST, SINGLE_CALL_GAS)
    .then(ext_this_contract::post_transfer(&env::current_account_id(), 0, SINGLE_CALL_GAS))
    
The first part of this calls `transfer`, followed by a `.then` that calls `post_transfer`. If a failure occurs during `transfer` it will not be visible in the simulation tests. You may temporarily remove the second part of the above snippet (everything including and after `.then`) in order to ascertain the failure/error from the first call.

### Cross-contract styles

There are two types of cross-contract calls in NEAR:

1. [High-level cross-contract](https://examples.near.org/rust-high-level-cross-contract) calls
2. [Low-level cross-contract](https://github.com/near/near-sdk-rs/tree/master/examples/cross-contract-low-level) calls

Please visit those repositories in order to see the difference. Both are used in this project in order to be helpful to developers choosing either path.
