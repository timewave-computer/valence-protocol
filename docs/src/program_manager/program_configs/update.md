# Update a program

Updating a programs allows you to:

* Change the owner of a program
* Update libraries configs
* Add/Modify/Enable/Disable authorizations

The manager will NOT perform those operations directly, rather output a list of messages that needs to be executed by the owner to achieve the updated program.

```rust
pub struct ProgramConfigUpdate {
    /// The id of a program to update
    pub id: u64,
    /// New owner, if the owner is to be updated
    pub owner: Option<String>,
    /// The list of library config updates to perform
    pub libraries: BTreeMap<Id, LibraryConfigUpdate>,
    /// A list of authorizations
    pub authorizations: Vec<AuthorizationInfoUpdate>,
}
```

## Id

The id of the program to perform the update on, the manager will look for this id in the on-chain registry and pull the current program config that exists.

## Owner

Optional field to update the owner, it takes the new owner address.

## Libraries 

A map of `library_id => library_config`.

`LibraryConfigUpdate` is an enum that includes all possible libraries and their `LibraryConfigUpdate` type

## Authorizations

A list o operations to do on the authorizations table

```rust
pub enum AuthorizationInfoUpdate {
    Add(AuthorizationInfo),
    Modify {
        label: String,
        not_before: Option<Expiration>,
        expiration: Option<Expiration>,
        max_concurrent_executions: Option<u64>,
        priority: Option<Priority>,
    },
    /// Disable by label
    Disable(String),
    /// Disable by label
    Enable(String),
}
```

### Add

Adds a new authorization with that info

### Modify

Nodifies an existing authorization with that label

### Disable

Disables an existing authorization by label

### Enable

Enable a disabled authorization by label