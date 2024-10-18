use cosmwasm_std::{coin, Coin, Uint128};

use osmosis_test_tube::{
    osmosis_std::{
        try_proto_to_cosmwasm_coins,
        types::{
            cosmos::bank::v1beta1::{MsgSend, QueryAllBalancesRequest},
            cosmwasm::wasm::v1::MsgExecuteContractResponse,
        },
    },
    Account, Bank, ExecuteResponse, Module, Wasm,
};
use valence_osmosis_utils::{
    suite::{
        approve_service, instantiate_input_account, OsmosisTestAppBuilder, OsmosisTestAppSetup,
        OSMO_DENOM, TEST_DENOM,
    },
    utils::DecimalRange,
};
use valence_service_utils::msg::{ExecuteMsg, InstantiateMsg};

use crate::msg::{ActionMsgs, LiquidityProviderConfig, OptionalServiceConfig, ServiceConfig};

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
            None,
        )
    }
}

impl LPerTestSuite {
    pub fn new(with_input_bals: Vec<Coin>, lp_config: Option<LiquidityProviderConfig>) -> Self {
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
            instantiate_lper_contract(&inner, input_acc.clone(), output_acc.clone(), lp_config);

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
    ) -> cosmwasm_std_old::StdResult<Vec<cosmwasm_std_old::Coin>> {
        let bank = Bank::new(&self.inner.app);
        let resp = bank
            .query_all_balances(&QueryAllBalancesRequest {
                address: addr.to_string(),
                pagination: None,
            })
            .unwrap();
        let bals = try_proto_to_cosmwasm_coins(resp.balances)?;
        Ok(bals)
    }

    pub fn provide_two_sided_liquidity(
        &self,
        expected_spot_price: Option<DecimalRange>,
    ) -> ExecuteResponse<MsgExecuteContractResponse> {
        let wasm = Wasm::new(&self.inner.app);

        wasm.execute::<ExecuteMsg<ActionMsgs, OptionalServiceConfig>>(
            &self.lper_addr,
            &ExecuteMsg::ProcessAction(ActionMsgs::ProvideDoubleSidedLiquidity {
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

        wasm.execute::<ExecuteMsg<ActionMsgs, OptionalServiceConfig>>(
            &self.lper_addr,
            &ExecuteMsg::ProcessAction(ActionMsgs::ProvideSingleSidedLiquidity {
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

fn instantiate_lper_contract(
    setup: &OsmosisTestAppSetup,
    input_acc: String,
    output_acc: String,
    lp_config: Option<LiquidityProviderConfig>,
) -> String {
    let wasm = Wasm::new(&setup.app);
    let wasm_byte_code = std::fs::read(format!(
        "{}/{}",
        CONTRACT_PATH, "valence_osmosis_gamm_lper.wasm"
    ))
    .unwrap();

    let code_id = wasm
        .store_code(&wasm_byte_code, None, setup.owner_acc())
        .unwrap()
        .data
        .code_id;

    let pool_id = setup.balancer_pool_cfg.pool_id.u64();

    let instantiate_msg = InstantiateMsg {
        owner: setup.owner_acc().address(),
        processor: setup.processor_acc().address(),
        config: ServiceConfig::new(
            input_acc.as_str(),
            output_acc.as_str(),
            lp_config.unwrap_or(LiquidityProviderConfig {
                pool_id,
                pool_asset_1: setup.balancer_pool_cfg.pool_asset1.to_string(),
                pool_asset_2: setup.balancer_pool_cfg.pool_asset2.to_string(),
            }),
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
