use cosmwasm_std::Uint64;
use osmosis_test_tube::{
    osmosis_std::types::cosmos::bank::v1beta1::{MsgSend, QueryAllBalancesRequest},
    Account, Bank, Module, Wasm,
};
use valence_osmosis_utils::suite::{OsmosisTestAppBuilder, OsmosisTestAppSetup};
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
}

fn approve_service(setup: &OsmosisTestAppSetup, account_addr: String, service_addr: String) {
    let wasm = Wasm::new(&setup.app);
    wasm.execute::<valence_account_utils::msg::ExecuteMsg>(
        &account_addr,
        &valence_account_utils::msg::ExecuteMsg::ApproveService {
            service: service_addr,
        },
        &[],
        setup.owner_acc(),
    )
    .unwrap();
}

fn instantiate_input_account(code_id: u64, setup: &OsmosisTestAppSetup) -> String {
    let wasm = Wasm::new(&setup.app);
    wasm.instantiate(
        code_id,
        &valence_account_utils::msg::InstantiateMsg {
            admin: setup.owner_acc().address(),
            approved_services: vec![],
        },
        None,
        Some("base_account"),
        &[],
        setup.owner_acc(),
    )
    .unwrap()
    .data
    .address
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
            setup.pool_id,
            LiquidityProviderConfig {
                pool_id: setup.pool_id.into(),
            },
        ),
    };

    println!("instantiate msg: {:?}", instantiate_msg);

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
