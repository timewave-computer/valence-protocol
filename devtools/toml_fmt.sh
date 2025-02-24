#!/bin/bash

toml_fmt() {
    # if no flags are provided we default to empty
    local flag="${1:-}"

    # create a temp toml config
    cat > /tmp/taplo.toml << 'EOF'
[formatting]
    align_entries = true
    align_comments = true
    array_auto_expand = true
    array_auto_collapse = true
    compact_arrays = true
    compact_inline_tables = false
    column_width = 120
    indent_string = "    "
    reorder_keys = false
EOF

    # run the formatter with provided flag (if provided)
    taplo fmt $flag --config /tmp/taplo.toml --check

    # remove the temp config
    rm /tmp/taplo.toml
}

toml_fmt "$@"
