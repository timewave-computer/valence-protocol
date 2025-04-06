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

A list of accounts that are being used by the program

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

### Name

Identifying name for this account

### AccountType

Account type allows the manager to know whether the account should be instantiated or not, and what type of account we should instantiate.

```rust
pub enum AccountType {
    /// Existing address on chain
    Addr { addr: String },
    /// This is our base account implementation
    Base { admin: Option<String> },
}
```

### Domain

On what domain the account exists or should be instantiated on.

### Addr

This field will be set by the manager once the account is intantiated.

## Libraries

A list of libraries that are being used by the program.

```rust
pub struct LibraryInfo {
    pub name: String,
    pub domain: Domain,
    pub config: LibraryConfig,
    pub addr: Option<String>,
}
```

### Name

The identifying name of this specific library

### Domain

The specific domain this library is on.

### Config

The library specific config that will be used during instantiation.

`LibraryConfig` is an enum of libraries that currently exist and can be used in programs.

### Addr

This will include the address of the library contract once instantiated

## Authorizations

This is a list of all authorizations that should be included in the authorization contract.

## Authorization data

This field includes all the data regarding authorization contract and processors on all chains.

```rust
pub struct AuthorizationData {
    /// authorization contract address on neutron
    pub authorization_addr: String,
    /// List of processor addresses by domain
    /// Key: domain name | Value: processor address
    pub processor_addrs: BTreeMap<String, String>,
    /// List of authorization bridge addresses by domain
    /// The addresses are on the specified domain
    /// Key: domain name | Value: authorization bridge address on that domain
    pub authorization_bridge_addrs: BTreeMap<String, String>,
    /// List of processor bridge addresses by domain on neutron chain
    pub processor_bridge_addrs: Vec<String>,
}
```

- `authorization_addr` - Authorization contract address on neutron
- `processor_addrs` - Map of all processors by domain
- `authorization_bridge_addrs` - Bridge account address of the authorization contract on all chains
- `processor_bridge_addrs` - List of bridge accounts of processors on neutron chain