use std::{
    error::Error,
    path::Path,
    str::FromStr,
    thread::sleep,
    time::{Duration, SystemTime},
};

use alloy::primitives::{Address, U256};
use evm::{setup_eth_accounts, setup_eth_libraries, EthereumUsers};
use localic_utils::{
    types::config::ConfigChain, utils::ethereum::EthClient, ConfigChainBuilder, TestContextBuilder,
    LOCAL_IC_API_URL, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_ID,
    NEUTRON_CHAIN_NAME,
};

use log::info;
use neutron::setup_astroport_cl_pool;
use program::{setup_neutron_accounts, setup_neutron_libraries, upload_neutron_contracts};

use strategist::{
    strategy::Strategy,
    strategy_config::{self, StrategyConfig},
};
use utils::wait_until_half_minute;
use valence_chain_client_utils::{
    cosmos::base_client::BaseClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
    noble::NobleClient,
};

use valence_e2e::{
    async_run,
    utils::{
        authorization::set_up_authorization_and_processor,
        ethereum as ethereum_utils,
        mock_cctp_relayer::MockCctpRelayer,
        parse::{get_chain_field_from_local_ic_log, get_grpc_address_and_port_from_url},
        solidity_contracts::ValenceVault,
        vault::{self},
        worker::{ValenceWorker, ValenceWorkerTomlSerde},
        ADMIN_MNEMONIC, DEFAULT_ANVIL_RPC_ENDPOINT, LOGS_FILE_PATH, NOBLE_CHAIN_ADMIN_ADDR,
        NOBLE_CHAIN_DENOM, NOBLE_CHAIN_ID, NOBLE_CHAIN_NAME, NOBLE_CHAIN_PREFIX, UUSDC_DENOM,
        VALENCE_ARTIFACTS_PATH,
    },
};

const _PROVIDE_LIQUIDITY_AUTHORIZATIONS_LABEL: &str = "provide_liquidity";
const _WITHDRAW_LIQUIDITY_AUTHORIZATIONS_LABEL: &str = "withdraw_liquidity";
const ASTROPORT_CONCENTRATED_PAIR_TYPE: &str = "concentrated";
const VAULT_NEUTRON_CACHE_PATH: &str = "e2e/examples/eth_vault/neutron_contracts/";

mod evm;
mod neutron;
mod program;
mod strategist;
mod utils;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Start anvil container
    let rt = tokio::runtime::Runtime::new()?;

    info!("Initializing ethereum side flow...");
    async_run!(rt, ethereum_utils::set_up_anvil_container().await)?;

    let eth = EthClient::new(DEFAULT_ANVIL_RPC_ENDPOINT)?;

    let eth_client = valence_chain_client_utils::ethereum::EthereumClient::new(
        DEFAULT_ANVIL_RPC_ENDPOINT,
        "test test test test test test test test test test test junk",
    )
    .unwrap();
    let eth_rp = async_run!(rt, eth_client.get_request_provider().await.unwrap());

    let eth_accounts = async_run!(rt, eth_client.get_provider_accounts().await.unwrap());
    let eth_admin_acc = eth_accounts[0];
    let _eth_user_acc = eth_accounts[2];
    let strategist_acc = Address::from_str("0x14dc79964da2c08b23698b3d3cc7ca32193d9955").unwrap();

    let ethereum_program_accounts = setup_eth_accounts(&rt, &eth_client, eth_admin_acc)?;

    // set up the cctp messenger
    let mock_cctp_messenger_address = evm::setup_mock_token_messenger(&rt, &eth_client)?;
    // eth side USDC token
    let usdc_token_address =
        ethereum_utils::mock_erc20::setup_deposit_erc20(&rt, &eth_client, "MockUSDC", "USDC")?;

    info!("Setting up Neutron side flow...");

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_chain(ConfigChain {
            denom: NOBLE_CHAIN_DENOM.to_string(),
            debugging: true,
            chain_id: NOBLE_CHAIN_ID.to_string(),
            chain_name: NOBLE_CHAIN_NAME.to_string(),
            chain_prefix: NOBLE_CHAIN_PREFIX.to_string(),
            admin_addr: NOBLE_CHAIN_ADMIN_ADDR.to_string(),
        })
        .with_log_file_path(LOGS_FILE_PATH)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, NOBLE_CHAIN_NAME)
        .build()?;

    // setup hyperlane between neutron and eth
    let program_hyperlane_contracts = utils::hyperlane_plumbing(&mut test_ctx, &eth)?;

    let uusdc_on_neutron_denom = test_ctx
        .get_ibc_denom()
        .base_denom(UUSDC_DENOM.to_owned())
        .src(NOBLE_CHAIN_NAME)
        .dest(NEUTRON_CHAIN_NAME)
        .get();

    // get neutron & noble grpc (url, port) values from local-ic logs
    let (noble_grpc_url, noble_grpc_port) = get_grpc_address_and_port_from_url(
        &get_chain_field_from_local_ic_log(NOBLE_CHAIN_ID, "grpc_address")?,
    )?;
    let (neutron_grpc_url, neutron_grpc_port) = get_grpc_address_and_port_from_url(
        &get_chain_field_from_local_ic_log(NEUTRON_CHAIN_ID, "grpc_address")?,
    )?;

    async_run!(rt, {
        let noble_client = NobleClient::new(
            &noble_grpc_url.to_string(),
            &noble_grpc_port.to_string(),
            ADMIN_MNEMONIC,
            NOBLE_CHAIN_ID,
            NOBLE_CHAIN_DENOM,
        )
        .await
        .unwrap();

        noble_client
            .set_up_test_environment(NOBLE_CHAIN_ADMIN_ADDR, 0, UUSDC_DENOM)
            .await;

        let tx_response = noble_client
            .mint_fiat(
                NOBLE_CHAIN_ADMIN_ADDR,
                NOBLE_CHAIN_ADMIN_ADDR,
                &999900000.to_string(),
                UUSDC_DENOM,
            )
            .await
            .unwrap();
        noble_client.poll_for_tx(&tx_response.hash).await.unwrap();

        let rx = noble_client
            .ibc_transfer(
                NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
                UUSDC_DENOM.to_string(),
                999000000.to_string(),
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
        noble_client.poll_for_tx(&rx.hash).await.unwrap();
    });

    sleep(Duration::from_secs(3));

    // setup astroport
    let (pool_addr, lp_token) =
        setup_astroport_cl_pool(&mut test_ctx, uusdc_on_neutron_denom.to_string())?;

    let amount_to_transfer = 1_000_000;

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

    let neutron_program_accounts = setup_neutron_accounts(&mut test_ctx)?;

    let neutron_program_libraries = setup_neutron_libraries(
        &mut test_ctx,
        &neutron_program_accounts,
        &pool_addr,
        &authorization_contract_address,
        &neutron_processor_address,
        amount_to_transfer,
        &uusdc_on_neutron_denom,
        ethereum_program_accounts.withdraw.to_string(),
        &lp_token,
    )?;

    let ethereum_program_libraries = setup_eth_libraries(
        &rt,
        &eth_client,
        eth_admin_acc,
        strategist_acc,
        ethereum_program_accounts.clone(),
        mock_cctp_messenger_address,
        usdc_token_address,
        neutron_program_accounts
            .noble_inbound_ica
            .remote_addr
            .to_string(),
        program_hyperlane_contracts
            .eth_hyperlane_contracts
            .mailbox
            .to_string(),
        authorization_contract_address,
        &eth_accounts,
    )?;

    let vault_address = Address::from_str(&ethereum_program_libraries.valence_vault).unwrap();
    let valence_vault = ValenceVault::new(vault_address, &eth_rp);

    let user_1_deposit_amount = U256::from(5_000_000);
    let user_2_deposit_amount = U256::from(1_000_000);
    let user_3_deposit_amount = U256::from(3_000_000);

    let mut eth_users = EthereumUsers::new(usdc_token_address, vault_address);
    eth_users.add_user(&rt, &eth_client, eth_accounts[2]);
    eth_users.fund_user(&rt, &eth_client, 0, user_1_deposit_amount);
    eth_users.add_user(&rt, &eth_client, eth_accounts[3]);
    eth_users.fund_user(&rt, &eth_client, 1, user_2_deposit_amount);
    eth_users.add_user(&rt, &eth_client, eth_accounts[4]);
    eth_users.fund_user(&rt, &eth_client, 2, user_3_deposit_amount);

    info!("Starting CCTP mock relayer between Noble and Ethereum...");
    let mock_cctp_relayer = async_run!(
        tokio::runtime::Runtime::new()?,
        MockCctpRelayer::new(mock_cctp_messenger_address, usdc_token_address).await
    )?;
    let _cctp_rly_join_handle = mock_cctp_relayer.start();

    info!("main sleep for 3...");
    sleep(Duration::from_secs(3));

    let strategy_config = StrategyConfig {
        noble: strategy_config::noble::NobleStrategyConfig {
            grpc_url: noble_grpc_url,
            grpc_port: noble_grpc_port,
            chain_id: NOBLE_CHAIN_ID.to_string(),
            mnemonic: ADMIN_MNEMONIC.to_string(),
        },
        neutron: strategy_config::neutron::NeutronStrategyConfig {
            grpc_url: neutron_grpc_url,
            grpc_port: neutron_grpc_port,
            chain_id: NEUTRON_CHAIN_ID.to_string(),
            mnemonic: ADMIN_MNEMONIC.to_string(),
            target_pool: pool_addr.to_string(),
            denoms: strategy_config::neutron::NeutronDenoms {
                lp_token: lp_token.to_string(),
                usdc: uusdc_on_neutron_denom.to_string(),
                ntrn: NEUTRON_CHAIN_DENOM.to_string(),
            },
            accounts: neutron_program_accounts,
            libraries: neutron_program_libraries,
        },
        ethereum: strategy_config::ethereum::EthereumStrategyConfig {
            rpc_url: DEFAULT_ANVIL_RPC_ENDPOINT.to_string(),
            mnemonic: "test test test test test test test test test test test junk".to_string(),
            denoms: strategy_config::ethereum::EthereumDenoms {
                usdc_erc20: usdc_token_address.to_string(),
            },
            accounts: ethereum_program_accounts.clone(),
            libraries: ethereum_program_libraries.clone(),
        },
    };

    let temp_path = Path::new("./e2e/examples/eth_vault/strategist/example_strategy.toml");
    strategy_config.to_file(temp_path)?;

    let strategy = async_run!(
        tokio::runtime::Runtime::new()?,
        Strategy::from_file(temp_path).await
    )?;

    info!("User3 depositing {user_3_deposit_amount}USDC tokens to vault...");
    vault::deposit_to_vault(
        &rt,
        &eth_client,
        *valence_vault.address(),
        eth_users.users[2],
        user_3_deposit_amount,
    )?;

    let _strategist_join_handle = strategy.start();

    let vault_address = Address::from_str(&ethereum_program_libraries.valence_vault).unwrap();
    let eth_withdraw_address =
        Address::from_str(&ethereum_program_accounts.withdraw.to_string()).unwrap();

    // epoch 0
    {
        info!("\n======================== EPOCH 0 ========================\n");
        async_run!(&rt, wait_until_half_minute().await);
        evm::mine_blocks(&rt, &eth_client, 5, 3);

        info!("User depositing {user_1_deposit_amount}USDC tokens to vault...");
        vault::deposit_to_vault(
            &rt,
            &eth_client,
            *valence_vault.address(),
            eth_users.users[0],
            user_1_deposit_amount,
        )?;

        evm::mine_blocks(&rt, &eth_client, 5, 3);
    }

    // epoch 1
    {
        info!("\n======================== EPOCH 1 ========================\n");
        async_run!(&rt, wait_until_half_minute().await);
        evm::mine_blocks(&rt, &eth_client, 5, 3);

        info!("User2 depositing {user_2_deposit_amount}USDC tokens to vault...");
        vault::deposit_to_vault(
            &rt,
            &eth_client,
            *valence_vault.address(),
            eth_users.users[1],
            U256::from(1_000_000),
        )?;
        evm::mine_blocks(&rt, &eth_client, 5, 3);
    }

    // epoch 2
    {
        info!("\n======================== EPOCH 2 ========================\n");
        async_run!(&rt, wait_until_half_minute().await);
        evm::mine_blocks(&rt, &eth_client, 5, 3);

        let user1_pre_redeem_shares_bal = eth_users.get_user_shares(&rt, &eth_client, 0);
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
            eth_users.users[0],
            user1_pre_redeem_shares_bal / U256::from(2),
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

        evm::mine_blocks(&rt, &eth_client, 5, 3);
    }

    {
        info!("\n======================== EPOCH 3 ========================\n");
        async_run!(&rt, wait_until_half_minute().await);
        evm::mine_blocks(&rt, &eth_client, 5, 3);

        let user2_pre_redeem_shares_bal = eth_users.get_user_shares(&rt, &eth_client, 1);
        info!("USER2 initiating the redeem of {user2_pre_redeem_shares_bal} shares from vault...");
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
            user2_pre_redeem_shares_bal,
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
        info!("Update withdraw request: {:?}", request);

        evm::mine_blocks(&rt, &eth_client, 5, 3);
    }

    {
        info!("\n======================== EPOCH 4 ========================\n");
        async_run!(&rt, wait_until_half_minute().await);
        evm::mine_blocks(&rt, &eth_client, 5, 3);

        // Get the withdrawal request details before completion
        let withdraw_request = async_run!(&rt, {
            eth_client
                .query(valence_vault.userWithdrawRequest(eth_users.users[0]))
                .await
                .unwrap()
        });
        info!("Withdraw request details: {:?}", withdraw_request);
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
                valence_e2e::utils::solidity_contracts::MockERC20::new(usdc_token_address, &eth_rp);
            eth_client
                .query(erc20.balanceOf(eth_withdraw_address))
                .await
                .unwrap()
        });
        info!("Withdraw account balance: {:?}", withdraw_acc_balance._0);

        async_run!(
            &rt,
            eth_users
                .log_balances(&eth_client, &vault_address, &usdc_token_address,)
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
                .log_balances(&eth_client, &vault_address, &usdc_token_address,)
                .await
        );

        let post_completion_user0_bal = eth_users.get_user_usdc(&rt, &eth_client, 0);
        let post_completion_user0_shares = eth_users.get_user_shares(&rt, &eth_client, 0);
        let user0_withdraw_request =
            vault::addr_has_active_withdraw(vault_address, &rt, &eth_client, eth_users.users[0])._0;
        info!("user0 has withdraw request: {user0_withdraw_request}");
        info!("post completion user0 usdc bal: {post_completion_user0_bal}",);
        info!("post completion user0 shares bal: {post_completion_user0_shares}",);

        evm::mine_blocks(&rt, &eth_client, 5, 3);
    }

    {
        info!("\n======================== EPOCH 5 ========================\n");
        async_run!(&rt, wait_until_half_minute().await);
        evm::mine_blocks(&rt, &eth_client, 5, 3);

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

        evm::mine_blocks(&rt, &eth_client, 5, 3);
    }

    {
        info!("\n======================== EPOCH 6 ========================\n");
        async_run!(&rt, wait_until_half_minute().await);
        evm::mine_blocks(&rt, &eth_client, 5, 3);

        let pre_completion_user2_bal = eth_users.get_user_usdc(&rt, &eth_client, 2);
        let pre_completion_user2_shares = eth_users.get_user_shares(&rt, &eth_client, 2);
        let user2_withdraw_request =
            vault::addr_has_active_withdraw(vault_address, &rt, &eth_client, eth_users.users[2])._0;
        info!("user2 has withdraw request: {user2_withdraw_request}");
        info!("pre completion user2 usdc bal: {pre_completion_user2_bal}",);
        info!("pre completion user2 shares bal: {pre_completion_user2_shares}",);

        info!("User2 completing withdraw request...");
        vault::complete_withdraw_request(vault_address, &rt, &eth_client, eth_users.users[2])?;

        let post_completion_user2_bal = eth_users.get_user_usdc(&rt, &eth_client, 2);
        let post_completion_user2_shares = eth_users.get_user_shares(&rt, &eth_client, 2);
        let user2_withdraw_request =
            vault::addr_has_active_withdraw(vault_address, &rt, &eth_client, eth_users.users[2])._0;
        info!("user2 has withdraw request: {user2_withdraw_request}");
        info!("post completion user2 usdc bal: {post_completion_user2_bal}",);
        info!("post completion user2 shares bal: {post_completion_user2_shares}",);

        evm::mine_blocks(&rt, &eth_client, 5, 3);
    }

    Ok(())
}
