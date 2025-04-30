use std::{
    error::Error,
    path::Path,
    str::FromStr,
    thread::sleep,
    time::{Duration, SystemTime},
};

use alloy::primitives::{Address, U256};

use evm::{setup_eth_accounts, setup_eth_libraries};
use localic_utils::{
    utils::ethereum::EthClient, ConfigChainBuilder, TestContextBuilder, GAIA_CHAIN_NAME,
    LOCAL_IC_API_URL, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_ID,
    NEUTRON_CHAIN_NAME,
};

use log::{info, warn};

use program::{setup_neutron_libraries, upload_neutron_contracts};
use strategist::{
    strategy::Strategy,
    strategy_config::{
        ethereum::{EthereumDenoms, EthereumStrategyConfig},
        neutron::{NeutronDenoms, NeutronStrategyConfig},
        StrategyConfig,
    },
};
use valence_chain_client_utils::{
    cosmos::base_client::BaseClient,
    ethereum::EthereumClient,
    evm::{
        anvil::AnvilImpersonationClient, base_client::EvmBaseClient,
        request_provider_client::RequestProviderClient,
    },
    neutron::NeutronClient,
};

mod program;

use valence_e2e::{
    async_run,
    utils::{
        astroport::setup_astroport_cl_pool,
        authorization::set_up_authorization_and_processor,
        ethereum::{mine_blocks, set_up_anvil_container, ANVIL_NAME, DEFAULT_ANVIL_PORT},
        parse::{get_chain_field_from_local_ic_log, get_grpc_address_and_port_from_url},
        solidity_contracts::{MockERC20, ValenceVault},
        vault::{self, time::wait_until_half_minute, vault_users::EthereumUsers},
        worker::{ValenceWorker, ValenceWorkerTomlSerde},
        DEFAULT_ANVIL_RPC_ENDPOINT, LOGS_FILE_PATH, VALENCE_ARTIFACTS_PATH,
    },
};

mod evm;
mod strategist;

const WBTC_ERC20: &str = "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599";
const WBTC_WHALE: &str = "0x70FBb965302D50D1783a2337Cb115B30Ae9C4638";
const WBTC_NEUTRON_SUBDENOM: &str = "WBTC";
const VAULT_NEUTRON_CACHE_PATH: &str = "e2e/examples/eth_eureka_vault/neutron_contracts/";
const WBTC_NEUTRON_DENOM: &str = "factory/neutron1hj5fveer5cjtn4wd6wstzugjfdxzl0xpznmsky/WBTC";
const ETH_MAINNET_FORK_URL: &str = "https://eth-mainnet.public.blastapi.io";
const EVM_MNEMONIC: &str = "test test test test test test test test test test test junk";
const COSMOS_MNEMONIC: &str = "decorate bright ozone fork gallery riot bus exhaust worth way bone indoor calm squirrel merry zero scheme cotton until shop any excess stage laundry";

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let rt = tokio::runtime::Runtime::new()?;
    async_run!(
        rt,
        set_up_anvil_container(ANVIL_NAME, DEFAULT_ANVIL_PORT, Some(ETH_MAINNET_FORK_URL))
            .await
            .unwrap()
    );

    let eth_client = EthereumClient::new(DEFAULT_ANVIL_RPC_ENDPOINT, EVM_MNEMONIC).unwrap();

    let eth = EthClient::new(DEFAULT_ANVIL_RPC_ENDPOINT)?;

    let eth_rp = async_run!(rt, eth_client.get_request_provider().await.unwrap());

    let strategist_acc = Address::from_str("0x14dc79964da2c08b23698b3d3cc7ca32193d9955").unwrap();
    let wbtc_token_address = Address::from_str(WBTC_ERC20).unwrap();
    let wbtc_contract = MockERC20::new(wbtc_token_address, eth_rp);

    let eth_accounts = async_run!(rt, eth_client.get_provider_accounts().await.unwrap());
    let eth_admin_acc = eth_accounts[0];
    let ethereum_program_accounts = setup_eth_accounts(&rt, &eth_client, eth_admin_acc)?;

    let (neutron_grpc_url, neutron_grpc_port) = get_grpc_address_and_port_from_url(
        &get_chain_field_from_local_ic_log(NEUTRON_CHAIN_ID, "grpc_address")?,
    )?;

    let neutron_client = async_run!(
        rt,
        NeutronClient::new(
            &neutron_grpc_url,
            &neutron_grpc_port,
            COSMOS_MNEMONIC,
            NEUTRON_CHAIN_ID,
        )
        .await
    )?;

    async_run!(rt, tokio::time::sleep(Duration::from_secs(3)).await);

    async_run!(rt, {
        match neutron_client
            .create_tokenfactory_denom(WBTC_NEUTRON_SUBDENOM)
            .await
        {
            Ok(tf_create_rx) => {
                neutron_client
                    .poll_for_tx(&tf_create_rx.hash)
                    .await
                    .unwrap();
            }
            Err(e) => warn!("tokenfactory denom already exists: {:?}", e),
        };
        let tf_mint_rx = neutron_client
            .mint_tokenfactory_tokens(
                WBTC_NEUTRON_SUBDENOM,
                100_000_000_000,
                Some(NEUTRON_CHAIN_ADMIN_ADDR),
            )
            .await
            .unwrap();
        neutron_client.poll_for_tx(&tf_mint_rx.hash).await.unwrap();
    });

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build().unwrap())
        .with_chain(ConfigChainBuilder::default_gaia().build().unwrap())
        .with_transfer_channels(NEUTRON_CHAIN_NAME, GAIA_CHAIN_NAME)
        .with_log_file_path(LOGS_FILE_PATH)
        .build()?;

    // setup astroport
    let (pool_addr, lp_token) =
        setup_astroport_cl_pool(&mut test_ctx, WBTC_NEUTRON_DENOM.to_string())?;

    // set up the authorization and processor contracts on neutron
    let (authorization_contract_address, neutron_processor_address) =
        set_up_authorization_and_processor(
            &mut test_ctx,
            hex::encode(
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)?
                    .as_secs()
                    .to_string(),
            ),
        )?;

    upload_neutron_contracts(&mut test_ctx)?;

    let neutron_program_accounts = program::setup_neutron_accounts(&mut test_ctx)?;

    let neutron_program_libraries = setup_neutron_libraries(
        &mut test_ctx,
        &neutron_program_accounts,
        &pool_addr,
        &authorization_contract_address,
        &neutron_processor_address,
        WBTC_NEUTRON_DENOM,
        ethereum_program_accounts.withdraw.to_string(),
        &lp_token,
    )?;

    let program_hyperlane_contracts =
        valence_e2e::utils::vault::hyperlane_plumbing(&mut test_ctx, &eth)?;

    let ethereum_program_libraries = setup_eth_libraries(
        &rt,
        &eth_client,
        eth_admin_acc,
        strategist_acc,
        ethereum_program_accounts.clone(),
        &eth_accounts,
        program_hyperlane_contracts
            .eth_hyperlane_contracts
            .mailbox
            .to_string(),
        authorization_contract_address,
        wbtc_token_address,
        neutron_program_accounts.deposit.to_string(),
    )?;

    let eth_rp = async_run!(rt, eth_client.get_request_provider().await.unwrap());

    let vault_address = Address::from_str(&ethereum_program_libraries.valence_vault).unwrap();
    let valence_vault = ValenceVault::new(vault_address, &eth_rp);

    // =========================================================================================== //
    // ================================ vault flow begins ======================================== //
    // =========================================================================================== //

    let user_0_deposit_amount = U256::from(5_000_000);
    let user_1_deposit_amount = U256::from(1_000_000);
    let user_2_deposit_amount = U256::from(3_000_000);
    let user_3_deposit_amount = U256::from(250_000_000);
    let mut eth_users = EthereumUsers::new(wbtc_token_address, vault_address);
    eth_users.add_user(&rt, &eth_client, eth_accounts[2]);
    eth_users.add_user(&rt, &eth_client, eth_accounts[3]);
    eth_users.add_user(&rt, &eth_client, eth_accounts[4]);
    eth_users.add_user(&rt, &eth_client, eth_accounts[9]);

    // use the wbtc whale to fund the users
    async_run!(rt, {
        info!("funding eth users with WBTC...");
        eth_client
            .execute_tx_as(
                WBTC_WHALE,
                wbtc_contract
                    .transfer(eth_users.users[0], user_0_deposit_amount)
                    .into_transaction_request(),
            )
            .await
            .unwrap();
        eth_client
            .execute_tx_as(
                WBTC_WHALE,
                wbtc_contract
                    .transfer(eth_users.users[1], user_1_deposit_amount)
                    .into_transaction_request(),
            )
            .await
            .unwrap();
        eth_client
            .execute_tx_as(
                WBTC_WHALE,
                wbtc_contract
                    .transfer(eth_users.users[2], user_2_deposit_amount)
                    .into_transaction_request(),
            )
            .await
            .unwrap();
        eth_client
            .execute_tx_as(
                WBTC_WHALE,
                wbtc_contract
                    .transfer(eth_users.users[3], user_3_deposit_amount)
                    .into_transaction_request(),
            )
            .await
            .unwrap();
    });
    // TODO: start eureka relayer

    info!("main sleep for 3...");
    sleep(Duration::from_secs(3));

    let strategy_config = StrategyConfig {
        neutron: NeutronStrategyConfig {
            grpc_url: neutron_grpc_url,
            grpc_port: neutron_grpc_port,
            chain_id: NEUTRON_CHAIN_ID.to_string(),
            mnemonic: COSMOS_MNEMONIC.to_string(),
            target_pool: pool_addr,
            denoms: NeutronDenoms {
                lp_token: lp_token.to_string(),
                wbtc: WBTC_NEUTRON_DENOM.to_string(),
                ntrn: NEUTRON_CHAIN_DENOM.to_string(),
            },
            accounts: neutron_program_accounts,
            libraries: neutron_program_libraries,
        },
        ethereum: EthereumStrategyConfig {
            rpc_url: DEFAULT_ANVIL_RPC_ENDPOINT.to_string(),
            mnemonic: EVM_MNEMONIC.to_string(),
            denoms: EthereumDenoms {
                wbtc: WBTC_ERC20.to_string(),
            },
            accounts: ethereum_program_accounts.clone(),
            libraries: ethereum_program_libraries.clone(),
        },
    };

    let temp_path = Path::new("./e2e/examples/eth_eureka_vault/strategist/example_strategy.toml");
    strategy_config.to_file(temp_path)?;

    let strategy = async_run!(rt, Strategy::from_file(temp_path).await)?;

    let user2_wbtc_bal = eth_users.get_user_deposit_token_bal(&rt, &eth_client, 2);
    let user2_shares_bal = eth_users.get_user_shares(&rt, &eth_client, 2);
    info!("User2 WBTC balance: {user2_wbtc_bal}");
    info!("User2 shares balance: {user2_shares_bal}");

    info!("User2 depositing {user_2_deposit_amount}WBTC tokens to vault...");
    vault::deposit_to_vault(
        &rt,
        &eth_client,
        *valence_vault.address(),
        eth_users.users[2],
        user_2_deposit_amount,
    )?;

    let user2_wbtc_bal = eth_users.get_user_deposit_token_bal(&rt, &eth_client, 2);
    let user2_shares_bal = eth_users.get_user_shares(&rt, &eth_client, 2);
    info!("User2 WBTC balance: {user2_wbtc_bal}");
    info!("User2 shares balance: {user2_shares_bal}");

    let _strategist_join_handle = strategy.start();

    let vault_address = Address::from_str(&ethereum_program_libraries.valence_vault).unwrap();
    let eth_withdraw_address =
        Address::from_str(&ethereum_program_accounts.withdraw.to_string()).unwrap();

    // ================================================================================ //
    // ================================ vault epoch 1 ================================= //
    // ================================================================================ //
    {
        info!("\n======================== EPOCH 1 ========================\n");
        async_run!(&rt, wait_until_half_minute().await);
        mine_blocks(&rt, &eth_client, 5, 3);

        info!("User0 depositing {user_0_deposit_amount}WBTC tokens to vault...");
        vault::deposit_to_vault(
            &rt,
            &eth_client,
            *valence_vault.address(),
            eth_users.users[0],
            user_0_deposit_amount,
        )?;

        mine_blocks(&rt, &eth_client, 5, 3);
    }

    // ================================================================================ //
    // ================================ vault epoch 2 ================================= //
    // ================================================================================ //
    {
        info!("\n======================== EPOCH 2 ========================\n");
        async_run!(&rt, wait_until_half_minute().await);
        mine_blocks(&rt, &eth_client, 5, 3);

        info!("User1 depositing {user_1_deposit_amount}WBTC tokens to vault...");
        vault::deposit_to_vault(
            &rt,
            &eth_client,
            *valence_vault.address(),
            eth_users.users[1],
            user_1_deposit_amount,
        )?;
        mine_blocks(&rt, &eth_client, 5, 3);
    }

    // ================================================================================ //
    // ================================ vault epoch 3 ================================= //
    // ================================================================================ //
    {
        info!("\n======================== EPOCH 3 ========================\n");
        async_run!(&rt, wait_until_half_minute().await);
        mine_blocks(&rt, &eth_client, 5, 3);

        let user0_pre_redeem_shares_bal = eth_users.get_user_shares(&rt, &eth_client, 0);

        let shares_to_withdraw = user0_pre_redeem_shares_bal / U256::from(2);
        info!("USER0 initiating the redeem of {shares_to_withdraw} shares from vault...");
        let total_to_withdraw_before = async_run!(&rt, {
            eth_client
                .query(valence_vault.totalAssetsToWithdrawNextUpdate())
                .await
                .unwrap()
        })
        ._0;
        info!("Total assets to withdraw before redeem: {total_to_withdraw_before}");

        vault::redeem(
            vault_address,
            &rt,
            &eth_client,
            eth_users.users[0],
            shares_to_withdraw,
            10_000,
            false,
        )?;

        let total_to_withdraw = async_run!(&rt, {
            eth_client
                .query(valence_vault.totalAssetsToWithdrawNextUpdate())
                .await
                .unwrap()
        })
        ._0;
        info!("Total assets to withdraw after redeem: {total_to_withdraw}",);
        assert_ne!(
            total_to_withdraw,
            U256::from(0),
            "totalAssetsToWithdraw should be non-zero"
        );

        let request = async_run!(&rt, {
            eth_client
                .query(valence_vault.userWithdrawRequest(eth_users.users[0]))
                .await
                .unwrap()
        });
        info!("Update withdraw request: {:?}", request);

        mine_blocks(&rt, &eth_client, 5, 3);
    }

    // ================================================================================ //
    // ================================ vault epoch 4 ================================= //
    // ================================================================================ //
    {
        info!("\n======================== EPOCH 4 ========================\n");
        async_run!(&rt, wait_until_half_minute().await);
        mine_blocks(&rt, &eth_client, 5, 3);

        let user1_pre_redeem_shares_bal = eth_users.get_user_shares(&rt, &eth_client, 1);
        info!("USER1 initiating the redeem of {user1_pre_redeem_shares_bal} shares from vault...");
        let total_to_withdraw_before = async_run!(&rt, {
            eth_client
                .query(valence_vault.totalAssetsToWithdrawNextUpdate())
                .await
                .unwrap()
        })
        ._0;
        info!("Total assets to withdraw before redeem: {total_to_withdraw_before}");

        vault::redeem(
            vault_address,
            &rt,
            &eth_client,
            eth_users.users[1],
            user1_pre_redeem_shares_bal,
            10_000,
            false,
        )?;

        let total_to_withdraw = async_run!(&rt, {
            eth_client
                .query(valence_vault.totalAssetsToWithdrawNextUpdate())
                .await
                .unwrap()
        })
        ._0;
        info!("Total assets to withdraw after redeem: {total_to_withdraw}",);
        assert_ne!(
            total_to_withdraw, total_to_withdraw_before,
            "totalAssetsToWithdraw should have increased"
        );

        let request = async_run!(&rt, {
            eth_client
                .query(valence_vault.userWithdrawRequest(eth_users.users[1]))
                .await
                .unwrap()
        });
        info!("User1 update withdraw request: {:?}", request);

        mine_blocks(&rt, &eth_client, 5, 3);
    }

    // ================================================================================ //
    // ================================ vault epoch 5 ================================= //
    // ================================================================================ //
    {
        info!("\n======================== EPOCH 5 ========================\n");
        async_run!(&rt, wait_until_half_minute().await);
        mine_blocks(&rt, &eth_client, 5, 3);

        // Get the withdrawal request details before completion
        let withdraw_request = async_run!(&rt, {
            eth_client
                .query(valence_vault.userWithdrawRequest(eth_users.users[0]))
                .await
                .unwrap()
        });
        info!("User0 Withdraw request details: {:?}", withdraw_request);
        // Get the update info for this request
        let update_info = async_run!(&rt, {
            eth_client
                .query(valence_vault.updateInfos(withdraw_request.updateId as u64))
                .await
                .unwrap()
        });
        info!("Update info for request: {:?}", update_info);

        // Check withdraw account balance
        let withdraw_acc_balance = async_run!(&rt, {
            let erc20 =
                valence_e2e::utils::solidity_contracts::MockERC20::new(wbtc_token_address, &eth_rp);
            eth_client
                .query(erc20.balanceOf(eth_withdraw_address))
                .await
                .unwrap()
        });
        info!("Withdraw account balance: {:?}", withdraw_acc_balance._0);

        async_run!(
            &rt,
            eth_users
                .log_balances(&eth_client, &vault_address, &wbtc_token_address)
                .await
        );

        let user0_withdraw_request =
            vault::addr_has_active_withdraw(vault_address, &rt, &eth_client, eth_users.users[0])._0;
        info!("user0 has withdraw request: {user0_withdraw_request}");

        info!("User0 completing withdraw request...");
        vault::complete_withdraw_request(vault_address, &rt, &eth_client, eth_users.users[0])?;

        async_run!(
            &rt,
            eth_users
                .log_balances(&eth_client, &vault_address, &wbtc_token_address)
                .await
        );

        let post_completion_user0_bal = eth_users.get_user_deposit_token_bal(&rt, &eth_client, 0);
        let post_completion_user0_shares = eth_users.get_user_shares(&rt, &eth_client, 0);
        let user0_withdraw_request =
            vault::addr_has_active_withdraw(vault_address, &rt, &eth_client, eth_users.users[0])._0;
        info!("user0 has withdraw request: {user0_withdraw_request}");
        info!("post completion user0 wbtc bal: {post_completion_user0_bal}",);
        info!("post completion user0 shares bal: {post_completion_user0_shares}",);

        mine_blocks(&rt, &eth_client, 5, 3);
    }

    // ================================================================================ //
    // ================================ vault epoch 6 ================================= //
    // ================================================================================ //
    {
        info!("\n======================== EPOCH 6 ========================\n");
        async_run!(&rt, wait_until_half_minute().await);
        mine_blocks(&rt, &eth_client, 5, 3);

        info!("User0 depositing 2_000_000 to vault");
        vault::deposit_to_vault(
            &rt,
            &eth_client,
            *valence_vault.address(),
            eth_users.users[0],
            U256::from(2_000_000),
        )?;

        let user2_shares = eth_users.get_user_shares(&rt, &eth_client, 2);
        info!("User 2 submitting withdraw request for {user2_shares}shares");

        vault::redeem(
            vault_address,
            &rt,
            &eth_client,
            eth_users.users[2],
            user2_shares,
            10_000,
            false,
        )?;

        mine_blocks(&rt, &eth_client, 5, 3);
    }

    // ================================================================================ //
    // ================================ vault epoch 7 ================================= //
    // ================================================================================ //
    {
        info!("\n======================== EPOCH 7 ========================\n");
        async_run!(&rt, wait_until_half_minute().await);
        mine_blocks(&rt, &eth_client, 5, 3);

        let pre_completion_user2_bal = eth_users.get_user_deposit_token_bal(&rt, &eth_client, 2);
        let pre_completion_user2_shares = eth_users.get_user_shares(&rt, &eth_client, 2);
        let user2_withdraw_request =
            vault::addr_has_active_withdraw(vault_address, &rt, &eth_client, eth_users.users[2])._0;
        info!("user2 has withdraw request: {user2_withdraw_request}");
        info!("pre completion user2 wbtc bal: {pre_completion_user2_bal}",);
        info!("pre completion user2 shares bal: {pre_completion_user2_shares}",);

        info!("User2 completing withdraw request...");
        vault::complete_withdraw_request(vault_address, &rt, &eth_client, eth_users.users[2])?;

        let post_completion_user2_bal = eth_users.get_user_deposit_token_bal(&rt, &eth_client, 2);
        let post_completion_user2_shares = eth_users.get_user_shares(&rt, &eth_client, 2);
        let user2_withdraw_request =
            vault::addr_has_active_withdraw(vault_address, &rt, &eth_client, eth_users.users[2])._0;
        info!("user2 has withdraw request: {user2_withdraw_request}");
        info!("post completion user2 wbtc bal: {post_completion_user2_bal}",);
        info!("post completion user2 shares bal: {post_completion_user2_shares}",);

        mine_blocks(&rt, &eth_client, 5, 3);
    }

    let mut i = 8;
    // ================================================================================ //
    // ============================= vault stayalive loop ============================= //
    // ================================================================================ //
    loop {
        info!("\n======================== EPOCH {i} ========================\n");

        async_run!(&rt, wait_until_half_minute().await);

        mine_blocks(&rt, &eth_client, 5, 3);

        i += 1;

        if i >= 100_000_000 {
            break;
        }
    }

    rt.shutdown_background();

    Ok(())
}
