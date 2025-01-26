# Middleware Type Registry

Middleware type registries are static components that define how primitives
external to the Valence Protocol are adapted to be used within Valence programs.

While type registries can be used independently, they are typically meant to be
registered into and used via [*brokers*](./broker.md) to ensure versioning is
kept up to date.

## Type Registry lifecycle

Type Registries are static contracts that define their primitives during compile time.

Once a registry is deployed, it is expected to remain unchanged.
If a type change is needed, a new registry should be compiled, deployed,
and registered into the broker to offer the missing or updated functionality.

## API

All type registry instances must implement the same interface defined in middleware-utils.

Type registries function in a read-only manner - all of their functionality is exposed
with the `RegistryQueryMsg`. Currently, the following primitive conversions are enabled:

```rust
pub enum RegistryQueryMsg {
    /// serialize a message to binary
    #[returns(NativeTypeWrapper)]
    FromCanonical { obj: ValenceType },
    /// deserialize a message from binary/bytes
    #[returns(Binary)]
    ToCanonical { type_url: String, binary: Binary },

    /// get the kvkey used for registering an interchain query
    #[returns(KVKey)]
    KVKey {
        type_id: String,
        params: BTreeMap<String, Binary>,
    },

    #[returns(NativeTypeWrapper)]
    ReconstructProto {
        type_id: String,
        icq_result: InterchainQueryResult,
    },
}
```

`RegistryQueryMsg` can be seen as the *superset* of all primitives that Valence Programs
can expect. No particular type being integrated into the system is required to implement
all available functionality, although that is possible.

To maintain a unified interface across all type registries, they have to adhere to the same
API as all other type registries. This means that if a particular type is enabled in a type
registry and only provides the means to perform native <-> canonical conversion, attempting
to call `ReconstructProto` on that type will return an error stating that reconstructing
protobuf for this type is not enabled.

## Module organization

Primitives defined in type registries should be outlined in a domain-driven manner.
Types, encodings, and any other functionality should be grouped by their domain and
are expected to be self-contained, not leaking into other primitives.

For instance, an osmosis type registry is expected to contain all registry instances related to
the Osmosis domain. Different registry instances should be versioned by `semver`, following that
of the external domain of which the primitives are being integrated.

## Enabled primitives

Currently, the following type registry primitives are enabled:

- Neutron Interchain Query types:
  - reconstructing native types from protobuf
  - obtaining the `KVKey` used to initiate the query for a given type
- Valence Canonical Types:
  - reconstructing native types from Valence Types
  - mapping native types into Valence Types

## Example integration

For an example, consider the integration of the osmosis gamm pool.

### Neutron Interchain Query integration

Neutron Interchain Query integration for a given type is achieved by implementing
the `IcqIntegration` trait:

```rust
pub trait IcqIntegration {
    fn get_kv_key(params: BTreeMap<String, Binary>) -> Result<KVKey, MiddlewareError>;
    fn decode_and_reconstruct(
        query_id: String,
        icq_result: InterchainQueryResult,
    ) -> Result<Binary, MiddlewareError>;
}
```

#### `get_kv_key`

Implementing the `get_kv_key` will provide the means to obtain the `KVKey` needed
to register the interchain query. For osmosis gamm pool, the implementation may look
like this:

```rust

impl IcqIntegration for OsmosisXykPool {
    fn get_kv_key(params: BTreeMap<String, Binary>) -> Result<KVKey, MiddlewareError> {
        let pool_prefix_key: u8 = 0x02;

        let id: u64 = try_unpack_domain_specific_value("pool_id", &params)?;

        let mut pool_access_key = vec![pool_prefix_key];
        pool_access_key.extend_from_slice(&id.to_be_bytes());

        Ok(KVKey {
            path: STORAGE_PREFIX.to_string(),
            key: Binary::new(pool_access_key),
        })
    }
}
```

#### `decode_and_reconstruct`

Other part of enabling interchain queries is the implementation of `decode_and_reconstruct`.
This method will be called upon ICQ relayer posting the query result back to the `interchainqueries`
module on Neutron. For osmosis gamm pool, the implementation may look
like this:

```rust
impl IcqIntegration for OsmosisXykPool {
    fn decode_and_reconstruct(
        _query_id: String,
        icq_result: InterchainQueryResult,
    ) -> Result<Binary, MiddlewareError> {
        let any_msg: Any = Any::decode(icq_result.kv_results[0].value.as_slice())
            .map_err(|e| MiddlewareError::DecodeError(e.to_string()))?;

        let osmo_pool: Pool = any_msg
            .try_into()
            .map_err(|_| StdError::generic_err("failed to parse into pool"))?;

        to_json_binary(&osmo_pool)
            .map_err(StdError::from)
            .map_err(MiddlewareError::Std)
    }
}
```

### Valence Type integration

Valence Type integration for a given type is achieved by implementing
the `ValenceTypeAdapter` trait:

```rust
pub trait ValenceTypeAdapter {
    type External;

    fn try_to_canonical(&self) -> Result<ValenceType, MiddlewareError>;
    fn try_from_canonical(canonical: ValenceType) -> Result<Self::External, MiddlewareError>;
}
```

Ideally, Valence Types should represent the minimal amount of information needed and
avoid any domain-specific logic or identifiers. In practice, this is a hard problem:
native types that are mapped into Valence types may need to be sent back to the remote
domains. For that reason, we cannot afford leaking any domain-specific fields and instead
store them in the Valence Type itself for later reconstruction.

In case of `ValenceXykPool`, this storage is kept in its `domain_specific_fields` field.
Any fields that are logically common across all possible integrations into this type
should be kept in their dedicated fields. In the case of constant product pools, such
fields are the assets in the pool, and the shares issued that represent those assets:

```rust
#[cw_serde]
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

#### `try_to_canonical`

Implementing the `try_from_canonical` will provide the means of mapping a native remote type
into the canonical Valence Type to be used in Valence Protocol.
For osmosis gamm pool, the implementation may look like this:

```rust
impl ValenceTypeAdapter for OsmosisXykPool {
    type External = Pool;

    fn try_to_canonical(&self) -> Result<ValenceType, MiddlewareError> {
        // pack all the domain-specific fields
        let mut domain_specific_fields = BTreeMap::from([
            (ADDRESS_KEY.to_string(), to_json_binary(&self.0.address)?),
            (ID_KEY.to_string(), to_json_binary(&self.0.id)?),
            (
                FUTURE_POOL_GOVERNOR_KEY.to_string(),
                to_json_binary(&self.0.future_pool_governor)?,
            ),
            (
                TOTAL_WEIGHT_KEY.to_string(),
                to_json_binary(&self.0.total_weight)?,
            ),
            (
                POOL_PARAMS_KEY.to_string(),
                to_json_binary(&self.0.pool_params)?,
            ),
        ]);

        if let Some(shares) = &self.0.total_shares {
            domain_specific_fields
                .insert(SHARES_DENOM_KEY.to_string(), to_json_binary(&shares.denom)?);
        }

        for asset in &self.0.pool_assets {
            if let Some(token) = &asset.token {
                domain_specific_fields.insert(
                    format!("pool_asset_{}_weight", token.denom),
                    to_json_binary(&asset.weight)?,
                );
            }
        }

        let mut assets = vec![];
        for asset in &self.0.pool_assets {
            if let Some(t) = &asset.token {
                assets.push(coin(u128::from_str(&t.amount)?, t.denom.to_string()));
            }
        }

        let total_shares = self
            .0
            .total_shares
            .clone()
            .map(|shares| shares.amount)
            .unwrap_or_default();

        Ok(ValenceType::XykPool(ValenceXykPool {
            assets,
            total_shares,
            domain_specific_fields,
        }))
    }
}
```

#### `try_from_canonical`

Other part of enabling Valence Type integration is the implementation of `try_from_canonical`.
This method will be called when converting from canonical back to the native version of the types.
For osmosis gamm pool, the implementation may look like this:

```rust
impl ValenceTypeAdapter for OsmosisXykPool {
    type External = Pool;

    fn try_from_canonical(canonical: ValenceType) -> Result<Self::External, MiddlewareError> {
        let inner = match canonical {
            ValenceType::XykPool(pool) => pool,
            _ => {
                return Err(MiddlewareError::CanonicalConversionError(
                    "canonical inner type mismatch".to_string(),
                ))
            }
        };
        // unpack domain specific fields from inner type
        let address: String = inner.get_domain_specific_field(ADDRESS_KEY)?;
        let id: u64 = inner.get_domain_specific_field(ID_KEY)?;
        let future_pool_governor: String =
            inner.get_domain_specific_field(FUTURE_POOL_GOVERNOR_KEY)?;
        let pool_params: Option<PoolParams> = inner.get_domain_specific_field(POOL_PARAMS_KEY)?;
        let shares_denom: String = inner.get_domain_specific_field(SHARES_DENOM_KEY)?;
        let total_weight: String = inner.get_domain_specific_field(TOTAL_WEIGHT_KEY)?;

        // unpack the pool assets
        let mut pool_assets = vec![];
        for asset in &inner.assets {
            let pool_asset = PoolAsset {
                token: Some(Coin {
                    denom: asset.denom.to_string(),
                    amount: asset.amount.into(),
                }),
                weight: inner
                    .get_domain_specific_field(&format!("pool_asset_{}_weight", asset.denom))?,
            };
            pool_assets.push(pool_asset);
        }

        Ok(Pool {
            address,
            id,
            pool_params,
            future_pool_governor,
            total_shares: Some(Coin {
                denom: shares_denom,
                amount: inner.total_shares,
            }),
            pool_assets,
            total_weight,
        })
    }
}
```
