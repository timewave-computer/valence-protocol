optimize:
    ./devtools/optimize.sh

fmt:
    cargo fmt --all -- --check
    cargo clippy --all-targets --verbose -- -D warnings
    taplo fmt --check
