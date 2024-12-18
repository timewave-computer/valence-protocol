use std::collections::BTreeMap;
use std::str::FromStr;

use cosmwasm_std::coin;
use cosmwasm_std::to_json_binary;

use cosmwasm_std::StdError;
use osmosis_std::types::osmosis::gamm::v1beta1::{Pool, PoolParams};
use osmosis_std::types::{cosmos::base::v1beta1::Coin, osmosis::gamm::v1beta1::PoolAsset};
use valence_middleware_utils::canonical_types::pools::xyk::ValenceXykPool;
use valence_middleware_utils::canonical_types::ValenceTypeAdapter;
use valence_middleware_utils::type_registry::types::ValenceType;
use valence_middleware_utils::{try_unpack_domain_specific_value, MiddlewareError};

use super::{
    OsmosisXykPool, ADDRESS_KEY, FUTURE_POOL_GOVERNOR_KEY, ID_KEY, POOL_PARAMS_KEY,
    SHARES_DENOM_KEY, TOTAL_WEIGHT_KEY,
};

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

    fn try_from_canonical(canonical: ValenceType) -> Result<Self::External, MiddlewareError> {
        let canonical_inner = match canonical {
            ValenceType::XykPool(pool) => pool,
            _ => {
                return Err(MiddlewareError::Std(StdError::generic_err(
                    "canonical inner type mismatch",
                )))
            }
        };
        // unpack the pool address
        let address: String =
            try_unpack_domain_specific_value(ADDRESS_KEY, &canonical_inner.domain_specific_fields)?;

        // unpack the pool id
        let id: u64 =
            try_unpack_domain_specific_value(ID_KEY, &canonical_inner.domain_specific_fields)?;

        // unpack the future pool governor
        let future_pool_governor: String = try_unpack_domain_specific_value(
            FUTURE_POOL_GOVERNOR_KEY,
            &canonical_inner.domain_specific_fields,
        )?;

        // unpack the pool params
        let pool_params: Option<PoolParams> = try_unpack_domain_specific_value(
            POOL_PARAMS_KEY,
            &canonical_inner.domain_specific_fields,
        )?;

        // unpack the shares denom and total shares amount before combining them to a proto coin
        let shares_denom: String = try_unpack_domain_specific_value(
            SHARES_DENOM_KEY,
            &canonical_inner.domain_specific_fields,
        )?;
        let shares_coin = Coin {
            denom: shares_denom,
            amount: canonical_inner.total_shares,
        };

        // unpack the total weight
        let total_weight: String = try_unpack_domain_specific_value(
            TOTAL_WEIGHT_KEY,
            &canonical_inner.domain_specific_fields,
        )?;

        // unpack the pool assets
        let mut pool_assets = vec![];
        for asset in &canonical_inner.assets {
            let pool_asset = PoolAsset {
                token: Some(Coin {
                    denom: asset.denom.to_string(),
                    amount: asset.amount.into(),
                }),
                weight: try_unpack_domain_specific_value(
                    &format!("pool_asset_{}_weight", asset.denom),
                    &canonical_inner.domain_specific_fields,
                )?,
            };
            pool_assets.push(pool_asset);
        }

        Ok(Pool {
            address,
            id,
            pool_params,
            future_pool_governor,
            total_shares: Some(shares_coin),
            pool_assets,
            total_weight,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{coin, to_json_binary};
    use std::collections::BTreeMap;

    #[test]
    fn test_try_into() {
        let domain_specific_fields = BTreeMap::from([
            (ADDRESS_KEY.to_string(), to_json_binary("pool1").unwrap()),
            (ID_KEY.to_string(), to_json_binary(&1).unwrap()),
            (
                FUTURE_POOL_GOVERNOR_KEY.to_string(),
                to_json_binary("gov1").unwrap(),
            ),
            (TOTAL_WEIGHT_KEY.to_string(), to_json_binary("100").unwrap()),
            (
                "pool_asset_uatom_weight".to_string(),
                to_json_binary("120").unwrap(),
            ),
            (
                "pool_asset_uosmo_weight".to_string(),
                to_json_binary("80").unwrap(),
            ),
            (
                SHARES_DENOM_KEY.to_string(),
                to_json_binary("osmo/gamm/whatever").unwrap(),
            ),
            (
                POOL_PARAMS_KEY.to_string(),
                to_json_binary(&Some(PoolParams {
                    swap_fee: "0.003".to_string(),
                    exit_fee: "0.0".to_string(),
                    smooth_weight_change_params: None,
                }))
                .unwrap(),
            ),
            (TOTAL_WEIGHT_KEY.to_string(), to_json_binary("100").unwrap()),
        ]);

        let pool = ValenceType::XykPool(ValenceXykPool {
            assets: vec![coin(100, "uatom"), coin(200, "uosmo")],
            total_shares: "1000".to_string(),
            domain_specific_fields,
        });

        let osmosis_pool = OsmosisXykPool::try_from_canonical(pool).unwrap();

        println!("osmosis_pool: {:?}", osmosis_pool);
        assert_eq!(osmosis_pool.address, "pool1");
        assert_eq!(osmosis_pool.id, 1);
        assert_eq!(osmosis_pool.future_pool_governor, "gov1");
        assert_eq!(osmosis_pool.total_weight, "100");
        assert_eq!(osmosis_pool.pool_assets.len(), 2);
        assert_eq!(osmosis_pool.pool_assets[0].weight, "120");
        assert_eq!(osmosis_pool.pool_assets[1].weight, "80");
        let total_shares = osmosis_pool.total_shares.unwrap();
        assert_eq!(total_shares.amount, "1000");
        assert_eq!(total_shares.denom, "osmo/gamm/whatever");
        assert_eq!(osmosis_pool.pool_params.unwrap().swap_fee, "0.003");
    }

    #[test]
    fn test_try_from() {
        let pool = osmosis_std::types::osmosis::gamm::v1beta1::Pool {
            address: "pool1".to_string(),
            id: 1,
            pool_params: Some(PoolParams {
                swap_fee: "0.003".to_string(),
                exit_fee: "0.0".to_string(),
                smooth_weight_change_params: None,
            }),
            future_pool_governor: "gov1".to_string(),
            total_shares: Some(Coin {
                denom: "osmo/gamm/whatever".to_string(),
                amount: "1000".to_string(),
            }),
            pool_assets: vec![
                PoolAsset {
                    token: Some(Coin {
                        denom: "uatom".to_string(),
                        amount: "100".to_string(),
                    }),
                    weight: "120".to_string(),
                },
                PoolAsset {
                    token: Some(Coin {
                        denom: "uosmo".to_string(),
                        amount: "200".to_string(),
                    }),
                    weight: "80".to_string(),
                },
            ],
            total_weight: "100".to_string(),
        };

        let canonical_valence_xyk_pool = OsmosisXykPool(pool).try_to_canonical().unwrap();
        println!("parsed xyk pool: {:?}", canonical_valence_xyk_pool);

        let valence_xyk_pool = match canonical_valence_xyk_pool {
            ValenceType::XykPool(pool) => pool,
            _ => panic!("unexpected type"),
        };

        assert_eq!(valence_xyk_pool.assets.len(), 2);
        assert_eq!(valence_xyk_pool.assets[0], coin(100, "uatom"));
        assert_eq!(valence_xyk_pool.assets[1], coin(200, "uosmo"));
        assert_eq!(valence_xyk_pool.total_shares, "1000");
    }
}
