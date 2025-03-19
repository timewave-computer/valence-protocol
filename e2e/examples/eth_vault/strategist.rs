use std::{error::Error, time::Duration};

use localic_utils::NEUTRON_CHAIN_DENOM;
use log::info;
use tokio::{runtime::Runtime, time::sleep};
use valence_chain_client_utils::{
    cosmos::{base_client::BaseClient, wasm_client::WasmClient},
    neutron::NeutronClient,
};

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
