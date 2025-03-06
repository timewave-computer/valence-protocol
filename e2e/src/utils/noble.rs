use log::info;
use valence_chain_client_utils::{cosmos::base_client::BaseClient, noble::NobleClient};

use super::NOBLE_CHAIN_ADMIN_ADDR;

const CCTP_MODULE_NAME: &str = "cctp";
const ALLOWANCE: &str = "1000000000000000000000";
const DUMMY_ADDRESS: &[u8; 32] = &[0x01; 32];

pub async fn set_up_noble(noble_client: &NobleClient, domain_id: u32, denom: &str) {
    // First get the module account for cctp
    let cctp_module_account_address = noble_client
        .query_module_account(CCTP_MODULE_NAME)
        .await
        .unwrap()
        .base_account
        .unwrap()
        .address;

    // Then we confgure the module account as a minter controller
    let tx_response = noble_client
        .configure_minter_controller(
            NOBLE_CHAIN_ADMIN_ADDR,
            NOBLE_CHAIN_ADMIN_ADDR,
            &cctp_module_account_address,
        )
        .await
        .unwrap();
    info!("Minter controller configured response: {:?}", tx_response);
    noble_client.poll_for_tx(&tx_response.hash).await.unwrap();

    // Then we configure the module account as a minter with a big mint allowance
    let tx_response = noble_client
        .configure_minter(
            NOBLE_CHAIN_ADMIN_ADDR,
            &cctp_module_account_address,
            ALLOWANCE,
            denom,
        )
        .await
        .unwrap();
    info!("Minter configured response: {:?}", tx_response);
    noble_client.poll_for_tx(&tx_response.hash).await.unwrap();

    // Add a remote token messenger address for the domain_id
    // Any address will do as we just want to test the burn functionality
    let tx_response = noble_client
        .add_remote_token_messenger(NOBLE_CHAIN_ADMIN_ADDR, domain_id, DUMMY_ADDRESS)
        .await;

    match tx_response {
        Ok(response) => {
            noble_client.poll_for_tx(&response.hash).await.unwrap();
            info!("Remote token messenger added response: {:?}", response);
        }
        Err(_) => {
            info!("Remote token messenger already added!");
        }
    }

    // Link the local token with a remote token
    // Any remote token will do for testing
    let tx_response = noble_client
        .link_token_pair(NOBLE_CHAIN_ADMIN_ADDR, domain_id, DUMMY_ADDRESS, denom)
        .await;
    match tx_response {
        Ok(response) => {
            noble_client.poll_for_tx(&response.hash).await.unwrap();
            info!("Token pair linked response: {:?}", response);
        }
        Err(_) => {
            info!("Token pair already linked!");
        }
    }
}
