optimize:
    ./devtools/optimize.sh

run-example program:
    RUST_LOG=debug cargo run --package valence-program-examples --bin {{program}}

run-e2e test:
    RUST_LOG=debug cargo run --package valence-e2e --example {{test}}

network_inspect := "docker network inspect hyperlane-net -f '{{range .Containers}}{{.Name}} {{end}}'"
disconnect-hyperlane-network:
    #!/usr/bin/env sh
    containers=$({{network_inspect}})
    for container in $containers; do
        docker network disconnect hyperlane-net $container
    done
    docker network rm hyperlane-net
