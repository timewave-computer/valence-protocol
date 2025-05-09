use std::error::Error;

use alloy::{
    primitives::{Address, U256},
    providers::Provider,
    rpc::types::TransactionRequest,
    sol_types::SolCall,
};
use valence_chain_client_utils::{
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_e2e::utils::solidity_contracts::AavePositionManager;

use super::strategy::getUserAccountDataCall;

pub async fn get_user_position(
    eth_client: &EthereumClient,
    position_manager_address: Address,
    user_address: Address,
) -> Result<(U256, U256, U256, U256), Box<dyn Error + Send + Sync>> {
    let eth_rp = eth_client.get_request_provider().await?;

    let aave_position_manager = AavePositionManager::new(position_manager_address, &eth_rp);

    let pool_address = eth_client
        .query(aave_position_manager.config())
        .await?
        .poolAddress;

    let user_account_data = getUserAccountDataCall { user: user_address }.abi_encode();

    let result = eth_rp
        .call(
            &TransactionRequest::default()
                .to(pool_address)
                .input(user_account_data.into()),
        )
        .await?;
    let return_data = getUserAccountDataCall::abi_decode_returns(&result, true)?;

    // Divide all values by 10^8 and health factor by 10^18 because that's how AAVE returns them
    let total_collateral_base = return_data
        .totalCollateralBase
        .checked_div(U256::from(1e8))
        .unwrap_or_default();
    let total_debt_base = return_data
        .totalDebtBase
        .checked_div(U256::from(1e8))
        .unwrap_or_default();
    let available_borrows_base = return_data
        .availableBorrowsBase
        .checked_div(U256::from(1e8))
        .unwrap_or_default();
    let health_factor = return_data.healthFactor;

    Ok((
        total_collateral_base,
        total_debt_base,
        available_borrows_base,
        health_factor,
    ))
}
