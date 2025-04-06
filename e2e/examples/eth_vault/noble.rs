use std::error::Error;

use log::info;
use tokio::runtime::Runtime;
use valence_chain_client_utils::{cosmos::base_client::BaseClient, noble::NobleClient};
use valence_e2e::{
    async_run,
    utils::{
        parse::{get_chain_field_from_local_ic_log, get_grpc_address_and_port_from_url},
        ADMIN_MNEMONIC, NOBLE_CHAIN_ADMIN_ADDR, NOBLE_CHAIN_DENOM, NOBLE_CHAIN_ID, UUSDC_DENOM,
    },
};

pub fn get_client(rt: &Runtime) -> Result<NobleClient, Box<dyn Error>> {
    let grpc_addr = get_chain_field_from_local_ic_log(NOBLE_CHAIN_ID, "grpc_address")?;
    let (grpc_url, grpc_port) = get_grpc_address_and_port_from_url(&grpc_addr)?;

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
    amount: u128,
) -> Result<(), Box<dyn Error>> {
    // Mint some funds to the ICA account
    async_run!(rt, {
        let tx_response = client
            .mint_fiat(NOBLE_CHAIN_ADMIN_ADDR, to, &amount.to_string(), UUSDC_DENOM)
            .await
            .unwrap();
        client.poll_for_tx(&tx_response.hash).await.unwrap();
        info!("Minted {UUSDC_DENOM} to {to}: {:?}", tx_response);
    });

    Ok(())
}
