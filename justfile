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
    echo '[formatting] \
    align_entries = true \
    align_comments = true \
    array_auto_expand = true \
    array_auto_collapse = true \
    compact_arrays = false \
    compact_inline_tables = false \
    column_width = 120 \
    indent_string = \"    \" \
    reorder_keys = false' > /tmp/taplo.toml
    # cargo install taplo-cli --locked
    taplo fmt {{flag}} --config /tmp/taplo.toml
    rm /tmp/taplo.toml
