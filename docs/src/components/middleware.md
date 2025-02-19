# WIP: Middleware

The Valence Middleware is a set of components that provide a unified interface for the Valence Type system.

At its core, middleware is made up from the following components.

## Design goals

TODO: describe modifiable middleware, design goals and philosophy behind it

These means are achieved with three key components:

- brokers
- type registries
- Valence types

## Middleware Brokers

Middleware brokers are responsible for managing the lifecycle of middleware instances and their associated types.

## Middleware Type Registries

Middleware Type Registries are responsible for unifying a set of foreign types to be used in Valence Programs.

## Valence Types

Valence Types are the canonical representations of various external domain implementations of some types.

## Valence Asserter

Valence Asserter enables Valence Programs to assert specific predicates during runtime. This is useful for programs that wish to enable conditional execution of a given function as long as some predicate evaluates to `true`.
