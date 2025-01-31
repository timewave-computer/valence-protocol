# Neutron IC-querier library

**Neutron IC Querier library** allows registering and receiving results for KV-based queries.
This library is effectively a wrapper around the `interchainqueries` module on Neutron.

### Prerequisites

This library is coupled with some other libraries and assumes functioning infrastructure (ICQ-relayer).

To learn more about its prerequisites, see [its documentation entry](https://docs.valence.zone/libraries/neutron_ic_querier.html#prerequisites).

## Library configuration

Neutron IC Querier is configured with the following parameters:

### `storage_account: LibraryAccountType`

Storage Account is where the performed query results are going to be written.

The provided account must grant this library the authorization to execute the
`StoreValenceType` method.

### `querier_config: QuerierConfig`

Querier config defines the global configuration parameters that are meant to
apply to all queries that are performed during the lifecycle of this library.

These parameters are:

- `broker_addr: String`, which specifies the address of the middleware broker
that will be responsible for type conversions relating to the queries being performed
- `connection_id: String`, which specifies the IBC connection id between Neutron
and the target domain which is to be queried

### `query_definitions: BTreeMap<String, QueryDefinition>`

Query definitions contain the information needed to carry out the entire flow
relating to any particular interchain query.

This parameter is passed as a map because we need a unique identifier for each
query to be performed. The key is therefore a unique (yet arbitrary) string meant
for internal identification of each query.

Each aforementioned key is associated with the actual query definition which
consists of the following parameters:

- `registry_version: Option<String>`, giving the option to pass the middleware
broker a particular type registry version. If it's `None`, the broker will default
to the latest type registry it is aware of.
- `type_url: String`, specifying the type url of the query being performed.
note that this type url is not necessarily the actual type url used by the remote
domain that is being queried; instead, it is the type url recognized by the
associated type registry. while they do not **have** to match, most of the times
they should match (unless there is a good reason for them not to).
- `update_period: Uint64`, specifying how often the given query should be updated
- `params: BTreeMap<String, Binary>`, providing the type registry with the base64
encoded query parameters that are unique to this query
- `query_id: Option<u64>`, optionally storing the assigned query_id after the
query is registered. on instantiation this field is validated to be `None`, as
it can only be modified via execute methods.

## Execution flow

After the Neutron IC Querier had been instantiated, it is ready to start carrying
out the configured queries.

Each interchain query lifecycle is bound by two events - its registration and its
removal. This library attempts to remain as agnostic as possible to enable those
two events with its exposed functions:

### `RegisterKvQuery { target_query: String }`

Each query that was passed in along with the `query_definitions` parameter can be
registered by calling `RegisterKvQuery` method and passing in the same string
that was used as its associated key in the `query_definitions` map.

This function call expects the message sender to cover the ICQ registration fees
imposed by the `interchainqueries` module. At the moment this fee sits at `1000000untrn`,
however this parameter is configured by Neutron governance and is therefore a subject
to change. This library makes no assumptions about what the fee is by querying
it on demand, so make sure that this message call has the necessary funds.

After the validations are complete, this function submits the KV-query registration
submessage. This submessage will return us the real `query_id` that the query was
assigned by the `interchainqueries` module. This `query_id` is crucial for the
query removal (more on that in the next section), so in the same callback this
library associates its internal `target_query: String` identifier that is known
prior to the query registration with the newly assigned `query_id`.

At this point the query is published in the `interchainqueries` module and if
everything is functioning as expected, the results posted back should be processed
into canonical `ValenceType`s and then written to the associated storage account.

### `DeregisterKvQuery { target_query: String }`

In order to conclude a given query, authorized addresses can call the `DeregisterKvQuery`
method with the query identifier.

This will in turn perform two functions:

1. submit the `RemoveInterchainQuery` message to the `interchainqueries` module
2. transfer the recovered escrow deposit to the caller

After a successful query removal this library will stop receiving ICQ-result
posting callbacks and therefore stop writing any results (relating to this particular
query) to its associated storage account, concluding the query lifecycle.
