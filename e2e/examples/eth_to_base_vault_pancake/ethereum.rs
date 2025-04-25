use crate::strategist::strategy_config;

pub fn setup_eth_accounts(
    eth_client: &EthereumClient,
    eth_admin_addr: Address,
) -> Result<strategy_config::ethereum::EthereumAccounts, Box<dyn Error>> {
    info!("Setting up Deposit and Withdraw accounts on Ethereum");

    // create two Valence Base Accounts on Ethereum to test the processor with libraries (in this case the forwarder)
    let deposit_acc_addr = valence_e2e::utils::ethereum::valence_account::setup_valence_account(
        rt,
        eth_client,
        eth_admin_addr,
    )?;
    let withdraw_acc_addr = valence_e2e::utils::ethereum::valence_account::setup_valence_account(
        rt,
        eth_client,
        eth_admin_addr,
    )?;

    let accounts = strategy_config::ethereum::EthereumAccounts {
        deposit: deposit_acc_addr.to_string(),
        withdraw: withdraw_acc_addr.to_string(),
    };

    Ok(accounts)
}
