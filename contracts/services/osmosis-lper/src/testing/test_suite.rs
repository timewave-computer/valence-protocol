use std::str::FromStr;

use cosmwasm_std::Uint128;
use osmosis_test_tube::{
    osmosis_std::types::cosmos::bank::v1beta1::{MsgSend, QueryBalanceRequest},
    Account, Bank, Module, Wasm,
};
use valence_osmosis_utils::suite::{
    approve_service, instantiate_input_account, OsmosisTestAppBuilder, OsmosisTestAppSetup,
};
use valence_service_utils::msg::InstantiateMsg;

use crate::{msg::LiquidityProviderConfig, valence_service_integration::ServiceConfig};

const CONTRACT_PATH: &str = "../../../artifacts";

pub struct LPerTestSuite {
    pub inner: OsmosisTestAppSetup,
    pub lper_addr: String,
    pub input_acc: String,
    pub output_acc: String,
}

impl Default for LPerTestSuite {
    fn default() -> Self {
        Self::new(true)
    }
}

impl LPerTestSuite {
    pub fn new(native_lp_token: bool) -> Self {
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
        let lper_addr = instantiate_lper_contract(
            &inner,
            native_lp_token,
            input_acc.clone(),
            output_acc.clone(),
        );

        // Approve the service for the input account
        approve_service(&inner, input_acc.clone(), lper_addr.clone());

        // Give some tokens to the input account so that it can provide liquidity
        let bank = Bank::new(&inner.app);
        bank.send(
            MsgSend {
                from_address: inner.owner_acc().address(),
                to_address: input_acc.clone(),
                amount: vec![
                    osmosis_test_tube::osmosis_std::types::cosmos::base::v1beta1::Coin {
                        denom: inner.pool_asset2.clone(),
                        amount: 1_000_000u128.to_string(),
                    },
                ],
            },
            inner.owner_acc(),
        )
        .unwrap();

        bank.send(
            MsgSend {
                from_address: inner.owner_acc().address(),
                to_address: input_acc.clone(),
                amount: vec![
                    osmosis_test_tube::osmosis_std::types::cosmos::base::v1beta1::Coin {
                        denom: inner.pool_asset1.clone(),
                        amount: 1_000_000u128.to_string(),
                    },
                ],
            },
            inner.owner_acc(),
        )
        .unwrap();

        LPerTestSuite {
            inner,
            lper_addr,
            input_acc,
            output_acc,
        }
    }

    pub fn query_lp_token_balance(&self, addr: String) -> u128 {
        let bank = Bank::new(&self.inner.app);
        let resp = bank
            .query_balance(&QueryBalanceRequest {
                address: addr,
                denom: self.inner.pool_liquidity_token.to_string(),
            })
            .unwrap();
        match resp.balance {
            Some(c) => Uint128::from_str(&c.amount).unwrap().u128(),
            None => 0,
        }
    }
}

fn instantiate_lper_contract(
    setup: &OsmosisTestAppSetup,
    _native_lp_token: bool,
    input_acc: String,
    output_acc: String,
) -> String {
    let wasm = Wasm::new(&setup.app);
    let wasm_byte_code =
        std::fs::read(format!("{}/{}", CONTRACT_PATH, "valence_osmosis_lper.wasm")).unwrap();

    let code_id = wasm
        .store_code(&wasm_byte_code, None, setup.owner_acc())
        .unwrap()
        .data
        .code_id;

    let instantiate_msg = InstantiateMsg {
        owner: setup.owner_acc().address(),
        processor: setup.processor_acc().address(),
        config: ServiceConfig::new(
            input_acc.as_str(),
            output_acc.as_str(),
            LiquidityProviderConfig {
                pool_id: setup.pool_id.into(),
                pool_asset_1: setup.pool_asset1.to_string(),
                pool_asset_2: setup.pool_asset2.to_string(),
            },
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
