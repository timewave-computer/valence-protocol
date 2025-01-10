# Valence Types

**Valence Types** are a set of canonical type wrappers to be used inside
Valence Programs.

Primary operational domain of Valence Protocol will need to consume, interpret,
and otherwise manipulate data from external domains. For that reason, canonical
representations of such types are defined in order to form an abstraction layer
that all Valence Programs can reason about.

## Canonical Type integrations

Canonical types to be used in Valence Programs are enabled by the Valence Protocol.

For instance, consider Astroport XYK and Osmosis GAMM pool types. These are two
distinct data types that represent the same underlying concept - a constant product
pool.

These types can be unified in the Valence Protocol context by being mapped to and
from the following Valence Type definition:

```rust
pub struct ValenceXykPool {
    /// assets in the pool
    pub assets: Vec<Coin>,

    /// total amount of shares issued
    pub total_shares: String,

    /// any other fields that are unique to the external pool type
    /// being represented by this struct
    pub domain_specific_fields: BTreeMap<String, Binary>,
}
```

For a remote type to be integrated into the Valence Protocol means that there are
available adapters that map between the canonical and original type definitions.

These adapters can be implemented by following the design outlined by [*type registries*](./type_registry.md).

## Active Valence Types

Active Valence types provide the interface for integrating remote domain representations
of the same underlying concepts. Remote types can be integrated into Valence Protocol
if and only if there is an enabled Valence Type representing the same underlying primitive.

Currently enabled Valence types are:
- XYK pool
- Balance response
