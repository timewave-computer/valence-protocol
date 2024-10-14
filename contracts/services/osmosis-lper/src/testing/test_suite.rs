use std::str::FromStr;

use cosmwasm_std::{coin, Coin, Uint128, Uint64};

use osmosis_test_tube::{
    osmosis_std::{
        try_proto_to_cosmwasm_coins,
        types::cosmos::bank::v1beta1::{MsgSend, QueryAllBalancesRequest, QueryBalanceRequest},
    },
    Account, Bank, Module, PoolManager, Wasm,
};
use valence_osmosis_utils::{
    suite::{
        approve_service, instantiate_input_account, OsmosisTestAppBuilder, OsmosisTestAppSetup,
        OSMO_DENOM, TEST_DENOM,
    },
    utils::OsmosisPoolType,
};
use valence_service_utils::msg::{ExecuteMsg, InstantiateMsg};

use crate::{
    msg::{ActionsMsgs, LiquidityProviderConfig},
    valence_service_integration::{OptionalServiceConfig, ServiceConfig},
};

const CONTRACT_PATH: &str = "../../../artifacts";

pub struct LPerTestSuite {
    pub inner: OsmosisTestAppSetup,
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
            // default to balancer (GAMM)
            OsmosisPoolType::Balancer,
        )
    }
}

impl LPerTestSuite {
    pub fn new(with_input_bals: Vec<Coin>, pool_type: OsmosisPoolType) -> Self {
        let inner = OsmosisTestAppBuilder::new().build().unwrap();

        // Create two base accounts
        let wasm = Wasm::new(&inner.app);

        let wasm_byte_code =
            std::fs::read(format!("{}/{}", CONTRACT_PATH, "valence_base_account.wasm")).unwrap();

        let code_id = wasm
            .store_code(&wasm_byte_code, None, inner.owner_acc())
            .unwrap()
            .data
            .code_id;

        let input_acc = instantiate_input_account(code_id, &inner);
        let output_acc = instantiate_input_account(code_id, &inner);
        let lper_addr =
            instantiate_lper_contract(&inner, input_acc.clone(), output_acc.clone(), pool_type);

        // Approve the service for the input account
        approve_service(&inner, input_acc.clone(), lper_addr.clone());

        // Give some tokens to the input account so that it can provide liquidity
        let bank = Bank::new(&inner.app);

        for input_bal in with_input_bals {
            bank.send(
                MsgSend {
                    from_address: inner.owner_acc().address(),
                    to_address: input_acc.clone(),
                    amount: vec![
                        osmosis_test_tube::osmosis_std::types::cosmos::base::v1beta1::Coin {
                            denom: input_bal.denom.clone(),
                            amount: input_bal.amount.to_string(),
                        },
                    ],
                },
                inner.owner_acc(),
            )
            .unwrap();
        }

        LPerTestSuite {
            inner,
            lper_addr,
            input_acc,
            output_acc,
        }
    }

    pub fn query_all_balances(
        &self,
        addr: &str,
    ) -> cosmwasm_std_polytone::StdResult<Vec<cosmwasm_std_polytone::Coin>> {
        let bank = Bank::new(&self.inner.app);
        let resp = bank
            .query_all_balances(&QueryAllBalancesRequest {
                address: addr.to_string(),
                pagination: None,
            })
            .unwrap();
        let bals = try_proto_to_cosmwasm_coins(resp.balances)?;
        println!("{addr} acc bals: {:?}", bals);
        Ok(bals)
    }

    pub fn _query_lp_token_balance(&self, addr: String) -> u128 {
        let bank = Bank::new(&self.inner.app);
        let resp = bank
            .query_balance(&QueryBalanceRequest {
                address: addr,
                denom: self
                    .inner
                    .balancer_pool_cfg
                    .pool_liquidity_token
                    .to_string(),
            })
            .unwrap();
        match resp.balance {
            Some(c) => Uint128::from_str(&c.amount).unwrap().u128(),
            None => 0,
        }
    }

    pub fn provide_two_sided_liquidity(&self) {
        let wasm = Wasm::new(&self.inner.app);

        wasm.execute::<ExecuteMsg<ActionsMsgs, OptionalServiceConfig>>(
            &self.lper_addr,
            &ExecuteMsg::ProcessAction(ActionsMsgs::ProvideDoubleSidedLiquidity {}),
            &[],
            self.inner.processor_acc(),
        )
        .unwrap();
    }

    pub fn _shift_cl_price(&self, denom_in: &str, amount_in: &str, denom_out: &str) {
        let pm = PoolManager::new(&self.inner.app);

        let swap_route = osmosis_test_tube::osmosis_std::types::osmosis::poolmanager::v1beta1::SwapAmountInRoute {
            pool_id: self.inner.cl_pool_cfg.pool_id.u64(),
            token_out_denom: denom_out.to_string(),
        };

        let proto_coin = osmosis_test_tube::osmosis_std::types::cosmos::base::v1beta1::Coin {
            denom: denom_in.to_string(),
            amount: amount_in.to_string(),
        };

        let msg_swap = osmosis_test_tube::osmosis_std::types::osmosis::poolmanager::v1beta1::MsgSwapExactAmountIn {
            sender: self.inner.owner_acc().address().to_string(),
            routes: vec![swap_route],
            token_in: Some(proto_coin.clone()),
            token_out_min_amount: "1".to_string(),
        };

        let swap_response = pm
            .swap_exact_amount_in(msg_swap, self.inner.owner_acc())
            .unwrap();

        println!(
            "swapped {:?}{} for {:?}",
            swap_response.data.token_out_amount, denom_out, proto_coin
        );
    }

    pub fn provide_single_sided_liquidity(&self, asset: &str, limit: Uint128) {
        let wasm = Wasm::new(&self.inner.app);

        wasm.execute::<ExecuteMsg<ActionsMsgs, OptionalServiceConfig>>(
            &self.lper_addr,
            &ExecuteMsg::ProcessAction(ActionsMsgs::ProvideSingleSidedLiquidity {
                asset: asset.to_string(),
                limit,
            }),
            &[],
            self.inner.processor_acc(),
        )
        .unwrap();
    }
}

fn instantiate_lper_contract(
    setup: &OsmosisTestAppSetup,
    input_acc: String,
    output_acc: String,
    pool_type: OsmosisPoolType,
) -> String {
    let wasm = Wasm::new(&setup.app);
    let wasm_byte_code =
        std::fs::read(format!("{}/{}", CONTRACT_PATH, "valence_osmosis_lper.wasm")).unwrap();

    let code_id = wasm
        .store_code(&wasm_byte_code, None, setup.owner_acc())
        .unwrap()
        .data
        .code_id;

    let pool_id = match pool_type {
        OsmosisPoolType::Balancer => setup.balancer_pool_cfg.pool_id,
        OsmosisPoolType::Concentrated => setup.cl_pool_cfg.pool_id,
        _ => Uint64::MAX, // todo
    }
    .u64();

    let instantiate_msg = InstantiateMsg {
        owner: setup.owner_acc().address(),
        processor: setup.processor_acc().address(),
        config: ServiceConfig::new(
            input_acc.as_str(),
            output_acc.as_str(),
            LiquidityProviderConfig {
                pool_id,
                pool_asset_1: setup.balancer_pool_cfg.pool_asset1.to_string(),
                pool_asset_2: setup.balancer_pool_cfg.pool_asset2.to_string(),
            },
            pool_type,
        ),
    };

    wasm.instantiate(
        code_id,
        &instantiate_msg,
        None,
        Some("lper"),
        &[],
        setup.owner_acc(),
    )
    .unwrap()
    .data
    .address
}
