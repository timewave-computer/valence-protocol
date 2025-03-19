use std::{error::Error, time::Duration};

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
