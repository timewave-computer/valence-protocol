# Middleware Brokers

*Middleware brokers* are the gateway to using the Valence Type
system.

Brokers are responsible for offering a unified interface for
domain-specific type access.

Given that, every broker is a singleton component responsible
for unifying a particular set of types.

Because a broker offers a single interface for all types, programs
relying on those types only need to know about the broker in order
to access them.

## Broker lifecycle

Brokers are singleton components that are instantiated before the
program start time. For that reason, it is important to ensure that
brokers are up to date and remains to be valid throughout the program
lifecycle.

This up-to-date property is achieved by enabling brokers to be updated
during runtime: once the broker is instantiated, it can be updated by
adding new registries.

## Adding new registries

Registries can be added by calling the following execute method:
```rust
AddRegistry {
    // semver string
    version: String,
    // address of the registry managing the types
    address: String
}
```

This will store the latest registry information in the broker state,
allowing it to grow dynamically.

Registries are indexed by their associated versions. Registering the
same `semver` key will overwrite the previous registry.

## API

Broker API is neutral and must remain compatible with all type registries.

This is done by wrapping around the registry api defined in `middleware-utils`.
