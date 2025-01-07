# Local Interchain Testing

## Installing local-interchain

Before you can run the tests, you need to install Local interchain. This is a one-time operation. NOTE: your binary will link back to the location of where you install, if you remove the folder, you need to `make install` the binary again.

```bash
git clone https://github.com/strangelove-ventures/interchaintest && cd interchaintest/local-interchain && make install
```

## Running your local environment

Run one of the set-up configs we have in the `local-interchaintest/chains` folder. For example, to run the `neutron_juno.json` config, run the following command inside the `local-interchaintest` folder:

```bash
local-ic start neutron_juno --api-port 42069
```

This will start a local environment with a Gaia chain, a Neutron (using ICS) chain and a Juno chain. The `--api-port` will expose the API on port 42069, we are using this port in our local-ic-utils crate so let's use the same to reuse some of the utils there.

To spin up a local Ethereum testnet node, we are using Anvil. If you want to run the tests that involve Ethereum, you need to start the Anvil node. You can do this by running the following command:

```bash
docker run --name anvil -e ANVIL_IP_ADDR=0.0.0.0 -p 8545:8545 ghcr.io/foundry-rs/foundry:latest anvil
```

## Running tests

Once you have your tests written, you can run them using the following command from the workspace directory, here I'm running the `polytone` tests that are in the `examples` folder:

```bash
cargo run --package local-interchaintest --example polytone
```
