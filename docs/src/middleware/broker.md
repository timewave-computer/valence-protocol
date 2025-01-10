# Middleware Broker

Middleware broker acts as an app-level integration gateway in Valence Programs.
*Integration* is used here rather ambiguously on purpose - brokers should remain
agnostic to the primitives being integrated into Valence Protocol. These primitives
may involve but not be limited to:

- data types
- functions
- encoding schemes
- any other distributed system building blocks that may be implemented differently

### Problem statement

Valence Programs can be configured to span over multiple domains and last for an
indefinite duration of time.

Domains integrated into Valence Protocol are sovereign and evolve on their own.

Middleware brokers provide the means to live with these differences by enabling
various primitive conversions to be as seamless as possible. Seamless here primarily
refers to causing **no downtime** to bring a given primitive up-to-date, and
making the process of doing so as **easy as possible** for the developers.

To visualize a rather complex instance of this problem, consider the following
situation. A Valence Program is initialized to continuously query a particular
type from a remote domain, modify some of its values, and send the altered object
back to the remote domain for further actions.
At some point during the runtime, remote domain performs an upgrade which extends
the given type with additional fields. The Valence Program is unaware of this
upgrade and continues with its order of operations. However, the type in question
from the perspective of the Valence Program had drifted and is no longer
representative of its origin domain.

Among other things, Middleware brokers should enable such programs to gracefully
recover into a synchronized state that can continue operating in a correct manner.

## Broker Lifecycle

Brokers are singleton components that are instantiated before the program start
time.

Valence Programs refer to their brokers of choice by their respective addresses.

This means that the same broker instance for a particular domain could be used
across many Valence Programs.

Brokers maintain their set of [*type registries*](./type_registry.md) and index
them by `semver`. New type registries can be added to the broker during runtime.
While programs have the freedom to select a particular version of a type registry
to be used for a given request, by default, the most up to date type registry is used.

Two aforementioned properties reduce the amount of work needed to upkeep the integrations
across active Valence Programs: updating one broker with the latest version of a
given domain will immediately become available for all Valence Programs using it.

## API

Broker interface is agnostic to the type registries it indexes. A single query is
exposed:

```rust
pub struct QueryMsg {
    pub registry_version: Option<String>,
    pub query: RegistryQueryMsg,
}
```

This query message should only change in situations where it may become limiting.

After receiving the query request, broker will relay the contained `RegistryQueryMsg`
to the correct type registry, and return the result to the caller.
