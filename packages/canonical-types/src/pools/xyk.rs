use std::collections::BTreeMap;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Binary, Coin, Decimal, StdError, StdResult};

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

impl ValenceXykPool {
    pub fn get_price(&self) -> StdResult<Decimal> {
        ensure!(
            self.assets.len() == 2,
            StdError::generic_err("price can be calculated iff xyk pool contains exactly 2 assets")
        );

        let a = self.assets[0].amount;
        let b = self.assets[1].amount;

        Ok(Decimal::from_ratio(a, b))
    }
}

pub trait ValenceXykAdapter {
    type External;

    fn try_to_canonical(&self) -> StdResult<ValenceXykPool>;
    fn try_from_canonical(canonical: ValenceXykPool) -> StdResult<Self::External>;
}

/*
OSMOSIS POOL

pub struct Pool {
    #[prost(string, tag = "1")]
    pub address: ::prost::alloc::string::String,
    #[prost(uint64, tag = "2")]
    #[serde(alias = "ID")]
    #[serde(
        serialize_with = "crate::serde::as_str::serialize",
        deserialize_with = "crate::serde::as_str::deserialize"
    )]
    pub id: u64,
    #[prost(message, optional, tag = "3")]
    pub pool_params: ::core::option::Option<PoolParams>,
    /// This string specifies who will govern the pool in the future.
    /// Valid forms of this are:
    /// {token name},{duration}
    /// {duration}
    /// where {token name} if specified is the token which determines the
    /// governor, and if not specified is the LP token for this pool.duration is
    /// a time specified as 0w,1w,2w, etc. which specifies how long the token
    /// would need to be locked up to count in governance. 0w means no lockup.
    /// TODO: Further improve these docs
    #[prost(string, tag = "4")]
    pub future_pool_governor: ::prost::alloc::string::String,
    /// sum of all LP tokens sent out
    #[prost(message, optional, tag = "5")]
    pub total_shares: ::core::option::Option<super::super::super::cosmos::base::v1beta1::Coin>,
    /// These are assumed to be sorted by denomiation.
    /// They contain the pool asset and the information about the weight
    #[prost(message, repeated, tag = "6")]
    pub pool_assets: ::prost::alloc::vec::Vec<PoolAsset>,
    /// sum of all non-normalized pool weights
    #[prost(string, tag = "7")]
    pub total_weight: ::prost::alloc::string::String,
}
*/

/*
    ASTROPORT POOL:
    "assets": [
            {
        "info": {
            "native_token": {
            "denom": "uluna"
            }
        },
        "amount": "50487988152"
        },
        {
        "info": {
            "token": {
            "contract_addr": "terra13c7t0xtfpafcr6gae9f404x5am7euf87c9qwmsphsecamqjjujqs7yfm4m"
            }
        },
        "amount": "648612030411"
        }
    ],
    "total_share": "177335913519"
*/
