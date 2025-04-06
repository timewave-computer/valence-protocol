# Build a program config

The manager entry points are expecting a rust type, you can use any way you are familiar with to build this type, here are some examples

## Using deployer

[Timewave Deployer](https://github.com/timewave-computer/program-deployer-template) is an easy way of building programs, you can follow the README to set the deployer.

You can view [Timewave deployments](https://github.com/timewave-computer/timewave-program-deployments) repository to see an example of already deployed programs using the deployer.

## Program builder

Our above deployer is using a rust builder to build a program, an example of this can be found in our [program template](https://github.com/timewave-computer/program-deployer-template/blob/main/programs/program_template/src/program_builder.rs)

```rust
let mut builder = ProgramConfigBuilder::new("example-program", owner.as_str());
```

`ProgramConfigBuilder::new(NAME, OWNER)` provides an easy way to add accounts, libraries and authorizations to build the program config.

## JSON file

A program config can also be parsed from a JSON file to `ProgramConfig` type.

Here is an [example](https://github.com/timewave-computer/timewave-program-deployments/blob/main/programs/2025-03-23-prod-dICS-ntrn-allocation/output/mainnet-2025-03-31_18%3A38%3A02-success/raw-program-config.json) from past deployments of a JSON file of a program config that can be provided to the manager to be intantiated.