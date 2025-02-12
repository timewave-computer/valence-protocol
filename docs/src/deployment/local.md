# Local interchain deployment

In order to test a program locally, we use the local interchaintest suite to spin up chains.

## 1. Installing local-interchain

Before you can run the tests, you need to install local interchain. This is a one-time operation. NOTE: your binary will link back to the location where you install, if you remove the folder, you need to run `make install` again.

```bash
git clone https://github.com/strangelove-ventures/interchaintest && cd interchaintest/local-interchain && make install
```

## 2. Running your local environment

Run one of the set-up configs we have in the `local-interchaintest/chains` folder. For example, to run the `neutron.json` config, run the following command inside the `local-interchaintest` folder:

```bash
local-ic start neutron --api-port 42069
```

This will start a local environment with a Gaia chain and a Neutron (using ICS) chain. The `--api-port` will expose the API on port 42069, we are using this port in our local-ic-utils crate so let's use the same to reuse some of the utils there.

## 3. Optimize Contracts

Use CosmWasm optimizer to optimize contracts and store the results in `./artifacts`

```bash
just optimize
```

Or

```bash
./devtools/optimize.sh
```

## 4. Generate manager config

Before we can deploy a program using the manager, we need to generate a manager config.

```bash
cargo run -p generate_local_ic_config
```

The default chain config that is used in this script is the `neutron.json` config, if in step 2 you started local-ic with a different chain config, please use the same config here.

Example with `neutron_juno.json` chain config:

```bash
cargo run -p generate_local_ic_config -- -c neutron_juno
```

## 5. Deploy a program

To deploy a program, you can use the `deploy_program` script. In (deploy_program)[deployment/scripts/deploy_program/src/main.rs] you will find an example to a built program using our program builder, you can customize this to build your own program.

After you customize your program, you can deploy it using the following command:

```bash
cargo run -p deploy_program
```

By default this will deploy a program to the local enviroment, you can change this by passing the `-c` argument with the wanted enviroment, for example:

```bash
cargo run -p deploy_program -- -c mainnet
```

Options are: `local`, `mainnet` and `testnet`.

## 6. Program Instantiated

After a program was instantiated successfully, you will see a success message in the console and the program config file path that was generated.

The name of the file will end with the program id, for example: `program_1.json`.

You will be able to find this file under the `deployment/results` folder.