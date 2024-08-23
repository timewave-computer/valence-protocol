#!/usr/bin/env bash

# Function to process a directory
process_directory() {
    if [ -d "$1" ] && [ -f "$1/Cargo.toml" ]; then
        echo "Processing: $1"
        cd "$1"
        # Delete old one if it exists
        rm -rf schema
        cargo schema
        rm -rf schema/raw
        cd - >/dev/null
    fi
}

# Process contracts in contracts/*/*
for d in contracts/*/*; do
    process_directory "$d"
done

# Process contracts in contracts/*
for d in contracts/*; do
    if [ -d "$d" ]; then
        process_directory "$d"
    fi
done
