#!/bin/bash
cargo +stable build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/simulation_example.wasm res/