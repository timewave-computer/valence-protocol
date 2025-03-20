use std::{error::Error, time::Duration};

use cosmwasm_std::{to_json_binary, CosmosMsg, WasmMsg};
use localic_utils::NEUTRON_CHAIN_DENOM;
use log::info;
use tokio::{runtime::Runtime, time::sleep};
use valence_astroport_utils::astroport_native_lp_token::{Asset, AssetInfo};
use valence_chain_client_utils::{
    cosmos::{base_client::BaseClient, wasm_client::WasmClient},
    neutron::NeutronClient,
    noble::NobleClient,
};
use valence_e2e::utils::UUSDC_DENOM;

use crate::{
    async_run,
    program::{NeutronProgramAccounts, NeutronProgramLibraries},
};

pub fn pull_funds_from_noble_inbound_ica(
    rt: &Runtime,
    neutron_client: &NeutronClient,
    neutron_program_accounts: &NeutronProgramAccounts,
    neutron_program_libraries: &NeutronProgramLibraries,
    uusdc_on_neutron_denom: &str,
    transfer_amount: u128,
) -> Result<(), Box<dyn Error>> {
    info!("bringing in USDC from Noble inbound ICA -> Neutron deposit acc...");
    async_run!(rt, {
        let init_bal = neutron_client
            .query_balance(
                &neutron_program_accounts
                    .deposit_account
                    .to_string()
                    .unwrap(),
                uusdc_on_neutron_denom,
            )
            .await
            .unwrap();
        let transfer_msg = &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
            valence_ica_ibc_transfer::msg::FunctionMsgs::Transfer {},
        );
        let rx = neutron_client
            .execute_wasm(
                &neutron_program_libraries.noble_inbound_transfer,
                transfer_msg,
                vec![],
            )
            .await
            .unwrap();
        neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        for i in 1..10 {
            sleep(Duration::from_secs(3)).await;
            let post_bal = neutron_client
                .query_balance(
                    &neutron_program_accounts
                        .deposit_account
                        .to_string()
                        .unwrap(),
                    uusdc_on_neutron_denom,
                )
                .await
                .unwrap();

            if init_bal + transfer_amount == post_bal {
                info!(
                    "Funds successfully routed in from Noble inbound ICA to Neutron deposit acc!"
                );
                break;
            } else if i == 10 {
                panic!("Failed to route funds from Noble inbound ICA to Neutron deposit acc!");
            } else {
                info!("Funds in transit #{i}...")
            }
        }
    });

    Ok(())
}

pub fn enter_position(
    rt: &Runtime,
    neutron_client: &NeutronClient,
    neutron_program_accounts: &NeutronProgramAccounts,
    neutron_program_libraries: &NeutronProgramLibraries,
    uusdc_on_neutron_denom: &str,
    lp_token_denom: &str,
) -> Result<(), Box<dyn Error>> {
    info!("entering LP position...");
    async_run!(rt, {
        let deposit_account_usdc_bal = neutron_client
            .query_balance(
                &neutron_program_accounts
                    .deposit_account
                    .to_string()
                    .unwrap(),
                uusdc_on_neutron_denom,
            )
            .await
            .unwrap();

        assert_ne!(
            deposit_account_usdc_bal, 0,
            "deposit account must have uusdc in order to lp"
        );

        let provide_liquidity_msg =
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_astroport_lper::msg::FunctionMsgs::ProvideSingleSidedLiquidity {
                    asset: uusdc_on_neutron_denom.to_string(),
                    limit: None,
                    expected_pool_ratio_range: None,
                },
            );
        let rx = neutron_client
            .execute_wasm(
                &neutron_program_libraries.astroport_lper,
                provide_liquidity_msg,
                vec![],
            )
            .await
            .unwrap();
        neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        let output_acc_bal = neutron_client
            .query_balance(
                &neutron_program_accounts
                    .position_account
                    .to_string()
                    .unwrap(),
                lp_token_denom,
            )
            .await
            .unwrap();
        info!("position account LP token balance: {:?}", output_acc_bal);
        assert_ne!(0, output_acc_bal);
    });

    Ok(())
}

pub fn exit_position(
    rt: &Runtime,
    neutron_client: &NeutronClient,
    neutron_program_accounts: &NeutronProgramAccounts,
    neutron_program_libraries: &NeutronProgramLibraries,
    uusdc_on_neutron_denom: &str,
    lp_token_denom: &str,
) -> Result<(), Box<dyn Error>> {
    info!("entering LP position...");
    async_run!(rt, {
        let position_account_shares_bal = neutron_client
            .query_balance(
                &neutron_program_accounts
                    .position_account
                    .to_string()
                    .unwrap(),
                lp_token_denom,
            )
            .await
            .unwrap();

        assert_ne!(
            position_account_shares_bal, 0,
            "position account must have shares in order to exit lp"
        );

        let withdraw_liquidity_msg =
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_astroport_withdrawer::msg::FunctionMsgs::WithdrawLiquidity {
                    expected_pool_ratio_range: None,
                },
            );
        let rx = neutron_client
            .execute_wasm(
                &neutron_program_libraries.astroport_lwer,
                withdraw_liquidity_msg,
                vec![],
            )
            .await
            .unwrap();
        neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        let withdraw_acc_usdc_bal = neutron_client
            .query_balance(
                &neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                uusdc_on_neutron_denom,
            )
            .await
            .unwrap();
        let withdraw_acc_ntrn_bal = neutron_client
            .query_balance(
                &neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                NEUTRON_CHAIN_DENOM,
            )
            .await
            .unwrap();
        info!(
            "withdraw account USDC token balance: {:?}",
            withdraw_acc_usdc_bal
        );
        info!(
            "withdraw account NTRN token balance: {:?}",
            withdraw_acc_ntrn_bal
        );
        assert_ne!(0, withdraw_acc_usdc_bal);
        assert_ne!(0, withdraw_acc_ntrn_bal);
    });

    Ok(())
}

pub fn swap_counterparty_denom_into_usdc(
    rt: &Runtime,
    neutron_client: &NeutronClient,
    neutron_program_accounts: &NeutronProgramAccounts,
    neutron_program_libraries: &NeutronProgramLibraries,
    uusdc_on_neutron_denom: &str,
    lp_token_denom: &str,
    pool_addr: &str,
) -> Result<(), Box<dyn Error>> {
    info!("swapping NTRN into USDC...");
    async_run!(rt, {
        let withdraw_account_ntrn_bal = neutron_client
            .query_balance(
                &neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                NEUTRON_CHAIN_DENOM,
            )
            .await
            .unwrap();

        assert_ne!(
            withdraw_account_ntrn_bal, 0,
            "withdraw account must have NTRN in order to swap into USDC"
        );

        let swap_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pool_addr.to_string(),
            msg: to_json_binary(
                &valence_astroport_utils::astroport_native_lp_token::ExecuteMsg::Swap {
                    offer_asset: Asset {
                        info: AssetInfo::NativeToken {
                            denom: NEUTRON_CHAIN_DENOM.to_string(),
                        },
                        amount: withdraw_account_ntrn_bal.into(),
                    },
                    max_spread: None,
                    belief_price: None,
                    to: None,
                    ask_asset_info: None,
                },
            )
            .unwrap(),
            funds: vec![cosmwasm_std::coin(
                withdraw_account_ntrn_bal,
                NEUTRON_CHAIN_DENOM.to_string(),
            )],
        });

        let base_account_execute_msgs = valence_account_utils::msg::ExecuteMsg::ExecuteMsg {
            msgs: vec![swap_msg],
        };

        let rx = neutron_client
            .execute_wasm(
                &neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                base_account_execute_msgs,
                vec![],
            )
            .await
            .unwrap();

        neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        let withdraw_acc_usdc_bal = neutron_client
            .query_balance(
                &neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                uusdc_on_neutron_denom,
            )
            .await
            .unwrap();
        let withdraw_acc_ntrn_bal = neutron_client
            .query_balance(
                &neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                NEUTRON_CHAIN_DENOM,
            )
            .await
            .unwrap();
        info!(
            "withdraw account USDC token balance: {:?}",
            withdraw_acc_usdc_bal
        );
        info!(
            "withdraw account NTRN token balance: {:?}",
            withdraw_acc_ntrn_bal
        );
        assert_ne!(0, withdraw_acc_usdc_bal);
        assert_eq!(0, withdraw_acc_ntrn_bal);
    });

    Ok(())
}

pub fn route_usdc_to_noble(
    rt: &Runtime,
    neutron_client: &NeutronClient,
    neutron_program_accounts: &NeutronProgramAccounts,
    neutron_program_libraries: &NeutronProgramLibraries,
    uusdc_on_neutron_denom: &str,
    lp_token_denom: &str,
    pool_addr: &str,
) -> Result<(), Box<dyn Error>> {
    info!("routing USDC to noble...");
    async_run!(rt, {
        let transfer_rx = neutron_client
            .transfer(
                &neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                110_000,
                NEUTRON_CHAIN_DENOM,
                None,
            )
            .await
            .unwrap();
        neutron_client.poll_for_tx(&transfer_rx.hash).await.unwrap();
        sleep(Duration::from_secs(3)).await;
        let withdraw_account_usdc_bal = neutron_client
            .query_balance(
                &neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                uusdc_on_neutron_denom,
            )
            .await
            .unwrap();
        let withdraw_account_ntrn_bal = neutron_client
            .query_balance(
                &neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                NEUTRON_CHAIN_DENOM,
            )
            .await
            .unwrap();

        assert_ne!(
            withdraw_account_usdc_bal, 0,
            "withdraw account must have usdc in order to route funds to noble"
        );
        assert_ne!(
            withdraw_account_ntrn_bal, 0,
            "withdraw account must have ntrn in order to route funds to noble"
        );
        let neutron_ibc_transfer_msg =
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_neutron_ibc_transfer_library::msg::FunctionMsgs::IbcTransfer {},
            );
        let rx = neutron_client
            .execute_wasm(
                &neutron_program_libraries.neutron_ibc_transfer,
                neutron_ibc_transfer_msg,
                vec![],
            )
            .await
            .unwrap();
        neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        let withdraw_acc_usdc_bal = neutron_client
            .query_balance(
                &neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                uusdc_on_neutron_denom,
            )
            .await
            .unwrap();
        let withdraw_acc_ntrn_bal = neutron_client
            .query_balance(
                &neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                NEUTRON_CHAIN_DENOM,
            )
            .await
            .unwrap();
        info!(
            "withdraw account USDC token balance: {:?}",
            withdraw_acc_usdc_bal
        );
        info!(
            "withdraw account NTRN token balance: {:?}",
            withdraw_acc_ntrn_bal
        );
        assert_eq!(0, withdraw_acc_usdc_bal);
    });

    Ok(())
}

pub fn cctp_route_usdc_from_noble(
    rt: &Runtime,
    neutron_client: &NeutronClient,
    noble_client: &NobleClient,
    neutron_program_accounts: &NeutronProgramAccounts,
    neutron_program_libraries: &NeutronProgramLibraries,
) -> Result<(), Box<dyn Error>> {
    info!("CCTP forwarding USDC from Noble to Ethereum...");
    async_run!(rt, {
        let transfer_rx = neutron_client
            .transfer(
                &neutron_program_accounts
                    .noble_outbound_ica
                    .library_account
                    .to_string()
                    .unwrap(),
                110_000,
                NEUTRON_CHAIN_DENOM,
                None,
            )
            .await
            .unwrap();
        neutron_client.poll_for_tx(&transfer_rx.hash).await.unwrap();
        sleep(Duration::from_secs(3)).await;

        let noble_outbound_acc_usdc_bal = noble_client
            .query_balance(
                &neutron_program_accounts.noble_outbound_ica.remote_addr,
                UUSDC_DENOM,
            )
            .await
            .unwrap();

        assert_ne!(
            noble_outbound_acc_usdc_bal, 0,
            "Noble outbound ICA account must have usdc in order to initiate CCTP forwarding"
        );

        let neutron_ica_cctp_transfer_msg =
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_ica_cctp_transfer::msg::FunctionMsgs::Transfer {},
            );
        let rx = neutron_client
            .execute_wasm(
                &neutron_program_libraries.noble_cctp_transfer,
                neutron_ica_cctp_transfer_msg,
                vec![],
            )
            .await
            .unwrap();
        neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        sleep(Duration::from_secs(10)).await;

        let noble_outbound_acc_usdc_bal = noble_client
            .query_balance(
                &neutron_program_accounts.noble_outbound_ica.remote_addr,
                UUSDC_DENOM,
            )
            .await
            .unwrap();

        info!(
            "Noble outbound ICA account balance post cctp transfer: {:?}",
            noble_outbound_acc_usdc_bal
        );
    });

    Ok(())
}
