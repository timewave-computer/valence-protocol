# Local Interchain Testing

## Installing local-interchain

Before you can run the tests, you need to install Local interchain. This is a one-time operation. NOTE: your binary will link back to the location of where you install, if you remove the folder, you need to `make install` the binary again.

```bash
git clone https://github.com/strangelove-ventures/interchaintest && cd interchaintest/local-interchain && make install
```

## Running your local environment

Run one of the set-up configs we have in the `e2e/chains` folder. For example, to run the `neutron_juno.json` config, run the following command:

```bash
./scripts/start-local-ic.sh start neutron_juno --api-port 42069
```

This will start a local environment (with automatic retry mechanism as sometimes local-ic starts the http server and then crashes) with a Gaia chain, a Neutron (using ICS) chain and a Juno chain. The `--api-port` will expose the API on port 42069, we are using this port in our local-ic-utils crate so let's use the same to reuse some of the utils there.

For tests that involve EVM chains, the chains and relayer are started from the test itself so no need to start them manually.

## Build and optimize all contracts

Uses CosmWasm optimizer to optimize CosmWasm contracts and store the results in `./artifacts` and Foundry to build the contracts and store them in `./solidity/out`

```bash
just optimize

## or

./devtools/optimize.sh
```

## Running tests

Check that the `local-ic start` command created `e2e/configs/logs.json` with the correct RPC URL mappings. This configuration file is required to run the example tests.

Once you have your tests written, you can run them using the following command from the workspace directory, here I'm running the `polytone` tests that are in the `examples` folder:

```bash
cargo run --package valence-e2e --example polytone
```

## Neutron ICQ Relayer setup

For tests involving interchain queries, an additional setup step is needed
to enable the query relayer functionality.

This can be achieved by cloning the official repository and building
the docker image:

```sh
git clone git@github.com:neutron-org/neutron-query-relayer.git
cd neutron-query-relayer
make build-docker
```

## Troubleshooting

```bash
cargo run --package valence-e2e --example example_file_name`
```

```txt
Error: LocalInterchain(Custom { msg: "channel_json is not an array" })
```

The chains required in the test are not running.

```bash
cargo run --package valence-e2e --example example_file_name`
```

```txt
called `Result::unwrap()` on an `Err` value: Os { code: 2, kind: NotFound, message: "No such file or directory" }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

1. Check that `e2e/configs/logs.json` exists. If it does not, kill all local-ic processes and rerun from `e2e` directory.
2. Check that the `artifacts` folder, and that all contracts used in the example were built. Run `just optimize` if contracts are missing.
