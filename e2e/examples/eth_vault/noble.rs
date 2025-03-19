use std::error::Error;

use localic_utils::{utils::test_context::TestContext, NEUTRON_CHAIN_NAME};
use log::info;
use tokio::runtime::Runtime;
use valence_chain_client_utils::{cosmos::base_client::BaseClient, noble::NobleClient};
use valence_e2e::utils::{
    parse::get_grpc_address_and_port_from_logs, ADMIN_MNEMONIC, NOBLE_CHAIN_ADMIN_ADDR,
    NOBLE_CHAIN_DENOM, NOBLE_CHAIN_ID, NOBLE_CHAIN_NAME, UUSDC_DENOM,
};

use crate::async_run;

pub fn get_client(rt: &Runtime) -> Result<NobleClient, Box<dyn Error>> {
    let (grpc_url, grpc_port) = get_grpc_address_and_port_from_logs(NOBLE_CHAIN_ID)?;

    let noble_client = async_run!(
        rt,
        NobleClient::new(
            &grpc_url,
            &grpc_port.to_string(),
            ADMIN_MNEMONIC,
            NOBLE_CHAIN_ID,
            NOBLE_CHAIN_DENOM,
        )
        .await
        .unwrap()
    );

    Ok(noble_client)
}

pub fn setup_environment(rt: &Runtime, client: &NobleClient) -> Result<(), Box<dyn Error>> {
    async_run!(
        rt,
        client
            .set_up_test_environment(NOBLE_CHAIN_ADMIN_ADDR, 0, "uusdc")
            .await
    );
    Ok(())
}

pub fn mint_usdc_to_addr(
    rt: &Runtime,
    client: &NobleClient,
    to: &str,
    amount: impl Into<String>,
) -> Result<(), Box<dyn Error>> {
    // Mint some funds to the ICA account
    async_run!(rt, {
        let amount_str: String = amount.into();

        let tx_response = client
            .mint_fiat(NOBLE_CHAIN_ADMIN_ADDR, to, &amount_str, UUSDC_DENOM)
            .await
            .unwrap();
        client.poll_for_tx(&tx_response.hash).await.unwrap();
        info!("Minted {UUSDC_DENOM} to {to}: {:?}", tx_response);
    });

    Ok(())
}

pub fn fund_neutron_addr(
    rt: &Runtime,
    test_ctx: &mut TestContext,
    client: &NobleClient,
    to: &str,
    amount: impl Into<String>,
) -> Result<(), Box<dyn Error>> {
    async_run!(&rt, {
        let rx = client
            .ibc_transfer(
                to.to_string(),
                UUSDC_DENOM.to_string(),
                amount.into(),
                test_ctx
                    .get_transfer_channels()
                    .src(NOBLE_CHAIN_NAME)
                    .dest(NEUTRON_CHAIN_NAME)
                    .get(),
                60,
                None,
            )
            .await
            .unwrap();
        client.poll_for_tx(&rx.hash).await.unwrap();
    });

    Ok(())
}
