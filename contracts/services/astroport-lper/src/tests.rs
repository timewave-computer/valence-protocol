use cosmwasm_std::Uint128;
use neutron_test_tube::{
    neutron_std::types::cosmos::{
        bank::v1beta1::{MsgSend, QueryAllBalancesRequest, QueryBalanceRequest},
        base::v1beta1::Coin as BankCoin,
    },
    Account, Bank, Module, Wasm,
};
use valence_astroport_utils::suite::{AstroportTestAppBuilder, AstroportTestAppSetup};
use valence_service_utils::{
    error::{ServiceError, UnauthorizedReason},
    msg::{ExecuteMsg, InstantiateMsg},
};

use crate::msg::{
    ActionsMsgs, AssetData, LiquidityProviderConfig, OptionalServiceConfig, PoolType, ServiceConfig,
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
        approve_service(&inner, input_acc.clone(), lper_addr.clone());

        // Give some tokens to the input account so that it can provide liquidity
        let bank = Bank::new(&inner.app);
        bank.send(
            MsgSend {
                from_address: inner.owner_acc().address(),
                to_address: input_acc.clone(),
                amount: vec![
                    BankCoin {
                        denom: inner.pool_asset2.clone(),
                        amount: 1_000_000u128.to_string(),
                    },
                    BankCoin {
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
            PoolType::NativeLpToken(
                valence_astroport_utils::astroport_native_lp_token::PairType::Xyk {},
            ),
        )
    } else {
        (
            setup.pool_cw20_addr.clone(),
            PoolType::Cw20LpToken(
                valence_astroport_utils::astroport_cw20_lp_token::PairType::Xyk {},
            ),
        )
    };

    wasm.instantiate(
        code_id,
        &InstantiateMsg {
            owner: setup.owner_acc().address(),
            processor: setup.processor_acc().address(),
            config: ServiceConfig::new(
                input_acc.as_str(),
                output_acc.as_str(),
                pool_addr,
                LiquidityProviderConfig {
                    pool_type,
                    asset_data: AssetData {
                        asset1: setup.pool_asset1.clone(),
                        asset2: setup.pool_asset2.clone(),
                    },
                    slippage_tolerance: None,
                },
            ),
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
    let setup = LPerTestSuite::default();
    let wasm = Wasm::new(&setup.inner.app);

    let new_config = OptionalServiceConfig {
        input_addr: Some(setup.input_acc.as_str().into()),
        output_addr: Some(setup.output_acc.as_str().into()),
        pool_addr: Some(setup.inner.pool_cw20_addr.clone()),
        lp_config: Some(LiquidityProviderConfig {
            pool_type: PoolType::Cw20LpToken(
                valence_astroport_utils::astroport_cw20_lp_token::PairType::Xyk {},
            ),
            asset_data: AssetData {
                asset1: setup.inner.pool_asset1.clone(),
                asset2: setup.inner.pool_asset2.clone(),
            },
            slippage_tolerance: None,
        }),
    };

    let error = wasm
        .execute::<ExecuteMsg<ActionsMsgs, OptionalServiceConfig>>(
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

    wasm.execute::<ExecuteMsg<ActionsMsgs, OptionalServiceConfig>>(
        &setup.lper_addr,
        &ExecuteMsg::UpdateConfig { new_config },
        &[],
        setup.inner.owner_acc(),
    )
    .unwrap();
}

#[test]
fn only_owner_can_update_processor() {
    let setup = LPerTestSuite::default();
    let wasm = Wasm::new(&setup.inner.app);

    let error = wasm
        .execute::<ExecuteMsg<ActionsMsgs, OptionalServiceConfig>>(
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

    wasm.execute::<ExecuteMsg<ActionsMsgs, OptionalServiceConfig>>(
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
    let setup = LPerTestSuite::default();
    let wasm = Wasm::new(&setup.inner.app);

    let error = wasm
        .execute::<ExecuteMsg<ActionsMsgs, OptionalServiceConfig>>(
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

    wasm.execute::<ExecuteMsg<ActionsMsgs, OptionalServiceConfig>>(
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
    let setup = LPerTestSuite::default();
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
                config: ServiceConfig::new(
                    setup.inner.owner_acc().address().as_str(),
                    setup.inner.owner_acc().address().as_str(),
                    setup.inner.pool_cw20_addr.clone(),
                    LiquidityProviderConfig {
                        pool_type: PoolType::Cw20LpToken(
                            valence_astroport_utils::astroport_cw20_lp_token::PairType::Xyk {},
                        ),
                        asset_data: AssetData {
                            asset1: setup.inner.pool_asset2.clone(),
                            asset2: setup.inner.pool_asset1.clone(),
                        },
                        slippage_tolerance: None,
                    },
                ),
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
    let setup = LPerTestSuite::default();
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
                config: ServiceConfig::new(
                    setup.inner.owner_acc().address().as_str(),
                    setup.inner.owner_acc().address().as_str(),
                    setup.inner.pool_cw20_addr.clone(),
                    LiquidityProviderConfig {
                        pool_type: PoolType::Cw20LpToken(
                            valence_astroport_utils::astroport_cw20_lp_token::PairType::Stable {},
                        ),
                        asset_data: AssetData {
                            asset1: setup.inner.pool_asset2.clone(),
                            asset2: setup.inner.pool_asset1.clone(),
                        },
                        slippage_tolerance: None,
                    },
                ),
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

#[test]
fn only_processor_can_execute_actions() {
    let setup = LPerTestSuite::default();
    let wasm = Wasm::new(&setup.inner.app);

    let error = wasm
        .execute::<ExecuteMsg<ActionsMsgs, OptionalServiceConfig>>(
            &setup.lper_addr,
            &ExecuteMsg::ProcessAction(ActionsMsgs::ProvideDoubleSidedLiquidity {
                expected_pool_ratio_range: None,
            }),
            &[],
            setup.inner.owner_acc(),
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ServiceError::Unauthorized(UnauthorizedReason::NotAllowed {})
            .to_string()
            .as_str(),
    ),);

    wasm.execute::<ExecuteMsg<ActionsMsgs, OptionalServiceConfig>>(
        &setup.lper_addr,
        &ExecuteMsg::ProcessAction(ActionsMsgs::ProvideDoubleSidedLiquidity {
            expected_pool_ratio_range: None,
        }),
        &[],
        setup.inner.processor_acc(),
    )
    .unwrap();
}

#[test]
fn provide_double_sided_liquidity_native_lp_token() {
    let setup = LPerTestSuite::default();
    let wasm = Wasm::new(&setup.inner.app);
    let bank = Bank::new(&setup.inner.app);

    // Get balances before providing liquidity
    let input_acc_balance_before = bank
        .query_all_balances(&QueryAllBalancesRequest {
            address: setup.input_acc.clone(),
            pagination: None,
            resolve_denom: false,
        })
        .unwrap();

    assert_eq!(input_acc_balance_before.balances.len(), 2);
    assert!(input_acc_balance_before
        .balances
        .iter()
        .any(|c| c.denom == setup.inner.pool_asset1));
    assert!(input_acc_balance_before
        .balances
        .iter()
        .any(|c| c.denom == setup.inner.pool_asset2));

    wasm.execute::<ExecuteMsg<ActionsMsgs, OptionalServiceConfig>>(
        &setup.lper_addr,
        &ExecuteMsg::ProcessAction(ActionsMsgs::ProvideDoubleSidedLiquidity {
            expected_pool_ratio_range: None,
        }),
        &[],
        setup.inner.processor_acc(),
    )
    .unwrap();

    // No balance should be left in the input account
    let input_acc_balance_after = bank
        .query_all_balances(&QueryAllBalancesRequest {
            address: setup.input_acc.clone(),
            pagination: None,
            resolve_denom: false,
        })
        .unwrap();

    assert_eq!(input_acc_balance_after.balances.len(), 0);

    // Output account should have the LP tokens
    let output_acc_balance = bank
        .query_all_balances(&QueryAllBalancesRequest {
            address: setup.output_acc.clone(),
            pagination: None,
            resolve_denom: false,
        })
        .unwrap();

    assert_eq!(output_acc_balance.balances.len(), 1);
    assert_eq!(
        output_acc_balance.balances[0].denom,
        setup.inner.pool_native_liquidity_token
    );
}

#[test]
fn provide_double_sided_liquidity_cw20_lp_token() {
    let setup = LPerTestSuite::new(false);
    let wasm = Wasm::new(&setup.inner.app);
    let bank = Bank::new(&setup.inner.app);

    // Get balances before providing liquidity
    let input_acc_balance_before = bank
        .query_all_balances(&QueryAllBalancesRequest {
            address: setup.input_acc.clone(),
            pagination: None,
            resolve_denom: false,
        })
        .unwrap();

    assert_eq!(input_acc_balance_before.balances.len(), 2);
    assert!(input_acc_balance_before
        .balances
        .iter()
        .any(|c| c.denom == setup.inner.pool_asset1));
    assert!(input_acc_balance_before
        .balances
        .iter()
        .any(|c| c.denom == setup.inner.pool_asset2));

    wasm.execute::<ExecuteMsg<ActionsMsgs, OptionalServiceConfig>>(
        &setup.lper_addr,
        &ExecuteMsg::ProcessAction(ActionsMsgs::ProvideDoubleSidedLiquidity {
            expected_pool_ratio_range: None,
        }),
        &[],
        setup.inner.processor_acc(),
    )
    .unwrap();

    // No balance should be left in the input account
    let input_acc_balance_after = bank
        .query_all_balances(&QueryAllBalancesRequest {
            address: setup.input_acc.clone(),
            pagination: None,
            resolve_denom: false,
        })
        .unwrap();

    assert_eq!(input_acc_balance_after.balances.len(), 0);

    // Output account should have the LP tokens
    let query_balance = wasm
        .query::<cw20::Cw20QueryMsg, cw20::BalanceResponse>(
            &setup.inner.pool_cw20_liquidity_token,
            &cw20::Cw20QueryMsg::Balance {
                address: setup.output_acc.clone(),
            },
        )
        .unwrap();

    assert!(query_balance.balance.u128() > 0);
}

#[test]
fn provide_single_sided_liquidity_native_lp_token() {
    let setup = LPerTestSuite::default();
    let wasm = Wasm::new(&setup.inner.app);
    let bank = Bank::new(&setup.inner.app);

    // Get balances before providing liquidity
    let input_acc_balance_before = bank
        .query_all_balances(&QueryAllBalancesRequest {
            address: setup.input_acc.clone(),
            pagination: None,
            resolve_denom: false,
        })
        .unwrap();

    assert_eq!(input_acc_balance_before.balances.len(), 2);
    assert!(input_acc_balance_before
        .balances
        .iter()
        .any(|c| c.denom == setup.inner.pool_asset1));
    assert!(input_acc_balance_before
        .balances
        .iter()
        .any(|c| c.denom == setup.inner.pool_asset2));

    wasm.execute::<ExecuteMsg<ActionsMsgs, OptionalServiceConfig>>(
        &setup.lper_addr,
        &ExecuteMsg::ProcessAction(ActionsMsgs::ProvideSingleSidedLiquidity {
            asset: setup.inner.pool_asset1.clone(),
            limit: None,
            expected_pool_ratio_range: None,
        }),
        &[],
        setup.inner.processor_acc(),
    )
    .unwrap();

    // No balance should be left in the input account
    let input_acc_balance_after = bank
        .query_all_balances(&QueryAllBalancesRequest {
            address: setup.input_acc.clone(),
            pagination: None,
            resolve_denom: false,
        })
        .unwrap();

    assert_eq!(input_acc_balance_after.balances.len(), 1);
    assert_eq!(
        input_acc_balance_after.balances[0].denom,
        setup.inner.pool_asset2
    );

    // Output account should have the LP tokens
    let output_acc_balance = bank
        .query_all_balances(&QueryAllBalancesRequest {
            address: setup.output_acc.clone(),
            pagination: None,
            resolve_denom: false,
        })
        .unwrap();

    assert_eq!(output_acc_balance.balances.len(), 1);
    assert_eq!(
        output_acc_balance.balances[0].denom,
        setup.inner.pool_native_liquidity_token
    );
}

#[test]
fn provide_single_sided_liquidity_cw20_lp_token() {
    let setup = LPerTestSuite::new(false);
    let wasm = Wasm::new(&setup.inner.app);
    let bank = Bank::new(&setup.inner.app);

    // Get balances before providing liquidity
    let input_acc_balance_before = bank
        .query_all_balances(&QueryAllBalancesRequest {
            address: setup.input_acc.clone(),
            pagination: None,
            resolve_denom: false,
        })
        .unwrap();

    assert_eq!(input_acc_balance_before.balances.len(), 2);
    assert!(input_acc_balance_before
        .balances
        .iter()
        .any(|c| c.denom == setup.inner.pool_asset1));
    assert!(input_acc_balance_before
        .balances
        .iter()
        .any(|c| c.denom == setup.inner.pool_asset2));

    wasm.execute::<ExecuteMsg<ActionsMsgs, OptionalServiceConfig>>(
        &setup.lper_addr,
        &ExecuteMsg::ProcessAction(ActionsMsgs::ProvideSingleSidedLiquidity {
            asset: setup.inner.pool_asset1.clone(),
            limit: None,
            expected_pool_ratio_range: None,
        }),
        &[],
        setup.inner.processor_acc(),
    )
    .unwrap();

    // No balance should be left in the input account
    let input_acc_balance_after = bank
        .query_all_balances(&QueryAllBalancesRequest {
            address: setup.input_acc.clone(),
            pagination: None,
            resolve_denom: false,
        })
        .unwrap();

    assert_eq!(input_acc_balance_after.balances.len(), 1);
    assert_eq!(
        input_acc_balance_after.balances[0].denom,
        setup.inner.pool_asset2
    );

    // Output account should have the LP tokens
    let query_balance = wasm
        .query::<cw20::Cw20QueryMsg, cw20::BalanceResponse>(
            &setup.inner.pool_cw20_liquidity_token,
            &cw20::Cw20QueryMsg::Balance {
                address: setup.output_acc.clone(),
            },
        )
        .unwrap();

    assert!(query_balance.balance.u128() > 0);
}

#[test]
fn test_limit_single_sided_liquidity() {
    let setup = LPerTestSuite::default();
    let wasm = Wasm::new(&setup.inner.app);
    let bank = Bank::new(&setup.inner.app);

    let query_balance = |address: &str| -> u128 {
        bank.query_balance(&QueryBalanceRequest {
            address: address.to_string(),
            denom: setup.inner.pool_asset1.clone(),
        })
        .unwrap()
        .balance
        .unwrap()
        .amount
        .parse()
        .unwrap()
    };

    let input_acc_balance_before = query_balance(&setup.input_acc);

    let liquidity_provided = 500_000u128;

    wasm.execute::<ExecuteMsg<ActionsMsgs, OptionalServiceConfig>>(
        &setup.lper_addr,
        &ExecuteMsg::ProcessAction(ActionsMsgs::ProvideSingleSidedLiquidity {
            asset: setup.inner.pool_asset1.clone(),
            limit: Some(Uint128::new(liquidity_provided)),
            expected_pool_ratio_range: None,
        }),
        &[],
        setup.inner.processor_acc(),
    )
    .unwrap();

    let input_acc_balance_after = query_balance(&setup.input_acc);

    assert!(input_acc_balance_after > 0);
    assert_eq!(
        input_acc_balance_before - liquidity_provided,
        input_acc_balance_after
    );
}
