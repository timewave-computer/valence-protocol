use std::{collections::BTreeMap, error::Error, str::FromStr};

use alloy::{
    primitives::{Address, Bytes, U256},
    sol_types::SolValue,
};
use cosmwasm_std::Uint128;
use cosmwasm_std_old::Coin as BankCoin;
use localic_std::modules::{bank, cosmwasm::contract_instantiate};
use localic_utils::{
    utils::{ethereum::EthClient, test_context::TestContext},
    DEFAULT_KEY, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_NAME,
};
use log::{info, warn};
use valence_domain_clients::{
    clients::ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_forwarder_library::msg::{ForwardingConstraints, UncheckedForwardingConfig};
use valence_generic_ibc_transfer_library::msg::IbcTransferAmount;
use valence_ibc_utils::types::EurekaConfig;
use valence_library_utils::{denoms::UncheckedDenom, LibraryAccountType};

use crate::{
    async_run,
    utils::{
        base_account::approve_library,
        hyperlane::{
            set_up_cw_hyperlane_contracts, set_up_eth_hyperlane_contracts, set_up_hyperlane,
        },
        manager::{FORWARDER_NAME, NEUTRON_IBC_TRANSFER_NAME},
        solidity_contracts::{
            ERC1967Proxy,
            ValenceVault::{self},
        },
        ETHEREUM_HYPERLANE_DOMAIN, HYPERLANE_RELAYER_NEUTRON_ADDRESS,
    },
};

#[allow(unused)]
pub struct ProgramHyperlaneContracts {
    pub neutron_hyperlane_contracts: HyperlaneContracts,
    pub eth_hyperlane_contracts: HyperlaneContracts,
}

use super::{hyperlane::HyperlaneContracts, solidity_contracts::ValenceVault::VaultConfig};

pub fn query_vault_packed_values(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
) -> ValenceVault::packedValuesReturn {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();

        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

        eth_client
            .query(valence_vault.packedValues())
            .await
            .unwrap()
    })
}

pub fn pause(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
) -> Result<(), Box<dyn Error>> {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

        match eth_client
            .execute_tx(valence_vault.pause().into_transaction_request())
            .await
        {
            Ok(_) => info!("vault paused!"),
            Err(_) => warn!("failed to pause the vault!"),
        };

        let packed_vals = eth_client
            .query(valence_vault.packedValues())
            .await
            .unwrap();

        assert!(packed_vals.paused, "vault should be paused");
    });

    Ok(())
}

pub fn unpause(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
) -> Result<(), Box<dyn Error>> {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);
        match eth_client
            .execute_tx(valence_vault.unpause().into_transaction_request())
            .await
        {
            Ok(_) => info!("vault resumed!"),
            Err(_) => warn!("failed to resume the vault!"),
        };
        let packed_vals = eth_client
            .query(valence_vault.packedValues())
            .await
            .unwrap();

        assert!(!packed_vals.paused, "vault should be unpaused");
    });

    Ok(())
}

pub fn deposit_to_vault(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    vault_addr: Address,
    user: Address,
    amount: U256,
) -> Result<(), Box<dyn Error>> {
    async_run!(rt, {
        info!("user depositing {amount} into vault...");

        let eth_rp = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

        let signed_tx = valence_vault
            .deposit(amount, user)
            .into_transaction_request()
            .from(user);

        match alloy::providers::Provider::send_transaction(&eth_rp, signed_tx).await {
            Ok(resp) => {
                let tx_hash = resp.get_receipt().await?.transaction_hash;
                info!("deposit completed: {:?}", tx_hash);
            }
            Err(e) => {
                warn!("failed to deposit into vault: {:?}", e)
            }
        };

        Ok(())
    })
}

pub fn query_vault_config(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
) -> ValenceVault::configReturn {
    let config = async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

        eth_client.query(valence_vault.config()).await.unwrap()
    });
    info!("VAULT CONFIG config: {:?}", config);
    config
}

pub fn query_vault_total_assets(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
) -> ValenceVault::totalAssetsReturn {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

        eth_client.query(valence_vault.totalAssets()).await.unwrap()
    })
}

pub fn query_vault_total_supply(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
) -> ValenceVault::totalSupplyReturn {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

        eth_client.query(valence_vault.totalSupply()).await.unwrap()
    })
}

pub fn query_redemption_rate(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
) -> ValenceVault::redemptionRateReturn {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

        eth_client
            .query(valence_vault.redemptionRate())
            .await
            .unwrap()
    })
}

pub fn query_vault_balance_of(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    addr: Address,
) -> ValenceVault::balanceOfReturn {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

        eth_client
            .query(valence_vault.balanceOf(addr))
            .await
            .unwrap()
    })
}

pub fn addr_has_active_withdraw(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    addr: Address,
) -> ValenceVault::hasActiveWithdrawReturn {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

        eth_client
            .query(valence_vault.hasActiveWithdraw(addr))
            .await
            .unwrap()
    })
}

pub fn addr_withdraw_request(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    addr: Address,
) -> ValenceVault::userWithdrawRequestReturn {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

        eth_client
            .query(valence_vault.userWithdrawRequest(addr))
            .await
            .unwrap()
    })
}

pub fn complete_withdraw_request(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    addr: Address,
) -> Result<(), Box<dyn Error>> {
    async_run!(rt, {
        let client = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &client);

        let signed_tx = valence_vault
            .completeWithdraw(addr)
            .into_transaction_request()
            .from(addr);

        match alloy::providers::Provider::send_transaction(&client, signed_tx).await {
            Ok(resp) => {
                let receipt = resp.get_receipt().await.unwrap();
                info!(
                    "withdrawal complete! receipt hash: {:?}",
                    receipt.transaction_hash
                );
            }
            Err(e) => warn!("complete withdrawal request error: {:?}", e),
        };
        Ok(())
    })
}

pub fn redeem(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    addr: Address,
    amount: U256,
    max_loss_bps: u32,
    allow_solver_completion: bool,
) -> Result<(), Box<dyn Error>> {
    async_run!(rt, {
        let client = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &client);
        let signed_tx = valence_vault
            .redeem_0(amount, addr, addr, max_loss_bps, allow_solver_completion)
            .into_transaction_request()
            .from(addr);
        match alloy::providers::Provider::send_transaction(&client, signed_tx).await {
            Ok(resp) => {
                let receipt = resp.get_receipt().await.unwrap();
                info!("redeem request response: {:?}", receipt.transaction_hash);
            }
            Err(e) => warn!("redeem request error: {:?}", e),
        };
        Ok(())
    })
}

pub fn update() -> Result<(), Box<dyn Error>> {
    // query both neutron and eth sides
    // find netting amount
    // update
    Ok(())
}

pub fn setup_liquidation_fwd_lib(
    test_ctx: &mut TestContext,
    input_account: String,
    output_addr: String,
    shares_denom: &str,
) -> Result<String, Box<dyn Error>> {
    let fwd_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get(FORWARDER_NAME)
        .unwrap();

    let fwd_instantiate_msg = valence_library_utils::msg::InstantiateMsg::<
        valence_forwarder_library::msg::LibraryConfig,
    > {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        processor: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        config: valence_forwarder_library::msg::LibraryConfig {
            input_addr: LibraryAccountType::Addr(input_account.clone()),
            output_addr: LibraryAccountType::Addr(output_addr.clone()),
            forwarding_configs: vec![UncheckedForwardingConfig {
                denom: UncheckedDenom::Native(shares_denom.to_string()),
                max_amount: Uint128::MAX,
            }],
            forwarding_constraints: ForwardingConstraints::new(None),
        },
    };

    info!(
        "Neutron Forwarder instantiate message: {:?}",
        fwd_instantiate_msg
    );

    let liquidation_forwarder = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        fwd_code_id,
        &serde_json::to_string(&fwd_instantiate_msg).unwrap(),
        "liquidation_forwarder",
        None,
        "",
    )
    .unwrap();

    info!(
        "Liquidation Forwarder library: {}",
        liquidation_forwarder.address.clone()
    );

    // Approve the library for the base account
    approve_library(
        test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &input_account,
        liquidation_forwarder.address.clone(),
        None,
    );

    Ok(liquidation_forwarder.address)
}

#[allow(clippy::too_many_arguments)]
pub fn setup_neutron_ibc_transfer_lib(
    test_ctx: &mut TestContext,
    input_account: String,
    output_addr: String,
    denom: &str,
    _authorizations: String,
    _processor: String,
    destination_chain_name: &str,
    eureka_config: Option<EurekaConfig>,
) -> Result<String, Box<dyn Error>> {
    let neutron_ibc_transfer_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get(NEUTRON_IBC_TRANSFER_NAME)
        .unwrap();

    info!("neutron ibc transfer code id: {neutron_ibc_transfer_code_id}");

    let remote_chain_info = valence_generic_ibc_transfer_library::msg::RemoteChainInfo {
        channel_id: test_ctx
            .get_transfer_channels()
            .src(NEUTRON_CHAIN_NAME)
            .dest(destination_chain_name)
            .get(),
        ibc_transfer_timeout: None,
    };

    let ibc_transfer_cfg = valence_neutron_ibc_transfer_library::msg::LibraryConfig {
        input_addr: LibraryAccountType::Addr(input_account.to_string()),
        amount: IbcTransferAmount::FullAmount,
        denom: valence_library_utils::denoms::UncheckedDenom::Native(denom.to_string()),
        remote_chain_info,
        output_addr: LibraryAccountType::Addr(output_addr.to_string()),
        memo: "-".to_string(),
        denom_to_pfm_map: BTreeMap::default(),
        eureka_config,
    };

    let neutron_ibc_transfer_instantiate_msg = valence_library_utils::msg::InstantiateMsg::<
        valence_neutron_ibc_transfer_library::msg::LibraryConfig,
    > {
        // TODO: uncomment to not bypass authorizations/processor logic
        // owner: authorizations.to_string(),
        // processor: processor.to_string(),
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        processor: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        config: ibc_transfer_cfg,
    };

    info!(
        "Neutron IBC Transfer instantiate message: {:?}",
        neutron_ibc_transfer_instantiate_msg
    );

    let ibc_transfer = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        neutron_ibc_transfer_code_id,
        &serde_json::to_string(&neutron_ibc_transfer_instantiate_msg).unwrap(),
        "neutron_ibc_transfer",
        None,
        "",
    )
    .unwrap();

    info!(
        "Neutron IBC Transfer library: {}",
        ibc_transfer.address.clone()
    );

    // Approve the library for the base account
    approve_library(
        test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &input_account,
        ibc_transfer.address.clone(),
        None,
    );

    Ok(ibc_transfer.address)
}

/// sets up a Valence Vault on Ethereum with a proxy.
/// approves deposit & withdraw accounts.
#[allow(clippy::too_many_arguments)]
pub fn setup_valence_vault(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    admin: Address,
    eth_deposit_account: String,
    eth_withdraw_account: String,
    vault_deposit_token_addr: Address,
    vault_config: VaultConfig,
    precision: f64,
) -> Result<Address, Box<dyn Error>> {
    let eth_rp = async_run!(rt, eth_client.get_request_provider().await.unwrap());

    info!("deploying Valence Vault on Ethereum...");

    // First deploy the implementation contract
    let implementation_tx = ValenceVault::deploy_builder(&eth_rp)
        .into_transaction_request()
        .from(admin);

    let implementation_address = async_run!(
        rt,
        eth_client
            .execute_tx(implementation_tx)
            .await
            .unwrap()
            .contract_address
            .unwrap()
    );

    info!("Vault deployed at: {implementation_address}");

    let proxy_address = async_run!(rt, {
        // Deploy the proxy contract
        let proxy_tx = ERC1967Proxy::deploy_builder(&eth_rp, implementation_address, Bytes::new())
            .into_transaction_request()
            .from(admin);

        let proxy_address = eth_client
            .execute_tx(proxy_tx)
            .await
            .unwrap()
            .contract_address
            .unwrap();
        info!("Proxy deployed at: {proxy_address}");
        proxy_address
    });

    // Initialize the Vault
    let vault = ValenceVault::new(proxy_address, &eth_rp);

    async_run!(rt, {
        let initialize_tx = vault
            .initialize(
                admin,                            // owner
                vault_config.abi_encode().into(), // encoded config
                vault_deposit_token_addr,         // underlying token
                "Valence Test Vault".to_string(), // vault token name
                "vTEST".to_string(),              // vault token symbol
                U256::from(precision),            // match deposit token precision
            )
            .into_transaction_request()
            .from(admin);

        eth_client.execute_tx(initialize_tx).await.unwrap();
    });

    info!("Approving vault for withdraw account...");
    crate::utils::ethereum::valence_account::approve_library(
        rt,
        eth_client,
        Address::from_str(&eth_withdraw_account).unwrap(),
        proxy_address,
    );

    info!("Approving vault for deposit account...");
    crate::utils::ethereum::valence_account::approve_library(
        rt,
        eth_client,
        Address::from_str(&eth_deposit_account).unwrap(),
        proxy_address,
    );

    Ok(proxy_address)
}

pub mod time {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use log::info;

    pub fn get_current_second() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            % 60
    }

    pub async fn wait_until_next_minute() -> SystemTime {
        let now = SystemTime::now();
        let since_epoch = now.duration_since(UNIX_EPOCH).unwrap();
        let seconds = since_epoch.as_secs() % 60;
        let wait_secs = 60 - seconds;

        log::info!("waiting {} seconds until next minute", wait_secs);
        tokio::time::sleep(Duration::from_secs(wait_secs)).await;

        SystemTime::now()
    }

    pub async fn wait_until_half_minute() {
        let current_second = get_current_second();
        if current_second >= 30 {
            // wait for next minute + 30 seconds
            wait_until_next_minute().await;
            tokio::time::sleep(Duration::from_secs(30)).await;
        } else {
            // wait until second 30 of current minute
            let seconds_to_wait = 30 - current_second;
            info!("waiting {seconds_to_wait} seconds until half minute");
            tokio::time::sleep(Duration::from_secs(seconds_to_wait)).await;
        }
    }
}

pub fn hyperlane_plumbing(
    test_ctx: &mut TestContext,
    eth: &EthClient,
) -> Result<ProgramHyperlaneContracts, Box<dyn Error>> {
    info!("uploading cosmwasm hyperlane contracts...");
    // Upload all Hyperlane contracts to Neutron
    let neutron_hyperlane_contracts = set_up_cw_hyperlane_contracts(test_ctx)?;

    info!("uploading evm hyperlane contracts...");
    // Deploy all Hyperlane contracts on Ethereum
    let eth_hyperlane_contracts = set_up_eth_hyperlane_contracts(eth, ETHEREUM_HYPERLANE_DOMAIN)?;

    info!("setting up hyperlane connection Neutron <> Ethereum");
    set_up_hyperlane(
        "hyperlane-net",
        vec!["localneutron-1-val-0-neutronic", "anvil"],
        "neutron",
        "ethereum",
        &neutron_hyperlane_contracts,
        &eth_hyperlane_contracts,
    )?;

    // Since we are going to relay callbacks to Neutron, let's fund the Hyperlane relayer account with some tokens
    info!("Fund relayer account...");
    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        HYPERLANE_RELAYER_NEUTRON_ADDRESS,
        &[BankCoin {
            denom: NEUTRON_CHAIN_DENOM.to_string(),
            amount: 5_000_000u128.into(),
        }],
        &BankCoin {
            denom: NEUTRON_CHAIN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    Ok(ProgramHyperlaneContracts {
        neutron_hyperlane_contracts,
        eth_hyperlane_contracts,
    })
}

pub mod vault_users {
    use std::collections::BTreeMap;

    use alloy::primitives::{Address, U256};
    use log::info;
    use valence_domain_clients::{
        clients::ethereum::EthereumClient,
        evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
    };

    use crate::utils::{
        ethereum::mock_erc20,
        solidity_contracts::{MockERC20, ValenceVault},
    };

    use super::query_vault_balance_of;

    #[derive(Clone, Debug)]
    pub struct EthereumUsers {
        pub users: Vec<Address>,
        pub starting_balances: BTreeMap<Address, U256>,
        pub erc20: Address,
        pub vault: Address,
    }

    impl EthereumUsers {
        pub fn new(erc20: Address, vault: Address) -> Self {
            Self {
                users: vec![],
                starting_balances: BTreeMap::new(),
                erc20,
                vault,
            }
        }

        pub fn add_user(
            &mut self,
            rt: &tokio::runtime::Runtime,
            eth_client: &EthereumClient,
            user: Address,
        ) {
            info!("Adding new user {user}");
            self.users.push(user);
            info!("Approving erc20 spend for vault on behalf of user");
            mock_erc20::approve(rt, eth_client, self.erc20, user, self.vault, U256::MAX);
        }

        pub fn fund_user(
            &mut self,
            rt: &tokio::runtime::Runtime,
            eth_client: &EthereumClient,
            user: usize,
            amount: U256,
        ) {
            self.starting_balances.insert(self.users[user], amount);
            mock_erc20::mint(rt, eth_client, self.erc20, self.users[user], amount);
        }

        pub fn get_user_shares(
            &self,
            rt: &tokio::runtime::Runtime,
            eth_client: &EthereumClient,
            user: usize,
        ) -> U256 {
            let user_shares_balance =
                query_vault_balance_of(self.vault, rt, eth_client, self.users[user]);
            user_shares_balance._0
        }

        pub fn get_user_deposit_token_bal(
            &self,
            rt: &tokio::runtime::Runtime,
            eth_client: &EthereumClient,
            user: usize,
        ) -> U256 {
            mock_erc20::query_balance(rt, eth_client, self.erc20, self.users[user])
        }

        pub async fn log_balances(
            &self,
            eth_client: &EthereumClient,
            vault_addr: &Address,
            vault_deposit_token: &Address,
        ) {
            let eth_rp = eth_client.get_request_provider().await.unwrap();

            let usdc_token = MockERC20::new(*vault_deposit_token, &eth_rp);
            let valence_vault = ValenceVault::new(*vault_addr, &eth_rp);

            let mut balances = vec![];
            for (i, user) in self.users.iter().enumerate() {
                let usdc_bal = eth_client
                    .query(usdc_token.balanceOf(*user))
                    .await
                    .unwrap()
                    ._0;
                let vault_bal = eth_client
                    .query(valence_vault.balanceOf(*user))
                    .await
                    .unwrap()
                    ._0;
                let starting_balance = self.starting_balances.get(user).unwrap();

                let user_balances_entry = format!(
                    "USER_{i}:
                    starting_usdc: {starting_balance},
                    current_usdc: {usdc_bal},
                    shares: {vault_bal}"
                );
                balances.push(user_balances_entry);
            }

            info!("\nETHEREUM ACCOUNTS LOG");
            for balance_entry in balances {
                info!("{balance_entry}");
            }
        }
    }
}
