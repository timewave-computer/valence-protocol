# Manager config - before the manager can work

The manager is performing actions on chains included in the program, and for that the manager need to have certain information that will allow him to perform those actions.

You can read more about the [manager config here](./manager_config.md).

# Wallet

The manager requires a funded wallet to perform actions on chain, it expects the mnemonic of the wallet to be included in the **MANAGER_MNEMONIC** environment variable.

* Note - This wallet should NOT be the owner of the program, this is a helper wallet that allows the manager to execute actions on chain, it should be funded with just enough funds to perform those actions.

# How to use program manager

The program manager is a library, it can be used as dependency in any rust project.

There are 3 functions that allow you to interact with a program:

1. `init_program(&mut ProgramConfig)` - Instantiate a new program
2. `update_program(ProgramConfigUpdate)` - Update existing program
3. `migrate_program(ProgramConfigMigrate)` - Migrate existing program to a new program

## Instantiate a program

`init_program()` takes a program config to instantiate and mutate it with the instantiated program config.

Read more in [Program config](./program_configs/instantiate.md)

## Update a program

`update_program()` takes a set of instructions to update an existing program and returns a set of messages that can be executed by the owner.

This is useful to batch update library configs and authorizations.

* Note - `update_program()` returns a set of messages that are needed to perform the update, those messages must be executed by the owner of the program.

Read more in [Program config update](./program_configs/update.md)

## Migrate a program

`migrate_program()` allows the owner to "disable" an old program, and move all the funds to the new program.

This is useful when you want to disable an old program and move the funds to a new program.

* Note - `migrate_program()` returns a set of messages to move the funds and pause the program that must be executed by the owner.

Read more in [Program config migrate](./program_configs/migrate.md)