use cosmwasm_std::{Coin, StdResult};

use osmosis_std::types::cosmos::bank::v1beta1::MsgSend;
use osmosis_test_tube::{
    osmosis_std::{
        try_proto_to_cosmwasm_coins,
        types::{
            cosmos::bank::v1beta1::QueryAllBalancesRequest,
            cosmwasm::wasm::v1::MsgExecuteContractResponse,
        },
    },
    Account, Bank, ExecuteResponse, Module, Wasm,
};
use valence_osmosis_utils::{
    suite::{OsmosisTestAppBuilder, OsmosisTestAppSetup},
    testing::balancer::BalancerPool,
};
use valence_service_utils::msg::{ExecuteMsg, InstantiateMsg};

use crate::msg::{ActionMsgs, LiquidityWithdrawerConfig, ServiceConfig, ServiceConfigUpdate};

pub struct LPerTestSuite {
    pub inner: OsmosisTestAppSetup<BalancerPool>,
    pub lp_withdrawer_addr: String,
    pub input_acc: String,
    pub output_acc: String,
}

impl Default for LPerTestSuite {
    fn default() -> Self {
        Self::new(50000000000000000000, None)
    }
}

impl LPerTestSuite {
    pub fn new(with_lp_token_amount: u128, lp_config: Option<LiquidityWithdrawerConfig>) -> Self {
        let inner: OsmosisTestAppSetup<BalancerPool> =
            OsmosisTestAppBuilder::new().build().unwrap();

        let wasm = Wasm::new(&inner.app);
        // Create two base accounts
        let account_code_id = inner.store_account_contract();
        let input_acc = inner.instantiate_input_account(account_code_id);
        let output_acc = inner.instantiate_input_account(account_code_id);

        let code_id = inner.store_withdrawer_contract();

        let instantiate_msg = InstantiateMsg {
            owner: inner.owner_acc().address(),
            processor: inner.processor_acc().address(),
            config: ServiceConfig::new(
                input_acc.as_str(),
                output_acc.as_str(),
                lp_config.unwrap_or(LiquidityWithdrawerConfig {
                    pool_id: inner.pool_cfg.pool_id.u64(),
                }),
            ),
        };

        let lp_withdrawer_addr = wasm
            .instantiate(
                code_id,
                &instantiate_msg,
                None,
                Some("lp_withdrawer"),
                &[],
                inner.owner_acc(),
            )
            .unwrap()
            .data
            .address;

        // Approve the service for the input account
        inner.approve_service(input_acc.clone(), lp_withdrawer_addr.clone());

        // transfer all lp tokens to the input account so that it can withdraw
        if with_lp_token_amount > 0 {
            let bank = Bank::new(&inner.app);

            bank.send(
                MsgSend {
                    from_address: inner.accounts[0].address(),
                    to_address: input_acc.clone(),
                    amount: vec![
                        osmosis_test_tube::osmosis_std::types::cosmos::base::v1beta1::Coin {
                            denom: inner.pool_cfg.pool_liquidity_token.clone(),
                            amount: with_lp_token_amount.to_string(),
                        },
                    ],
                },
                &inner.accounts[0],
            )
            .unwrap();
        }

        LPerTestSuite {
            inner,
            lp_withdrawer_addr,
            input_acc,
            output_acc,
        }
    }

    pub fn query_all_balances(&self, addr: &str) -> StdResult<Vec<Coin>> {
        let bank = Bank::new(&self.inner.app);
        let resp = bank
            .query_all_balances(&QueryAllBalancesRequest {
                address: addr.to_string(),
                pagination: None,
                resolve_denom: false,
            })
            .unwrap();
        try_proto_to_cosmwasm_coins(resp.balances)
    }

    pub fn withdraw_liquidity(&self) -> ExecuteResponse<MsgExecuteContractResponse> {
        let wasm = Wasm::new(&self.inner.app);

        wasm.execute::<ExecuteMsg<ActionMsgs, ServiceConfigUpdate>>(
            &self.lp_withdrawer_addr,
            &ExecuteMsg::ProcessAction(ActionMsgs::WithdrawLiquidity {}),
            &[],
            self.inner.processor_acc(),
        )
        .unwrap()
    }
}
