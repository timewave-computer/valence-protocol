#!/bin/bash

cargo fmt --all -- --check
cargo clippy --all --all-targets --all-features -- -D warnings