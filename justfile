optimize:
    ./devtools/optimize.sh

network_inspect := "docker network inspect hyperlane-net -f '{{range .Containers}}{{.Name}} {{end}}'"
disconnect-hyperlane-network:
    #!/usr/bin/env sh
    containers=$({{network_inspect}})
    for container in $containers; do
        docker network disconnect hyperlane-net $container
    done
    docker network rm hyperlane-net

run-example program:
    RUST_LOG=debug cargo run --package valence-program-examples --bin {{program}}

precommit:
    cargo fmt --all -- --check
    cargo clippy --all-targets --verbose -- -D warnings
    just toml_fmt --check
    ./devtools/schema.sh

toml_fmt flag="":
    ./devtools/toml_fmt.sh {{flag}}
