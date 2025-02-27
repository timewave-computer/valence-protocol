#!/usr/bin/env bash

if [[ $(uname -m) =~ "arm64" ]]; then
    docker run --rm -v "$(pwd)":/code \
        --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
        --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
        cosmwasm/optimizer-arm64:0.16.1

else
    docker run --rm -v "$(pwd)":/code \
        --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
        --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
        cosmwasm/optimizer:0.16.1
fi

docker run --rm -it \
    -v "$(pwd)/solidity":/solidity \
    -w /solidity \
    --entrypoint sh \
    ghcr.io/foundry-rs/foundry:stable -c "forge soldeer install && forge build"
