use cosmwasm_std::Uint128;
use cw20::Cw20ExecuteMsg;
use neutron_test_tube::{
    neutron_std::types::cosmos::{
        bank::v1beta1::{MsgSend, QueryAllBalancesRequest},
        base::v1beta1::Coin as BankCoin,
    },
    Account, Bank, Module, Wasm,
};
use valence_astroport_utils::suite::{AstroportTestAppBuilder, AstroportTestAppSetup};
use valence_library_utils::{
    error::{LibraryError, UnauthorizedReason},
    msg::{ExecuteMsg, InstantiateMsg},
};

use crate::msg::{
    ActionMsgs, LibraryConfig, LibraryConfigUpdate, LiquidityWithdrawerConfig, PoolType,
};

const CONTRACT_PATH: &str = "../../../artifacts";

struct WithdrawerTestSuite {
    pub inner: AstroportTestAppSetup,
    pub withdrawer_addr: String,
    pub input_acc: String,
    pub output_acc: String,
}

impl Default for WithdrawerTestSuite {
    fn default() -> Self {
        Self::new(true)
    }
}

impl WithdrawerTestSuite {
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
        let withdrawer_addr = instantiate_withdrawer_contract(
            &inner,
            native_lp_token,
            input_acc.clone(),
            output_acc.clone(),
        );

        // Approve the library for the input account
        approve_library(&inner, input_acc.clone(), withdrawer_addr.clone());

        // Send some LP tokens that the owner provided to the input account so that it can proceed with the withdraws
        // Send the ones for the native pool
        let bank = Bank::new(&inner.app);
        bank.send(
            MsgSend {
                from_address: inner.owner_acc().address(),
                to_address: input_acc.clone(),
                amount: vec![BankCoin {
                    denom: inner.pool_native_liquidity_token.clone(),
                    amount: "10000".to_string(),
                }],
            },
            inner.owner_acc(),
        )
        .unwrap();

        // Send the ones for the cw20 pool
        wasm.execute(
            &inner.pool_cw20_liquidity_token,
            &Cw20ExecuteMsg::Transfer {
                recipient: input_acc.clone(),
                amount: Uint128::new(10000),
            },
            &[],
            inner.owner_acc(),
        )
        .unwrap();

        WithdrawerTestSuite {
            inner,
            withdrawer_addr,
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
            approved_libraries: vec![],
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

fn approve_library(setup: &AstroportTestAppSetup, account_addr: String, library_addr: String) {
    let wasm = Wasm::new(&setup.app);
    wasm.execute::<valence_account_utils::msg::ExecuteMsg>(
        &account_addr,
        &valence_account_utils::msg::ExecuteMsg::ApproveLibrary {
            library: library_addr,
        },
        &[],
        setup.owner_acc(),
    )
    .unwrap();
}

fn instantiate_withdrawer_contract(
    setup: &AstroportTestAppSetup,
    native_lp_token: bool,
    input_acc: String,
    output_acc: String,
) -> String {
    let wasm = Wasm::new(&setup.app);
    let wasm_byte_code = std::fs::read(format!(
        "{}/{}",
        CONTRACT_PATH, "valence_astroport_withdrawer.wasm"
    ))
    .unwrap();

    let code_id = wasm
        .store_code(&wasm_byte_code, None, setup.owner_acc())
        .unwrap()
        .data
        .code_id;

    let (pool_addr, pool_type) = if native_lp_token {
        (setup.pool_native_addr.clone(), PoolType::NativeLpToken)
    } else {
        (setup.pool_cw20_addr.clone(), PoolType::Cw20LpToken)
    };

    wasm.instantiate(
        code_id,
        &InstantiateMsg {
            owner: setup.owner_acc().address(),
            processor: setup.processor_acc().address(),
            config: LibraryConfig::new(
                input_acc.as_str(),
                output_acc.as_str(),
                pool_addr,
                LiquidityWithdrawerConfig { pool_type },
            ),
        },
        None,
        Some("withdrawer"),
        &[],
        setup.owner_acc(),
    )
    .unwrap()
    .data
    .address
}

#[test]
pub fn only_owner_can_update_config() {
    let setup = WithdrawerTestSuite::default();
    let wasm = Wasm::new(&setup.inner.app);

    let new_config = LibraryConfigUpdate {
        input_addr: Some(setup.input_acc.as_str().into()),
        output_addr: Some(setup.output_acc.as_str().into()),
        pool_addr: Some(setup.inner.pool_cw20_addr.clone()),
        withdrawer_config: Some(LiquidityWithdrawerConfig {
            pool_type: PoolType::Cw20LpToken,
        }),
    };

    let error = wasm
        .execute::<ExecuteMsg<ActionMsgs, LibraryConfigUpdate>>(
            &setup.withdrawer_addr,
            &ExecuteMsg::UpdateConfig {
                new_config: new_config.clone(),
            },
            &[],
            setup.inner.processor_acc(),
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        LibraryError::OwnershipError(cw_ownable::OwnershipError::NotOwner)
            .to_string()
            .as_str(),
    ),);

    wasm.execute::<ExecuteMsg<ActionMsgs, LibraryConfigUpdate>>(
        &setup.withdrawer_addr,
        &ExecuteMsg::UpdateConfig { new_config },
        &[],
        setup.inner.owner_acc(),
    )
    .unwrap();
}

#[test]
fn only_owner_can_update_processor() {
    let setup = WithdrawerTestSuite::default();
    let wasm = Wasm::new(&setup.inner.app);

    let error = wasm
        .execute::<ExecuteMsg<ActionMsgs, LibraryConfigUpdate>>(
            &setup.withdrawer_addr,
            &ExecuteMsg::UpdateProcessor {
                processor: setup.inner.owner_acc().address(),
            },
            &[],
            setup.inner.processor_acc(),
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        LibraryError::OwnershipError(cw_ownable::OwnershipError::NotOwner)
            .to_string()
            .as_str(),
    ),);

    wasm.execute::<ExecuteMsg<ActionMsgs, LibraryConfigUpdate>>(
        &setup.withdrawer_addr,
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
    let setup = WithdrawerTestSuite::default();
    let wasm = Wasm::new(&setup.inner.app);

    let error = wasm
        .execute::<ExecuteMsg<ActionMsgs, LibraryConfigUpdate>>(
            &setup.withdrawer_addr,
            &ExecuteMsg::UpdateOwnership(cw_ownable::Action::TransferOwnership {
                new_owner: setup.inner.processor_acc().address(),
                expiry: None,
            }),
            &[],
            setup.inner.processor_acc(),
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        LibraryError::OwnershipError(cw_ownable::OwnershipError::NotOwner)
            .to_string()
            .as_str(),
    ),);

    wasm.execute::<ExecuteMsg<ActionMsgs, LibraryConfigUpdate>>(
        &setup.withdrawer_addr,
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
fn only_processor_can_execute_actions() {
    let setup = WithdrawerTestSuite::default();
    let wasm = Wasm::new(&setup.inner.app);

    let error = wasm
        .execute::<ExecuteMsg<ActionMsgs, LibraryConfigUpdate>>(
            &setup.withdrawer_addr,
            &ExecuteMsg::ProcessAction(ActionMsgs::WithdrawLiquidity {}),
            &[],
            setup.inner.owner_acc(),
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        LibraryError::Unauthorized(UnauthorizedReason::NotAllowed {})
            .to_string()
            .as_str(),
    ),);

    wasm.execute::<ExecuteMsg<ActionMsgs, LibraryConfigUpdate>>(
        &setup.withdrawer_addr,
        &ExecuteMsg::ProcessAction(ActionMsgs::WithdrawLiquidity {}),
        &[],
        setup.inner.processor_acc(),
    )
    .unwrap();
}

#[test]
fn withdraw_liquidity_native_lp_token() {
    let setup = WithdrawerTestSuite::default();
    let wasm = Wasm::new(&setup.inner.app);
    let bank = Bank::new(&setup.inner.app);

    wasm.execute::<ExecuteMsg<ActionMsgs, LibraryConfigUpdate>>(
        &setup.withdrawer_addr,
        &ExecuteMsg::ProcessAction(ActionMsgs::WithdrawLiquidity {}),
        &[],
        setup.inner.processor_acc(),
    )
    .unwrap();

    // Output account should have received the pool assets
    let output_account_balances_after = bank
        .query_all_balances(&QueryAllBalancesRequest {
            address: setup.output_acc.clone(),
            pagination: None,
            resolve_denom: false,
        })
        .unwrap();

    assert_eq!(output_account_balances_after.balances.len(), 2);
    assert!(output_account_balances_after
        .balances
        .iter()
        .any(|c| c.denom == setup.inner.pool_asset1));
    assert!(output_account_balances_after
        .balances
        .iter()
        .any(|c| c.denom == setup.inner.pool_asset2));
}

#[test]
fn withdraw_liquidity_cw20_lp_token() {
    let setup = WithdrawerTestSuite::new(false);
    let wasm = Wasm::new(&setup.inner.app);
    let bank = Bank::new(&setup.inner.app);

    wasm.execute::<ExecuteMsg<ActionMsgs, LibraryConfigUpdate>>(
        &setup.withdrawer_addr,
        &ExecuteMsg::ProcessAction(ActionMsgs::WithdrawLiquidity {}),
        &[],
        setup.inner.processor_acc(),
    )
    .unwrap();

    // Output account should have received the pool assets
    let output_account_balances_after = bank
        .query_all_balances(&QueryAllBalancesRequest {
            address: setup.output_acc.clone(),
            pagination: None,
            resolve_denom: false,
        })
        .unwrap();

    assert_eq!(output_account_balances_after.balances.len(), 2);
    assert!(output_account_balances_after
        .balances
        .iter()
        .any(|c| c.denom == setup.inner.pool_asset1));
    assert!(output_account_balances_after
        .balances
        .iter()
        .any(|c| c.denom == setup.inner.pool_asset2));
}
