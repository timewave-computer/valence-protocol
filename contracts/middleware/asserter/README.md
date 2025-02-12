# Valence Asserter

*Middleware Asserters* are contracts responsible for evaluating
predefined predicates about `ValenceType`s.

This component is primarily meant to be used for *conditional*
execution of Valence Library functions. This is enabled via the
Processor, where message batches can be constructed in a way that
treats the Asserter evaluation responses as an `if` statement.

## Asserter lifecycle

Asserters are singleton components that are instantiated before the
program start time. Asserter is a stateless contract, meaning that
it can be reused across multiple Valence Programs.

## Asserting predicates

Predicates can be evaluated by calling the following execute method:

```rust
Assert {
    a: AssertionValue,
    predicate: Predicate,
    b: AssertionValue,
},
```

This will perform the following actions:

1. get the assertion values `a` and `b`
2. evaluate the predicate `predicate`

Assertion Values can be configured to either be variable or constant.

If the value is variable, Asserter will try to access the specified
Storage Account and fetch the relevant fields from the given `ValenceType`.
If the value is constant, Asserter will unpack the constant.

After both `a` and `b` are in the scope, we evaluate the predicate which
returns a `bool`. Predicate result is then mapped into a `Response` as follows:

```rust
true => Ok(Response::default()),
false => Err(StdError::generic_err("assertion failed").into()),
```

Returning an error means that any batches being processed from the processor
will be aborted, while an `Ok` response will allow the processing to continue.

## API

### Execute

This contract contains a single `ExecuteMsg` variant that either returns
`Ok(())` or `Err(StdError)`:

```rust
Assert {
    a: AssertionValue,
    predicate: Predicate,
    b: AssertionValue,
},
```

Predicate supports the following values and applies them in the manner of
`a predicate b`:

```rust
pub enum Predicate {
    LT,
    LTE,
    EQ,
    GT,
    GTE,
}
```

Assertion values can be specified with the following type:

```rust
pub enum AssertionValue {
    // storage account slot query
    Variable(QueryInfo),
    // constant valence primitive value
    Constant(ValencePrimitive),
}
```

where

```rust
pub struct QueryInfo {
    // addr of the storage account
    pub storage_account: String,
    // key to access the value in the storage account
    pub storage_slot_key: String,
    // b64 encoded query
    pub query: Binary,
}

pub enum ValencePrimitive {
    Decimal(cosmwasm_std::Decimal),
    Uint64(cosmwasm_std::Uint64),
    Uint128(cosmwasm_std::Uint128),
    Uint256(cosmwasm_std::Uint256),
    String(cosmwasm_std::String),
}
```

### Queries

Query entry point is disabled for this contract.
