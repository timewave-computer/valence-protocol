use cosmwasm_std::{coin, Coin, Int64};

use osmosis_std::{
    cosmwasm_to_proto_coins,
    types::osmosis::concentratedliquidity::v1beta1::{
        MsgCreatePosition, MsgTransferPositions, MsgTransferPositionsResponse,
    },
};
use osmosis_test_tube::{
    osmosis_std::types::{
        cosmwasm::wasm::v1::MsgExecuteContractResponse,
        osmosis::{
            concentratedliquidity::v1beta1::{Pool, UserPositionsRequest, UserPositionsResponse},
            poolmanager::v1beta1::PoolRequest,
        },
    },
    Account, ExecuteResponse, Module, PoolManager, SigningAccount, Wasm,
};
use valence_osmosis_utils::{
    suite::{OsmosisTestAppBuilder, OsmosisTestAppSetup, OSMO_DENOM, TEST_DENOM},
    testing::concentrated_liquidity::ConcentratedLiquidityPool,
};
use valence_service_utils::msg::{ExecuteMsg, InstantiateMsg};

use crate::msg::{ActionMsgs, ServiceConfig, ServiceConfigUpdate};

use super::ConcentratedLiquidityExt;

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
            config: ServiceConfig::new(
                input_acc.as_str(),
                output_acc.as_str(),
                inner.pool_cfg.pool_id,
            ),
        };

        let cl = ConcentratedLiquidityExt::new(&inner.app);

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
        inner.approve_service(input_acc.clone(), lw_addr.clone());

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
        cl.transfer_positions(
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
        let cl = ConcentratedLiquidityExt::new(&self.inner.app);
        let request = UserPositionsRequest {
            address: addr,
            pool_id: self.inner.pool_cfg.pool_id.u64(),
            pagination: None,
        };

        cl.query_user_positions(&request).unwrap()
    }

    pub fn query_cl_pool(&self, id: u64) -> Pool {
        let pm_querier = PoolManager::new(&self.inner.app);
        let pool_response = pm_querier.query_pool(&PoolRequest { pool_id: id }).unwrap();
        let cl_pool: Pool = pool_response.pool.unwrap().try_into().unwrap();

        cl_pool
    }

    pub fn transfer_cl_position(
        &self,
        id: u64,
        from: String,
        to: String,
        signer: &SigningAccount,
    ) -> ExecuteResponse<MsgTransferPositionsResponse> {
        let cl_ext = ConcentratedLiquidityExt::new(&self.inner.app);
        cl_ext
            .transfer_positions(
                MsgTransferPositions {
                    position_ids: vec![id],
                    sender: from,
                    new_owner: to,
                },
                signer,
            )
            .unwrap()
    }

    pub fn liquidate_position(
        &self,
        position_id: u64,
        liquidity_amount: String,
    ) -> ExecuteResponse<MsgExecuteContractResponse> {
        let wasm = Wasm::new(&self.inner.app);

        wasm.execute::<ExecuteMsg<ActionMsgs, ServiceConfigUpdate>>(
            &self.lw_addr,
            &ExecuteMsg::ProcessAction(ActionMsgs::WithdrawLiquidity {
                position_id: position_id.into(),
                liquidity_amount,
            }),
            &[],
            self.inner.processor_acc(),
        )
        .unwrap()
    }
}

// pub struct ConcentratedLiquidityExt<'a, R: Runner<'a>> {
//     runner: &'a R,
// }

// impl<'a, R: Runner<'a>> Module<'a, R> for ConcentratedLiquidityExt<'a, R> {
//     fn new(runner: &'a R) -> Self {
//         Self { runner }
//     }
// }

// impl<'a, R> ConcentratedLiquidityExt<'a, R>
// where
//     R: Runner<'a>,
// {
//     // transfer CL position
//     fn_execute! { pub transfer_positions: MsgTransferPositions => MsgTransferPositionsResponse }
// }
