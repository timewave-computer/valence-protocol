use osmosis_std::types::osmosis::gamm::v1beta1::Pool;

pub mod domain_adapter;
pub mod valence_adapter;

const ADDRESS_KEY: &str = "address";
const ID_KEY: &str = "id";
const FUTURE_POOL_GOVERNOR_KEY: &str = "future_pool_governor";
const TOTAL_WEIGHT_KEY: &str = "total_weight";
const SHARES_DENOM_KEY: &str = "shares_denom";
const POOL_PARAMS_KEY: &str = "pool_params";
const STORAGE_PREFIX: &str = "gamm";

pub struct OsmosisXykPool(pub Pool);

#[cfg(test)]
mod tests {
    use cosmwasm_std::{from_json, Binary};
    use neutron_sdk::bindings::types::{InterchainQueryResult, StorageValue};
    use osmosis_std::types::osmosis::gamm::v1beta1::Pool;
    use valence_middleware_utils::{
        canonical_types::ValenceTypeAdapter, type_registry::types::ValenceType, IcqIntegration,
    };

    use crate::definitions::gamm_pool::{OsmosisXykPool, STORAGE_PREFIX};

    #[test]
    fn e2e() {
        let b64_key = "AgAAAAAAAAAB";
        let binary_key = Binary::from_base64(b64_key).unwrap();

        let b64_value = "Chovb3Ntb3Npcy5nYW1tLnYxYmV0YTEuUG9vbBKGAgo/b3NtbzE5ZTJtZjdjeXdrdjd6YXVnNm5rNWY4N2QwN2Z4cmRncmxhZHZ5bWgyZ3d2NWNydm0zdm5zdWV3aGg3EAEaBgoBMBIBMCIEMTI4aCokCgtnYW1tL3Bvb2wvMRIVMTAwMDAwMDAwMDAwMDAwMDAwMDAwMl8KUQpEaWJjLzRFNDFFRDhGM0RDQUVBMTVGNEQ2QURDNkVERDdDMDRBNjc2MTYwNzM1Qzk3MTBCOTA0QjdCRjUzNTI1QjU2RDYSCTEwMDAwMDAwMBIKMTA3Mzc0MTgyNDIgChIKBXVvc21vEgkxMDAwMDAwMDASCjEwNzM3NDE4MjQ6CjIxNDc0ODM2NDg=";
        let binary_value = Binary::from_base64(b64_value).unwrap();

        let storage_value = StorageValue {
            storage_prefix: STORAGE_PREFIX.to_string(),
            key: binary_key,
            value: binary_value,
        };

        // first we simulate the icq result reconstruction of b64(proto) -> type -> b64(type)
        let osmo_pool_binary = OsmosisXykPool::decode_and_reconstruct(
            Pool::TYPE_URL.to_string(),
            InterchainQueryResult {
                kv_results: vec![storage_value],
                height: 1,
                revision: 1,
            },
        )
        .unwrap();

        // unpack the binary into a type
        let osmo_pool: Pool = from_json(osmo_pool_binary).unwrap();

        // parse the external type into a valence type

        let canonical_valence_pool = OsmosisXykPool(osmo_pool).try_to_canonical().unwrap();

        let mut valence_pool = match canonical_valence_pool.clone() {
            ValenceType::XykPool(pool) => pool,
            _ => panic!("unexpected type"),
        };

        // simulate modifying the pool instance
        valence_pool.assets.push(cosmwasm_std::coin(100, "batom"));
        valence_pool.domain_specific_fields.insert(
            "pool_asset_batom_weight".to_string(),
            cosmwasm_std::to_json_binary(&"120").unwrap(),
        );

        // convert the valence type back into the external type
        let osmo_pool = OsmosisXykPool::try_from_canonical(canonical_valence_pool).unwrap();

        assert_eq!(osmo_pool.pool_assets.len(), 3);
    }
}
