#!/usr/bin/env bash

CONTRACT_NAME=$1

if [ -z "$CONTRACT_NAME" ]; then
    echo "Usage: $0 <contract_name>"
    exit 1
fi

CONTRACT_PATH="contracts/libraries/$CONTRACT_NAME"

if [ ! -d "$CONTRACT_PATH" ]; then
    echo "Error: Contract '$CONTRACT_NAME' not found in contracts/libraries/"
    exit 1
fi

if [[ $(uname -m) =~ "arm64" ]]; then
    IMAGE="cosmwasm/optimizer-arm64:0.16.1"
else
    IMAGE="cosmwasm/optimizer:0.16.1"
fi

docker run --rm -v "$(pwd)":/workspace \
    --mount type=volume,source="${CONTRACT_NAME}_cache",target=/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    -w /workspace/"$CONTRACT_PATH" \
    "$IMAGE"

