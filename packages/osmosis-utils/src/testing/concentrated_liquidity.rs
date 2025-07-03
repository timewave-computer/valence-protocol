use cosmwasm_std::{Coin, StdResult, Uint64};
use osmosis_test_tube::{
    osmosis_std::types::{
        cosmos::params::v1beta1::{ParamChange, ParameterChangeProposal},
        osmosis::concentratedliquidity::{
            poolmodel::concentrated::v1beta1::{
                MsgCreateConcentratedPool, MsgCreateConcentratedPoolResponse,
            },
            v1beta1::MsgCreatePosition,
        },
    },
    Account, ConcentratedLiquidity, GovWithAppAccess, Module, OsmosisTestApp, SigningAccount,
};

use crate::suite::{OsmosisTestPoolConfig, OSMO_DENOM, TEST_DENOM};

pub struct ConcentratedLiquidityPool {
    pub pool_id: Uint64,
    pub pool_asset_1: String,
    pub pool_asset_2: String,
}

impl OsmosisTestPoolConfig for ConcentratedLiquidityPool {
    fn get_provider_contract_name() -> String {
        "valence_osmosis_cl_lper.wasm".to_string()
    }

    fn get_withdrawer_contract_name() -> String {
        "valence_osmosis_cl_withdrawer.wasm".to_string()
    }

    fn pool_id(&self) -> Uint64 {
        self.pool_id
    }
    fn pool_asset_1(&self) -> String {
        self.pool_asset_1.clone()
    }
    fn pool_asset_2(&self) -> String {
        self.pool_asset_2.clone()
    }

    fn setup_pool(app: &OsmosisTestApp, creator: &SigningAccount) -> StdResult<Self> {
        let gov_mod = GovWithAppAccess::new(app);
        gov_mod
            .propose_and_execute(
                "/cosmos.params.v1beta1.ParameterChangeProposal".to_string(),
                ParameterChangeProposal {
                    title: "freedom".to_string(),
                    description: "stop gatekeeping cl pools".to_string(),
                    changes: vec![ParamChange {
                        subspace: "concentratedliquidity".to_string(),
                        key: "UnrestrictedPoolCreatorWhitelist".to_string(),
                        value: format!("[\"{}\"]", creator.address().as_str()),
                    }],
                },
                creator.address(),
                creator,
            )
            .unwrap();

        let cl = ConcentratedLiquidity::new(app);

        let pool: MsgCreateConcentratedPoolResponse = cl
            .create_concentrated_pool(
                MsgCreateConcentratedPool {
                    sender: creator.address().to_string(),
                    denom0: OSMO_DENOM.to_string(),
                    denom1: TEST_DENOM.to_string(),
                    tick_spacing: 1000,
                    spread_factor: "500000000000000".to_string(),
                },
                creator,
            )
            .unwrap()
            .data;

        let _create_position_response = cl
            .create_position(
                MsgCreatePosition {
                    pool_id: pool.pool_id,
                    sender: creator.address().to_string(),
                    lower_tick: 1000,
                    upper_tick: 2000,
                    tokens_provided: vec![
                        Coin::new(10_000_000u128, OSMO_DENOM).into(),
                        Coin::new(10_000_000u128, TEST_DENOM).into(),
                    ],
                    token_min_amount0: "0".to_string(),
                    token_min_amount1: "0".to_string(),
                },
                creator,
            )
            .unwrap();

        let cl_pool = ConcentratedLiquidityPool {
            pool_id: pool.pool_id.into(),
            pool_asset_1: OSMO_DENOM.to_string(),
            pool_asset_2: TEST_DENOM.to_string(),
        };

        let pool = cl
            .query_pools(
                &osmosis_std::types::osmosis::concentratedliquidity::v1beta1::PoolsRequest {
                    pagination: None,
                },
            )
            .unwrap();

        let scl_pool: osmosis_std::types::osmosis::concentratedliquidity::v1beta1::Pool =
            pool.pools[0].clone().try_into().unwrap();
        println!("pools: {scl_pool:?}");

        Ok(cl_pool)
    }
}
