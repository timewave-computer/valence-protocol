use cosmwasm_std::{coin, Coin, Uint128};

use osmosis_test_tube::{
    osmosis_std::types::cosmwasm::wasm::v1::MsgExecuteContractResponse, Account, ExecuteResponse,
    Module, Wasm,
};
use valence_osmosis_utils::{
    suite::{OsmosisTestAppBuilder, OsmosisTestAppSetup, OSMO_DENOM, TEST_DENOM},
    testing::balancer::BalancerPool,
    utils::{DecimalRange, LiquidityProviderConfig},
};
use valence_service_utils::msg::{ExecuteMsg, InstantiateMsg};

use crate::msg::{ActionsMsgs, OptionalServiceConfig, ServiceConfig};

pub struct LPerTestSuite {
    pub inner: OsmosisTestAppSetup<BalancerPool>,
    pub lper_addr: String,
    pub input_acc: String,
    pub output_acc: String,
}

impl Default for LPerTestSuite {
    fn default() -> Self {
        Self::new(
            vec![
                coin(1_000_000u128, OSMO_DENOM),
                coin(1_000_000u128, TEST_DENOM),
            ],
            None,
        )
    }
}

impl LPerTestSuite {
    pub fn new(with_input_bals: Vec<Coin>, lp_config: Option<LiquidityProviderConfig>) -> Self {
        let inner: OsmosisTestAppSetup<BalancerPool> =
            OsmosisTestAppBuilder::new().build().unwrap();

        let wasm = Wasm::new(&inner.app);

        let account_code_id = inner.store_account_contract();
        let input_acc = inner.instantiate_input_account(account_code_id);
        let output_acc = inner.instantiate_input_account(account_code_id);
        let code_id = inner.store_contract();

        let instantiate_msg = InstantiateMsg {
            owner: inner.owner_acc().address(),
            processor: inner.processor_acc().address(),
            config: ServiceConfig::new(
                input_acc.as_str(),
                output_acc.as_str(),
                lp_config.unwrap_or(LiquidityProviderConfig {
                    pool_id: inner.pool_cfg.pool_id.u64(),
                    pool_asset_1: inner.pool_cfg.pool_asset1.to_string(),
                    pool_asset_2: inner.pool_cfg.pool_asset2.to_string(),
                }),
            ),
        };

        let lper_addr = wasm
            .instantiate(
                code_id,
                &instantiate_msg,
                None,
                Some("lper"),
                &[],
                inner.owner_acc(),
            )
            .unwrap()
            .data
            .address;

        // approve the service for the input account
        inner.approve_service(input_acc.clone(), lper_addr.clone());

        // give some tokens to the input account so that it can provide liquidity
        inner.fund_input_acc(input_acc.to_string(), with_input_bals);

        LPerTestSuite {
            inner,
            lper_addr,
            input_acc,
            output_acc,
        }
    }

    pub fn provide_two_sided_liquidity(
        &self,
        expected_spot_price: Option<DecimalRange>,
    ) -> ExecuteResponse<MsgExecuteContractResponse> {
        let wasm = Wasm::new(&self.inner.app);

        wasm.execute::<ExecuteMsg<ActionsMsgs, OptionalServiceConfig>>(
            &self.lper_addr,
            &ExecuteMsg::ProcessAction(ActionsMsgs::ProvideDoubleSidedLiquidity {
                expected_spot_price,
            }),
            &[],
            self.inner.processor_acc(),
        )
        .unwrap()
    }

    pub fn provide_single_sided_liquidity(
        &self,
        asset: &str,
        limit: Uint128,
        expected_spot_price: Option<DecimalRange>,
    ) -> ExecuteResponse<MsgExecuteContractResponse> {
        let wasm = Wasm::new(&self.inner.app);

        wasm.execute::<ExecuteMsg<ActionsMsgs, OptionalServiceConfig>>(
            &self.lper_addr,
            &ExecuteMsg::ProcessAction(ActionsMsgs::ProvideSingleSidedLiquidity {
                expected_spot_price,
                asset: asset.to_string(),
                limit,
            }),
            &[],
            self.inner.processor_acc(),
        )
        .unwrap()
    }
}
