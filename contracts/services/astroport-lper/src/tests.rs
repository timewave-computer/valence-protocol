use neutron_test_tube::{
    neutron_std::types::cosmos::bank::v1beta1::QueryAllBalancesRequest, Account, Bank, Module, Wasm,
};
use valence_astroport_utils::suite::{AstroportTestAppBuilder, AstroportTestAppSetup};

use crate::{
    error::ServiceError,
    msg::{
        AssetData, ExecuteMsg, InstantiateMsg, LiquidityProviderConfig, PoolType, ServiceConfig,
    },
};

const CONTRACT_PATH: &str = "../../../artifacts/valence_astroport_lper.wasm";

fn instantiate_lper_contract(setup: &AstroportTestAppSetup, native_lp_token: bool) -> String {
    let wasm = Wasm::new(&setup.app);

    let wasm_byte_code = std::fs::read(CONTRACT_PATH).unwrap();

    let code_id = wasm
        .store_code(&wasm_byte_code, None, setup.owner_acc())
        .unwrap()
        .data
        .code_id;

    let (pool_addr, pool_type) = if native_lp_token {
        (
            setup.pool_native_addr.clone(),
            PoolType::NativeLpToken(astroport::factory::PairType::Xyk {}),
        )
    } else {
        (
            setup.pool_cw20_addr.clone(),
            PoolType::Cw20LpToken(astroport_cw20_lp_token::factory::PairType::Xyk {}),
        )
    };

    wasm.instantiate(
        code_id,
        &InstantiateMsg {
            owner: setup.owner_acc().address(),
            processor: setup.processor_acc().address(),
            config: ServiceConfig {
                input_addr: setup.input_acc().address(),
                output_addr: setup.output_acc().address(),
                pool_addr,
                lp_config: LiquidityProviderConfig {
                    pool_type,
                    asset_data: AssetData {
                        asset1: setup.pool_asset1.clone(),
                        asset2: setup.pool_asset2.clone(),
                    },
                    slippage_tolerance: None,
                },
            },
        },
        None,
        Some("lper"),
        &[],
        setup.owner_acc(),
    )
    .unwrap()
    .data
    .address
}

#[test]
pub fn test_input_account_balance_initiation() {
    let setup = AstroportTestAppBuilder::new().build().unwrap();

    let bank = Bank::new(&setup.app);

    let balance = bank
        .query_all_balances(&QueryAllBalancesRequest {
            address: setup.input_acc().address(),
            pagination: None,
            resolve_denom: false,
        })
        .unwrap();

    assert_eq!(balance.balances.len(), 2);
    assert!(balance
        .balances
        .iter()
        .any(|token| token.denom == setup.pool_asset1));
    assert!(balance
        .balances
        .iter()
        .any(|token| token.denom == setup.pool_asset2));
}

#[test]
pub fn only_owner_can_update_config() {
    let setup = AstroportTestAppBuilder::new().build().unwrap();
    let lper_addr = instantiate_lper_contract(&setup, false);
    let wasm = Wasm::new(&setup.app);

    let new_config = ServiceConfig {
        input_addr: setup.input_acc().address(),
        output_addr: setup.output_acc().address(),
        pool_addr: setup.pool_cw20_addr.clone(),
        lp_config: LiquidityProviderConfig {
            pool_type: PoolType::Cw20LpToken(astroport_cw20_lp_token::factory::PairType::Xyk {}),
            asset_data: AssetData {
                asset1: setup.pool_asset1.clone(),
                asset2: setup.pool_asset2.clone(),
            },
            slippage_tolerance: None,
        },
    };

    let error = wasm
        .execute::<ExecuteMsg>(
            &lper_addr,
            &ExecuteMsg::UpdateConfig {
                new_config: new_config.clone(),
            },
            &[],
            setup.input_acc(),
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ServiceError::OwnershipError(cw_ownable::OwnershipError::NotOwner)
            .to_string()
            .as_str(),
    ),);

    wasm.execute::<ExecuteMsg>(
        &lper_addr,
        &ExecuteMsg::UpdateConfig { new_config },
        &[],
        setup.owner_acc(),
    )
    .unwrap();
}

#[test]
fn only_owner_can_update_processor() {
    let setup = AstroportTestAppBuilder::new().build().unwrap();
    let lper_addr = instantiate_lper_contract(&setup, false);
    let wasm = Wasm::new(&setup.app);

    let error = wasm
        .execute::<ExecuteMsg>(
            &lper_addr,
            &ExecuteMsg::UpdateProcessor {
                processor: setup.input_acc().address(),
            },
            &[],
            setup.input_acc(),
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ServiceError::OwnershipError(cw_ownable::OwnershipError::NotOwner)
            .to_string()
            .as_str(),
    ),);

    wasm.execute::<ExecuteMsg>(
        &lper_addr,
        &ExecuteMsg::UpdateProcessor {
            processor: setup.input_acc().address(),
        },
        &[],
        setup.owner_acc(),
    )
    .unwrap();
}

#[test]
fn only_owner_can_transfer_ownership() {
    let setup = AstroportTestAppBuilder::new().build().unwrap();
    let lper_addr = instantiate_lper_contract(&setup, false);
    let wasm = Wasm::new(&setup.app);

    let error = wasm
        .execute::<ExecuteMsg>(
            &lper_addr,
            &ExecuteMsg::UpdateOwnership(cw_ownable::Action::TransferOwnership {
                new_owner: setup.input_acc().address(),
                expiry: None,
            }),
            &[],
            setup.input_acc(),
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ServiceError::OwnershipError(cw_ownable::OwnershipError::NotOwner)
            .to_string()
            .as_str(),
    ),);

    wasm.execute::<ExecuteMsg>(
        &lper_addr,
        &ExecuteMsg::UpdateOwnership(cw_ownable::Action::TransferOwnership {
            new_owner: setup.input_acc().address(),
            expiry: None,
        }),
        &[],
        setup.owner_acc(),
    )
    .unwrap();
}

#[test]
fn cant_instantiate_with_wrong_assets() {
    let setup = AstroportTestAppBuilder::new().build().unwrap();
    let wasm = Wasm::new(&setup.app);

    let error = wasm
        .instantiate(
            wasm.store_code(
                &std::fs::read(CONTRACT_PATH).unwrap(),
                None,
                setup.owner_acc(),
            )
            .unwrap()
            .data
            .code_id,
            &InstantiateMsg {
                owner: setup.owner_acc().address(),
                processor: setup.processor_acc().address(),
                config: ServiceConfig {
                    input_addr: setup.input_acc().address(),
                    output_addr: setup.output_acc().address(),
                    pool_addr: setup.pool_cw20_addr.clone(),
                    lp_config: LiquidityProviderConfig {
                        pool_type: PoolType::Cw20LpToken(
                            astroport_cw20_lp_token::factory::PairType::Xyk {},
                        ),
                        asset_data: AssetData {
                            asset1: setup.pool_asset2.clone(),
                            asset2: setup.pool_asset1.clone(),
                        },
                        slippage_tolerance: None,
                    },
                },
            },
            None,
            Some("lper"),
            &[],
            setup.owner_acc(),
        )
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("Pool asset does not match the expected asset"),);
}
