# Migrate a program

Migrating a program allows you to pause an existing program and perform funds transfer from accounts that hold funds in an old program to accounts in the new program.

Like updating a program, the manager will not perform those actions but will output a set of instructions to be executed by the owner.

Unlike the update, migration requires 2 sets of actions:

1. Transfer all funds from the old program to the new program
2. Pause the old program processors

Pausing the program will not allow any actions to be done on the old program including transferring the funds, for this reason, we first transfer all the funds, and only then pausing the old program.

```rust
pub struct ProgramConfigMigrate {
    pub old_id: Id,
    /// The new program we instantiate
    pub new_program: ProgramConfig,
    /// Transfer funds details
    pub transfer_funds: Vec<FundsTransfer>,
}
```

## Old id

This is the id of the old program

## New program

This is the config of the new program to instantiate

## Transfer funds

A list of transfers to perform

```rust
pub struct FundsTransfer {
    pub from: String,
    pub to: LibraryAccountType,
    pub domain: Domain,
    pub funds: Coin,
}
```

- `from` - From what adress to move funds to, must be an account owned by the old program
- `to` - A `LibraryAccountType` can either be set as an address, or an account id of an account in the new program
- `domain` - On what domain to perform this transfer, both `from` and `to` must be on that domain
- `funds` - The amount of funds to transfer