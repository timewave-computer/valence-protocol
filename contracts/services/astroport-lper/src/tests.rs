use neutron_test_tube::{Account, Module, Wasm};
use valence_astroport_utils::suite::{AstroportTestAppBuilder, AstroportTestAppSetup};

use crate::{
    error::ServiceError,
    msg::{
        AssetData, ExecuteMsg, InstantiateMsg, LiquidityProviderConfig, PoolType, ServiceConfig,
    },
};

const CONTRACT_PATH: &str = "../../../artifacts";

struct LPerTestSuite {
    pub inner: AstroportTestAppSetup,
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
        let inner = AstroportTestAppBuilder::new().build().unwrap();

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
        approve_service(&inner, input_acc.clone(), output_acc.clone());

        LPerTestSuite {
            inner,
            lper_addr,
            input_acc,
            output_acc,
        }
    }
}

fn instantiate_input_account(code_id: u64, setup: &AstroportTestAppSetup) -> String {
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

fn approve_service(setup: &AstroportTestAppSetup, account_addr: String, service_addr: String) {
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

fn instantiate_lper_contract(
    setup: &AstroportTestAppSetup,
    native_lp_token: bool,
    input_acc: String,
    output_acc: String,
) -> String {
    let wasm = Wasm::new(&setup.app);
    let wasm_byte_code = std::fs::read(format!(
        "{}/{}",
        CONTRACT_PATH, "valence_astroport_lper.wasm"
    ))
    .unwrap();

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
                input_addr: input_acc,
                output_addr: output_acc,
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
pub fn only_owner_can_update_config() {
    let setup = LPerTestSuite::new(true);
    let wasm = Wasm::new(&setup.inner.app);

    let new_config = ServiceConfig {
        input_addr: setup.input_acc.clone(),
        output_addr: setup.output_acc.clone(),
        pool_addr: setup.inner.pool_cw20_addr.clone(),
        lp_config: LiquidityProviderConfig {
            pool_type: PoolType::Cw20LpToken(astroport_cw20_lp_token::factory::PairType::Xyk {}),
            asset_data: AssetData {
                asset1: setup.inner.pool_asset1.clone(),
                asset2: setup.inner.pool_asset2.clone(),
            },
            slippage_tolerance: None,
        },
    };

    let error = wasm
        .execute::<ExecuteMsg>(
            &setup.lper_addr,
            &ExecuteMsg::UpdateConfig {
                new_config: new_config.clone(),
            },
            &[],
            setup.inner.processor_acc(),
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ServiceError::OwnershipError(cw_ownable::OwnershipError::NotOwner)
            .to_string()
            .as_str(),
    ),);

    wasm.execute::<ExecuteMsg>(
        &setup.lper_addr,
        &ExecuteMsg::UpdateConfig { new_config },
        &[],
        setup.inner.owner_acc(),
    )
    .unwrap();
}

#[test]
fn only_owner_can_update_processor() {
    let setup = LPerTestSuite::new(true);
    let wasm = Wasm::new(&setup.inner.app);

    let error = wasm
        .execute::<ExecuteMsg>(
            &setup.lper_addr,
            &ExecuteMsg::UpdateProcessor {
                processor: setup.inner.owner_acc().address(),
            },
            &[],
            setup.inner.processor_acc(),
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ServiceError::OwnershipError(cw_ownable::OwnershipError::NotOwner)
            .to_string()
            .as_str(),
    ),);

    wasm.execute::<ExecuteMsg>(
        &setup.lper_addr,
        &ExecuteMsg::UpdateProcessor {
            processor: setup.inner.owner_acc().address(),
        },
        &[],
        setup.inner.owner_acc(),
    )
    .unwrap();
}

#[test]
fn only_owner_can_transfer_ownership() {
    let setup = LPerTestSuite::new(true);
    let wasm = Wasm::new(&setup.inner.app);

    let error = wasm
        .execute::<ExecuteMsg>(
            &setup.lper_addr,
            &ExecuteMsg::UpdateOwnership(cw_ownable::Action::TransferOwnership {
                new_owner: setup.inner.processor_acc().address(),
                expiry: None,
            }),
            &[],
            setup.inner.processor_acc(),
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ServiceError::OwnershipError(cw_ownable::OwnershipError::NotOwner)
            .to_string()
            .as_str(),
    ),);

    wasm.execute::<ExecuteMsg>(
        &setup.lper_addr,
        &ExecuteMsg::UpdateOwnership(cw_ownable::Action::TransferOwnership {
            new_owner: setup.inner.processor_acc().address(),
            expiry: None,
        }),
        &[],
        setup.inner.owner_acc(),
    )
    .unwrap();
}

#[test]
fn instantiate_with_wrong_assets() {
    let setup = LPerTestSuite::new(true);
    let wasm = Wasm::new(&setup.inner.app);

    let error = wasm
        .instantiate(
            wasm.store_code(
                &std::fs::read(format!(
                    "{}/{}",
                    CONTRACT_PATH, "valence_astroport_lper.wasm"
                ))
                .unwrap(),
                None,
                setup.inner.owner_acc(),
            )
            .unwrap()
            .data
            .code_id,
            &InstantiateMsg {
                owner: setup.inner.owner_acc().address(),
                processor: setup.inner.processor_acc().address(),
                config: ServiceConfig {
                    input_addr: setup.inner.owner_acc().address(),
                    output_addr: setup.inner.owner_acc().address(),
                    pool_addr: setup.inner.pool_cw20_addr.clone(),
                    lp_config: LiquidityProviderConfig {
                        pool_type: PoolType::Cw20LpToken(
                            astroport_cw20_lp_token::factory::PairType::Xyk {},
                        ),
                        asset_data: AssetData {
                            asset1: setup.inner.pool_asset2.clone(),
                            asset2: setup.inner.pool_asset1.clone(),
                        },
                        slippage_tolerance: None,
                    },
                },
            },
            None,
            Some("lper"),
            &[],
            setup.inner.owner_acc(),
        )
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("Pool asset does not match the expected asset"),);
}

#[test]
fn instantiate_with_wrong_pool_type() {
    let setup = LPerTestSuite::new(true);
    let wasm = Wasm::new(&setup.inner.app);

    let error = wasm
        .instantiate(
            wasm.store_code(
                &std::fs::read(format!(
                    "{}/{}",
                    CONTRACT_PATH, "valence_astroport_lper.wasm"
                ))
                .unwrap(),
                None,
                setup.inner.owner_acc(),
            )
            .unwrap()
            .data
            .code_id,
            &InstantiateMsg {
                owner: setup.inner.owner_acc().address(),
                processor: setup.inner.processor_acc().address(),
                config: ServiceConfig {
                    input_addr: setup.inner.owner_acc().address(),
                    output_addr: setup.inner.owner_acc().address(),
                    pool_addr: setup.inner.pool_cw20_addr.clone(),
                    lp_config: LiquidityProviderConfig {
                        pool_type: PoolType::Cw20LpToken(
                            astroport_cw20_lp_token::factory::PairType::Stable {},
                        ),
                        asset_data: AssetData {
                            asset1: setup.inner.pool_asset2.clone(),
                            asset2: setup.inner.pool_asset1.clone(),
                        },
                        slippage_tolerance: None,
                    },
                },
            },
            None,
            Some("lper"),
            &[],
            setup.inner.owner_acc(),
        )
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("Pool type does not match the expected pair type"),);
}
