optimize:
    ./devtools/optimize.sh

run-example program:
    RUST_LOG=debug cargo run --package valence-program-examples --bin {{program}}

run-e2e test:
    RUST_LOG=debug cargo test --package valence-e2e --test {{test}}
