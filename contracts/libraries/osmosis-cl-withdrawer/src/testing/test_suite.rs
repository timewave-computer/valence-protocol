use cosmwasm_std::{coin, Coin, Int64};

use osmosis_std::{
    cosmwasm_to_proto_coins,
    types::osmosis::concentratedliquidity::v1beta1::{MsgCreatePosition, MsgTransferPositions},
};
use osmosis_test_tube::{
    osmosis_std::types::{
        cosmwasm::wasm::v1::MsgExecuteContractResponse,
        osmosis::concentratedliquidity::v1beta1::{UserPositionsRequest, UserPositionsResponse},
    },
    Account, ConcentratedLiquidity, ExecuteResponse, Module, Wasm,
};
use valence_library_utils::msg::{ExecuteMsg, InstantiateMsg};
use valence_osmosis_utils::{
    suite::{OsmosisTestAppBuilder, OsmosisTestAppSetup, OSMO_DENOM, TEST_DENOM},
    testing::concentrated_liquidity::ConcentratedLiquidityPool,
};

use crate::msg::{FunctionMsgs, LibraryConfig, LibraryConfigUpdate};

use super::ConcentratedLiquidityExts;

// use super::ConcentratedLiquidityExt;

pub struct LPerTestSuite {
    pub inner: OsmosisTestAppSetup<ConcentratedLiquidityPool>,
    pub lw_addr: String,
    pub input_acc: String,
    pub output_acc: String,
}

impl Default for LPerTestSuite {
    fn default() -> Self {
        Self::new(vec![
            coin(1_000_000u128, OSMO_DENOM),
            coin(1_000_000u128, TEST_DENOM),
        ])
    }
}

impl LPerTestSuite {
    pub fn new(with_input_bals: Vec<Coin>) -> Self {
        let inner: OsmosisTestAppSetup<ConcentratedLiquidityPool> =
            OsmosisTestAppBuilder::new().build().unwrap();

        // Create two base accounts
        let wasm = Wasm::new(&inner.app);

        let account_code_id = inner.store_account_contract();
        let input_acc = inner.instantiate_input_account(account_code_id);
        let output_acc = inner.instantiate_input_account(account_code_id);
        let lw_code_id = inner.store_withdrawer_contract();

        let instantiate_msg = InstantiateMsg {
            owner: inner.owner_acc().address(),
            processor: inner.processor_acc().address(),
            config: LibraryConfig::new(
                input_acc.as_str(),
                output_acc.as_str(),
                inner.pool_cfg.pool_id,
            ),
        };

        let lw_addr = wasm
            .instantiate(
                lw_code_id,
                &instantiate_msg,
                None,
                Some("lwer"),
                &[],
                inner.owner_acc(),
            )
            .unwrap()
            .data
            .address;

        // Approve the service for the input account
        inner.approve_library(input_acc.clone(), lw_addr.clone());
        let cl = ConcentratedLiquidity::new(&inner.app);

        // create a CL position and transfer it to the input acc
        cl.create_position(
            MsgCreatePosition {
                pool_id: inner.pool_cfg.pool_id.u64(),
                sender: inner.accounts[0].address(),
                lower_tick: Int64::from(-1000).i64(),
                upper_tick: Int64::from(1000).i64(),
                tokens_provided: cosmwasm_to_proto_coins(with_input_bals),
                token_min_amount0: "0".to_string(),
                token_min_amount1: "0".to_string(),
            },
            &inner.accounts[0],
        )
        .unwrap();

        ConcentratedLiquidityExts::new(&inner.app)
            .transfer_positions(
                MsgTransferPositions {
                    position_ids: vec![2],
                    sender: inner.accounts[0].address(),
                    new_owner: input_acc.to_string(),
                },
                &inner.accounts[0],
            )
            .unwrap();

        LPerTestSuite {
            inner,
            lw_addr,
            input_acc,
            output_acc,
        }
    }

    pub fn query_cl_positions(&self, addr: String) -> UserPositionsResponse {
        let cl = ConcentratedLiquidity::new(&self.inner.app);
        let request = UserPositionsRequest {
            address: addr,
            pool_id: self.inner.pool_cfg.pool_id.u64(),
            pagination: None,
        };

        cl.query_user_positions(&request).unwrap()
    }

    pub fn liquidate_position(
        &self,
        position_id: u64,
        liquidity_amount: String,
    ) -> ExecuteResponse<MsgExecuteContractResponse> {
        let wasm = Wasm::new(&self.inner.app);

        wasm.execute::<ExecuteMsg<FunctionMsgs, LibraryConfigUpdate>>(
            &self.lw_addr,
            &ExecuteMsg::ProcessFunction(FunctionMsgs::WithdrawLiquidity {
                position_id: position_id.into(),
                liquidity_amount,
            }),
            &[],
            self.inner.processor_acc(),
        )
        .unwrap()
    }
}
