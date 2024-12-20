# Middleware utils

Middleware utils package defines the unified types and interfaces that
are used meant to be used by middleware contracts.

There are two main types of such definitions:
- canonical types
- middleware API

## Canonical Types

Types declared in `./src/canonical_types` package are ones ready to be
used by Valence Programs.

## Type Registry

Type registry declarations define the common API used by the middleware.
All instances of type registries are expected to use those declarations
in order to be compatible with Valence Protocol.
