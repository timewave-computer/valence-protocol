# Neutron IC-querier library

**Neutron IC Querier library** allows registering and receiving results for KV-based queries.
This library is effectively a wrapper around the `interchainqueries` module on Neutron.

## Prerequisites

### Active Neutron ICQ relayer

This library requires active [neutron icq relayers](https://github.com/neutron-org/neutron-query-relayer) operating on the specified routes.

### Valence Middleware broker

Each KV-based query requires a correctly encoded key in order to be registered.
This library obtains the query keys from [Valence Middleware brokers](../../middleware/README.md).
For a given KV-query to be performed, the select broker must have that type enabled.

## Flow

Queries can be registered by submitting the following message:

```rust
RegisterKvQuery {
    broker_addr: String,
    registry_version: Option<String>,
    type_id: String,
    connection_id: String,
    update_period: Uint64,
    params: BTreeMap<String, Binary>,
},
```

After a query result is fetched, sudo `SudoMsg::KVQueryResult` is triggered.
That will, in turn, decode the response and parse it into the respective `ValenceType`,
object of which is then encoded as a json binary and stored in storage.
