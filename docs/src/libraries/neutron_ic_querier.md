# Neutron Interchain Querier

**Neutron Interchain Querier library** allows registering and receiving results for KV-based queries.
This library wraps around the functionality enabled by the `interchainqueries` module on Neutron.

## Prerequisites

### Active Neutron ICQ relayer

This library requires active [Neutron ICQ Relayers](https://github.com/neutron-org/neutron-query-relayer) operating on the specified routes.

### Valence Middleware broker

Each KV-based query requires a correctly encoded key in order to be registered.
This library obtains the query keys from [Valence Middleware brokers](./../middleware/broker.md),
which expose particular type registries.

For a given KV-query to be performed, the underlying type registry must implement `IcqIntegration` trait
which thefore provides the following adapters:
1. `get_kv_key`, enabling the ability to get the correct `KVKey` for query registration
2. `decode_and_reconstruct`, allowing to reconstruct the interchain query result

Read more about the given type ICQ integration in the [type registry documentation page](./../middleware/type_registry.md#neutron-interchain-query-integration).

### Query registration fee

Neutron `interchainqueries` module is configured to escrow a fee (denominated in `untrn`) in order to register a query.
The fee parameter is dynamic and can be queried via the `interchainqueries` module.

### Query deregistration

Interchain Query escrow payments can be reclaimed by submitting the
`RemoveInterchainQuery` message.
Only the query owner (this contract) is able to submit this message.

Interchain Queries should be removed after they are no longer needed,
however, that moment may be different for each Valence Program depending
on its configuration.

### relevant `interchainqueries` module details

#### Query Registration Message types

Interchain queries can be registered and unregistered by submitting the following `neutron-sdk` messages:

```rust
pub enum NeutronMsg {
	// other variants

	RegisterInterchainQuery {
		/// **query_type** is a query type identifier ('tx' or 'kv' for now).
		query_type: String,

		/// **keys** is the KV-storage keys for which we want to get values from remote chain.
		keys: Vec<KVKey>,

		/// **transactions_filter** is the filter for transaction search ICQ.
		transactions_filter: String,

		/// **connection_id** is an IBC connection identifier between Neutron and remote chain.
		connection_id: String,

		/// **update_period** is used to say how often the query must be updated.
		update_period: u64,
	},
	RemoveInterchainQuery {
    query_id: u64,
	},
}
```

where the `KVKey` is defined as follows:

```rust
pub struct KVKey {
    /// **path** is a path to the storage (storage prefix) where you want to read value by key (usually name of cosmos-packages module: 'staking', 'bank', etc.)
    pub path: String,

    /// **key** is a key you want to read from the storage
    pub key: Binary,
}
```

This variant applies for both *tx*- and *kv*-based queries. Given that we are dealing with *kv*-based queries, `transactions_filter` field is irrelevant.

Therefore our query registration message may look like the following:

```rust
    let kv_registration_msg = NeutronMsg::RegisterInterchainQuery {
        query_type: QueryType::KV.into(),
        keys: vec![query_kv_key],
        transactions_filter: String::new(),
        connection_id: "connection-3".to_string(),
        update_period: 5,
    }
```

`query_kv_key` here is obtained by calling into the associated broker module for a given type and query parameters.

#### Query Result Message types

After a query is registered and fetched back to Neutron, its results can be queried with the following neutron query:

```rust
pub enum NeutronQuery {
    /// Query a result of registered interchain query on remote chain
    InterchainQueryResult {
        /// **query_id** is an ID registered interchain query
        query_id: u64,
    },
	// other types
}
```

which will return the interchain query result:

```rust
pub struct InterchainQueryResult {
    /// **kv_results** is a raw key-value pairs of query result
    pub kv_results: Vec<StorageValue>,

    /// **height** is a height of remote chain
    pub height: u64,

    #[serde(default)]
    /// **revision** is a revision of remote chain
    pub revision: u64,
}
```

where `StorageValue` is defined as:

```rust
/// Describes value in the Cosmos-SDK KV-storage on remote chain
pub struct StorageValue {
    /// **storage_prefix** is a path to the storage (storage prefix) where you want to read
    /// value by key (usually name of cosmos-packages module: 'staking', 'bank', etc.)
    pub storage_prefix: String,

    /// **key** is a key under which the **value** is stored in the storage on remote chain
    pub key: Binary,

    /// **value** is a value which is stored under the **key** in the storage on remote chain
    pub value: Binary,
}
```

## Query lifecycle

After `RegisterInterchainQuery` message is submitted, `interchainqueries` module will deduct
the query registration fee from the caller.

At that point the query is assigned its unique `query_id` identifier, which is not known in advance.
This identifier is returned to the caller in the reply.

Once the query is registered, the responsible query relayer performs the following steps:

1. fetch the specified value from the target domain
2. post the query result to `interchainqueries` module
3. trigger `SudoMsg::KVQueryResult` endpoint on the contract that registered the query

`SudoMsg::KVQueryResult` does not carry back the actual query result. Instead, it posts back
a `query_id` of the query which had been performed, announcing that its result is available.

That `query_id` can then be used to query the `interchainqueries` module to obtain the raw
interchainquery result. These raw results fetched from other cosmos chains will be encoded
in protobuf and require additional processing in order to be reasoned about.

## Library Functions

At its core, this library should support initiating the interchain queries, receiving their
responses, and reclaiming the escrowed fees by unregistering the queries.

In practice, however, these functions are not very useful in a broader Valence Program context
by themselves - remote domain *KV-Query* results (next just *V*) arrive back encoded in
formats meant for those remote domains.

For most cosmos-sdk based chains, *V* is stored in protobuf. Interpreting protobuf from
within cosmwasm context is not straightforward and requires additional steps.
Other domains may store their state in other encoding formats. We do not make any assumptions
about remote domain encodings in this library - instead, that responsibility is handed over
to the middleware.

For that reason, it is likely that this library will take on the additional responsibility of
transforming those remote-encoded responses into [canonical data formats](./../middleware/valence_types.md)
that will be easily recognized within the Valence Protocol scope.
Aforementioned transformation will be performed by making use of [Valence Middleware](./../middleware/_overview.md).

After the query response is transformed into its canonical representation, the resulting
data type is written into a [Storage Account](./../components/storage_account.md) making
it available for further processing, interpretation, or other functions.

## Library Lifecycle

With the baseline functionality in mind, there are multiple design decisions to be made
that will shape the overall lifecycle of this library.

This library may take on one of two possible routes with respect to how long it is considered
active:
1. single-use, instantiated to perform a particular query and complete
2. multi-use, instantiated to perform multiple domain queries

For single-use design, one of the main questions is at what point does the query
actually get registered.

One approach for the single-use approach could be to register the query as soon as
the library is instantiated. This way, given a complex Valence Program configuration,
IC Querier library would have the smallest chance of being the bottleneck and
potentially blocking other libraries from performing their functions (due to query
result not being fetched yet).
Alternatively, queries could be registered on-demand. That way, the library
would be instantiated just like any other library. When a query would be required,
a message would be executed and the query would get registered.

Both approaches would perform the queries, but the library would remain active.
For that reason we likely need to introduce some notion of finalization.
Registered Interchain Queries continue receiving updates according to the `update_period`
specified during the query registration. This is quite an open design space -
some potential approaches to query deregistration may involve:

- deregister the query after n query results (e.g. after 1)
- deregister the query once the result is posted on a block that exceeds a given block/time
- deregister the query manually with a specific message

## Library in Valence Programs

Neutron IC Querier does not behave as a standard library in that it does not produce
any fungible outcome. Instead, it produces a foreign type that gets converted
into a Valence Type.

While that result could be posted directly to the state of this library,
instead, it is posted to an associated output account meant for storing data.
Just as some other libraries have a notion of output accounts for transferring some
funds, Neutron IC Querier has a notion of output account for writing some data.

For example, consider a situation where this library had queried the balance of some
remote account, parsed the response into a Valence Balance type, and wrote that resulting
object into its associated output account. That output account may be the input account
of some other library, which will attempt to perform its function based on the content
written to its input account. This may involve something along the lines of:
`if balance > 0, do x; otherwise, do y;`.

It is a little less obvious as to what is the *input account* of Neutron IC Querier.
For one, the input account could be standard type account that deals with token transfers.
Prior to registering the interchain query, input account could be expected to receive
the query deposit (in `untrn`) meant to cover the query registration costs.
With that, the IC Querier flow in a Valence Program may look like this:

```
┌──────────────┐                ┌────────────┐                   ┌───────────┐
│   standard   │   pay query    │ Neutron IC │   write Valence   │  storage  │
│   account    │────deposit────▶│  Querier   │──────result──────▶│  account  │
└──────────────┘                └────────────┘                   └───────────┘
```

Input account being funded with sufficient `untrn` here could even be seen as the trigger
for when a query should be registered: input account could be gated by some logic that should
precede the query registration. It's probably worth noting that anyone would
be free to transfer funds into that account and "trigger" the query registration,
but it is hardly an attack - just an early start to a flow that was meant to happen anyways.
Prematurely registered queries could be dealt with (permissioned/permisionless deregistration),
and given the escrow cost, this type of attack does not seem very likely. Also worth
noting, this type of design would probably not be compatible with query registration
happening as soon as the library is instantiated.
