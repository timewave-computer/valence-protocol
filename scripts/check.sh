#!/bin/bash
set -euo pipefail

cargo fmt --all -- --check
cargo clippy --all-targets --verbose -- -D warnings