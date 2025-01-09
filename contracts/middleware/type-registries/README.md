# Middleware Type Registries

*Middleware type registries* are static components that define how
types external to the Valence Protocol are adapted to be used within
Valence programs.

While type registries can be used independently, they are typically
meant to be registered into a *broker* which will orchestrate different
type registries. With that in mind, it is important to ensure that
every type registry adheres to the standards expected by the broker.

## Type Registry lifecycle

Type Registries are static contracts that define their types during
compile time.

Once a registry is deployed, it is expected to remain unchanged.
If a type change is needed, a new registry should be compiled, deployed,
and registered into the broker to offer the missing or updated
functionality.

## API

All type registry instances must implement the same interface defined
in `middleware-utils`.

With that, the only thing that changes between registries is the
contents of `/definitions` directory.

## Module organization

Under `/type-registries`, organisation is outlined in a domain-driven
manner. Types are grouped by their domain and are expected to be
self-contained.

For instance, `/type-registries/osmosis` is expected to contain all
registry instances related to the Osmosis domain. Different instances
should be versioned by semver which follows the external domain versioning.
