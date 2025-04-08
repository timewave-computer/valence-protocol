use std::{error::Error, str::FromStr};

use alloy::{
    primitives::{Address, U256},
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use cosmwasm_std::Uint128;
use localic_utils::{NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_ID};
use log::{info, warn};
use tokio::runtime::Runtime;
use valence_chain_client_utils::{
    cosmos::base_client::BaseClient,
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
    neutron::NeutronClient,
    noble::NobleClient,
};

use valence_e2e::{
    async_run,
    utils::{
        parse::{get_chain_field_from_local_ic_log, get_grpc_address_and_port_from_url},
        solidity_contracts::{MockERC20, ValenceVault},
        ADMIN_MNEMONIC, DEFAULT_ANVIL_RPC_ENDPOINT, NOBLE_CHAIN_DENOM, NOBLE_CHAIN_ID,
    },
};

use crate::{
    evm::EthereumProgramAccounts,
    program::{NeutronProgramAccounts, NeutronProgramLibraries},
    strategist::{astroport::AstroportOps, routing::EthereumVaultRouting, vault::EthereumVault},
};

pub(crate) struct Strategist {
    pub eth_client: EthereumClient,
    pub noble_client: NobleClient,
    pub neutron_client: NeutronClient,
    pub neutron_program_accounts: NeutronProgramAccounts,
    pub neutron_program_libraries: NeutronProgramLibraries,
    pub eth_program_accounts: EthereumProgramAccounts,
    pub uusdc_on_neutron_denom: String,
    pub lp_token_denom: String,
    pub pool_addr: String,
    pub cctp_transfer_lib: Address,
    pub vault_addr: Address,
    pub ethereum_usdc_erc20: Address,
}

impl Strategist {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rt: &Runtime,
        neutron_program_accounts: NeutronProgramAccounts,
        neutron_program_libraries: NeutronProgramLibraries,
        ethereum_program_accounts: EthereumProgramAccounts,
        uusdc_on_neutron_denom: String,
        lp_token_denom: String,
        pool_addr: String,
        cctp_transfer_lib: Address,
        vault_addr: Address,
        ethereum_usdc_erc20: Address,
    ) -> Result<Self, Box<dyn Error>> {
        let noble_grpc_addr = get_chain_field_from_local_ic_log(NOBLE_CHAIN_ID, "grpc_address")?;
        let (noble_grpc_url, noble_grpc_port) =
            get_grpc_address_and_port_from_url(&noble_grpc_addr)?;

        let noble_client = async_run!(rt, {
            NobleClient::new(
                &noble_grpc_url,
                &noble_grpc_port.to_string(),
                ADMIN_MNEMONIC,
                NOBLE_CHAIN_ID,
                NOBLE_CHAIN_DENOM,
            )
            .await
            .expect("failed to create noble client")
        });

        let neutron_grpc_addr =
            get_chain_field_from_local_ic_log(NEUTRON_CHAIN_ID, "grpc_address")?;
        let (neutron_grpc_url, neutron_grpc_port) =
            get_grpc_address_and_port_from_url(&neutron_grpc_addr)?;

        let neutron_client = async_run!(rt, {
            NeutronClient::new(
                &neutron_grpc_url,
                &neutron_grpc_port.to_string(),
                ADMIN_MNEMONIC,
                NEUTRON_CHAIN_ID,
            )
            .await
            .expect("failed to create neutron client")
        });

        let signer = MnemonicBuilder::<English>::default()
            .phrase("test test test test test test test test test test test junk")
            .index(7)? // derive the mnemonic at a different index to avoid nonce issues
            .build()?;

        let eth_client = EthereumClient {
            rpc_url: DEFAULT_ANVIL_RPC_ENDPOINT.to_string(),
            signer,
        };

        Ok(Self {
            eth_client,
            noble_client,
            neutron_client,
            neutron_program_accounts,
            neutron_program_libraries,
            eth_program_accounts: ethereum_program_accounts,
            uusdc_on_neutron_denom,
            lp_token_denom,
            pool_addr,
            cctp_transfer_lib,
            vault_addr,
            ethereum_usdc_erc20,
        })
    }
}

impl Strategist {
    pub async fn start(self) {
        info!("Starting...");

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(15));
        let mut i = 0;
        loop {
            info!("loop #{i}");
            interval.tick().await;

            // STEP 1: pulling funds due for withdrawal from position to origin domain
            //   0. swap neutron withdraw acc neutron tokens into usdc (leaving enough neutron for ibc transfer)
            //   1. ibc transfer neutron withdraw acc -> noble outbound ica
            //   2. cctp transfer noble outbound ica -> eth withdraw acc
            self.swap_ntrn_into_usdc().await;

            let neutron_withdraw_acc_usdc_bal = self
                .neutron_client
                .query_balance(
                    &self
                        .neutron_program_accounts
                        .withdraw_account
                        .to_string()
                        .unwrap(),
                    &self.uusdc_on_neutron_denom,
                )
                .await
                .unwrap();
            if neutron_withdraw_acc_usdc_bal > 0 {
                info!("Neutron withdraw account USDC balance greater than 0!\nRouting from position to origin chain.");
                self.route_neutron_to_noble().await;
                self.route_noble_to_eth().await;
            }

            // STEP 2: updating the vault to conclude the previous epoch:
            // redemption rate R = total_shares / total_assets
            let redemption_rate = self.calculate_redemption_rate().await.unwrap();
            let total_fee = self.calculate_total_fee().await.unwrap();
            let netting_amount = self.calculate_netting_amount().await.unwrap();
            let r = U256::from(redemption_rate.atomics().u128());
            // Update the Vault with R, F_total, N
            match self
                .vault_update(r, total_fee, U256::from(netting_amount))
                .await
            {
                Ok(resp) => {
                    info!("vault update response: {:?}", resp);
                }
                Err(err) => warn!("vault update error: {:?}", err),
            };

            // STEP 3. pulling funds due for deposit from origin to position domain
            //   1. cctp transfer eth deposit acc -> noble inbound ica
            //   2. ica ibc transfer noble inbound ica -> neutron deposit acc
            self.route_eth_to_noble().await;
            self.route_noble_to_neutron().await;

            // STEP 4. enter the position with funds available in neutron deposit acc
            self.enter_position().await;

            // STEP 5. TODO: exit the position with necessary amount of shares needed
            // to fulfill the withdraw obligations
            let eth_rp = self.eth_client.get_request_provider().await.unwrap();
            let valence_vault = ValenceVault::new(self.vault_addr, &eth_rp);

            let assets_to_withdraw = self
                .eth_client
                .query(valence_vault.totalAssetsToWithdrawNextUpdate())
                .await
                .unwrap()
                ._0;

            let usdc_to_withdraw_u128 = Uint128::from_str(&assets_to_withdraw.to_string()).unwrap();
            let halved_usdc_obligation_amt =
                usdc_to_withdraw_u128.checked_div(Uint128::new(2)).unwrap();

            info!(
                "ValenceVault assets_to_withdraw (USDC?): {:?}",
                assets_to_withdraw
            );

            let swap_simulation_output = self
                .reverse_simulate_swap(
                    &self.pool_addr,
                    NEUTRON_CHAIN_DENOM,
                    &self.uusdc_on_neutron_denom,
                    halved_usdc_obligation_amt,
                )
                .await
                .unwrap();

            info!(
                "swap simulation output to get {halved_usdc_obligation_amt}usdc: {:?}untrn",
                swap_simulation_output
            );

            // convert assets to shares
            //
            let shares_to_liquidate = self
                .simulate_provide_liquidity(
                    &self.pool_addr,
                    &self.uusdc_on_neutron_denom,
                    halved_usdc_obligation_amt,
                    NEUTRON_CHAIN_DENOM,
                    swap_simulation_output,
                )
                .await
                .unwrap();

            self.forward_shares_for_liquidation(shares_to_liquidate)
                .await;
            self.exit_position().await;

            i += 1;
            self.neutron_program_accounts
                .log_balances(
                    &self.neutron_client,
                    &self.noble_client,
                    vec![
                        self.uusdc_on_neutron_denom.to_string(),
                        NEUTRON_CHAIN_DENOM.to_string(),
                        self.lp_token_denom.to_string(),
                    ],
                )
                .await;
            self.eth_program_accounts
                .log_balances(
                    &self.eth_client,
                    &self.vault_addr,
                    &self.ethereum_usdc_erc20,
                )
                .await;
        }
    }

    pub(crate) async fn blocking_erc20_expected_balance_query(
        &self,
        addr: Address,
        min_amount: U256,
        interval_sec: u64,
        max_attempts: u32,
    ) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_sec));
        let eth_rp = self.eth_client.get_request_provider().await.unwrap();

        info!("EVM polling {addr} balance to exceed {min_amount}");

        let erc20 = MockERC20::new(self.ethereum_usdc_erc20, &eth_rp);

        for attempt in 1..max_attempts + 1 {
            interval.tick().await;

            match self.eth_client.query(erc20.balanceOf(addr)).await {
                Ok(balance) => {
                    let bal = balance._0;
                    if bal >= min_amount {
                        info!("balance exceeded!");
                        return;
                    } else {
                        info!(
                            "Balance polling attempt {attempt}/{max_attempts}: current={bal}, target={min_amount}"
                        );
                    }
                }
                Err(e) => warn!(
                    "Balance polling attempt {attempt}/{max_attempts} failed: {:?}",
                    e
                ),
            }
        }
    }

    async fn state_log(&self) {}
}
