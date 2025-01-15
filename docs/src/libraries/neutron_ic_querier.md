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
