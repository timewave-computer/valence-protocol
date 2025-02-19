# Valence Storage Account

The Valence Storage Account is a type of Valence account that can store Valence Type data
objects.

Like all other accounts, Storage Accounts follow the same pattern of approving and revoking
authorized libraries from being able to post Valence Types into a given account.

While regular Valence accounts are meant for storage of fungible tokens, Valence Storage
accounts are meant for storage of non-fungible objects.

## API

### Execute Methods

Storage Account is a simple component exposing the following execute methods:

```rust
pub enum ExecuteMsg {
    // Add library to approved list (only admin)
    ApproveLibrary { library: String },
    // Remove library from approved list (only admin)
    RemoveLibrary { library: String },
    // stores the given `ValenceType` variant under storage key `key`
    StoreValenceType { key: String, variant: ValenceType },
}
```

Library approval and removal follow the same implementation as that of the fund accounts.

`StoreValenceType` is the key method of this contract. It takes in a *key* of type `String`, and its
associated value of type `ValenceType`.

If `StoreValenceType` is called by the owner or an approved library, it will persist the *key-value*
mapping in its state. Storage here works in an overriding manner, meaning that posting data
for a key that already exists will override its previous value and act as an update method.

### Query Methods

Once a given type had been posted into the storage account using `StoreValenceType` call, it becomes available
for querying.

Storage account exposes the following `QueryMsg`:

```rust
pub enum QueryMsg {
    // Get list of approved libraries
    #[returns(Vec<String>)]
    ListApprovedLibraries {},
    // Get Valence type variant from storage
    #[returns(ValenceType)]
    QueryValenceType { key: String },
}
```
