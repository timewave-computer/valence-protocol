# Instantiate program

The manager is using the program config to instantiate the full flow of the program on-chain.

After instantiation of a program, the program config will contain the instantiated data of the program.

```rust
pub struct ProgramConfig {
    pub id: u64,
    pub name: String,
    pub owner: String,
    pub links: BTreeMap<Id, Link>,
    pub accounts: BTreeMap<Id, AccountInfo>,
    pub libraries: BTreeMap<Id, LibraryInfo>,
    pub authorizations: Vec<AuthorizationInfo>,
    #[serde(default)]
    pub authorization_data: AuthorizationData,
}
```

## Id
Unique identifier of a program, it is used to save the program config on-chain.

Should be set to `0` when instantiating a new program.

## Name

A short description of the program to easily identify it.

# Links

A map of links between libraries and the connected input and output accounts.

This allows us to represent a program in a graph.

```rust
pub struct Link {
    /// List of input accounts by id
    pub input_accounts_id: Vec<Id>,
    /// List of output accounts by id
    pub output_accounts_id: Vec<Id>,
    /// The library id
    pub library_id: Id,
}
```

## Accounts 

A map of accounts that are being used by the program

```rust
pub struct AccountInfo {
    // The name of the account
    pub name: String,
    // The type of the account
    pub ty: AccountType,
    // The domain this account is on
    pub domain: Domain,
    // The instantiated address of the account
    pub addr: Option<String>,
}
```

### AccountType

```rust
pub enum AccountType {
    /// This means the account is already instantiated
    Addr { addr: String },
    /// This is our base account implementation
    Base { admin: Option<String> },
}
```