use std::{error::Error, str::FromStr};

use alloy::{
    primitives::{Address, U256},
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use cosmwasm_std::{Decimal, Uint128};
use localic_utils::{NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_ID};
use log::{info, warn};
use tokio::runtime::Runtime;
use valence_astroport_utils::astroport_native_lp_token::{self};
use valence_chain_client_utils::{
    cosmos::{base_client::BaseClient, wasm_client::WasmClient},
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
        ADMIN_MNEMONIC, DEFAULT_ANVIL_RPC_ENDPOINT, NOBLE_CHAIN_DENOM, NOBLE_CHAIN_ID, UUSDC_DENOM,
    },
};
use valence_forwarder_library::msg::UncheckedForwardingConfig;

use crate::{
    evm::EthereumProgramAccounts,
    program::{NeutronProgramAccounts, NeutronProgramLibraries},
    strategist::{astroport::AstroportOps, bridge::EthereumVaultBridging},
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
        info!("[STRATEGIST] Starting...");

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(15));
        let mut i = 0;
        loop {
            info!("[STRATEGIST] loop #{i}");
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
                info!("[STRATEGIST] Neutron withdraw account USDC balance greater than 0!\nRouting from position to origin chain.");
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
                    info!("[STRATEGIST] vault update response: {:?}", resp);
                }
                Err(err) => warn!("[STRATEGIST] vault update error: {:?}", err),
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
                "[STRATEGIST] ValenceVault assets_to_withdraw (USDC?): {:?}",
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
                "[STRATEGIST] swap simulation output to get {halved_usdc_obligation_amt}usdc: {:?}untrn",
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

    /// calculates the amount of shares that need to be liquidated to fulfill all
    /// pending withdraw obligations and forwards those shares from the position
    /// account to the withdrawal account.
    async fn forward_shares_for_liquidation(&self, amount: Uint128) {
        if amount.is_zero() {
            info!("[STRATEGIST] zero-shares liquidation request; returning");
            return;
        }

        let new_fwd_cfgs = vec![UncheckedForwardingConfig {
            denom: valence_library_utils::denoms::UncheckedDenom::Native(
                self.lp_token_denom.to_string(),
            ),
            max_amount: amount,
        }];

        info!(
            "[STRATEGIST] updating liquidation forwarder cfg to: {:?}",
            new_fwd_cfgs
        );

        let update_cfg_msg = &valence_library_utils::msg::ExecuteMsg::<
            valence_forwarder_library::msg::FunctionMsgs,
            valence_forwarder_library::msg::LibraryConfigUpdate,
        >::UpdateConfig {
            new_config: valence_forwarder_library::msg::LibraryConfigUpdate {
                input_addr: None,
                output_addr: None,
                forwarding_configs: Some(new_fwd_cfgs),
                forwarding_constraints: None,
            },
        };

        let update_rx = self
            .neutron_client
            .execute_wasm(
                &self.neutron_program_libraries.liquidation_forwarder,
                update_cfg_msg,
                vec![],
            )
            .await
            .unwrap();

        self.neutron_client
            .poll_for_tx(&update_rx.hash)
            .await
            .unwrap();

        info!("[STRATEGIST] update cfg complete; executing forwarding");

        let pre_fwd_position = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .position_account
                    .to_string()
                    .unwrap(),
                &self.lp_token_denom,
            )
            .await
            .unwrap();

        info!(
            "[STRATEGIST] pre forward position account shares balance: {:?}",
            pre_fwd_position
        );

        let fwd_msg = &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
            valence_forwarder_library::msg::FunctionMsgs::Forward {},
        );
        let rx = self
            .neutron_client
            .execute_wasm(
                &self.neutron_program_libraries.liquidation_forwarder,
                fwd_msg,
                vec![],
            )
            .await
            .unwrap();
        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        let post_fwd_position = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .position_account
                    .to_string()
                    .unwrap(),
                &self.lp_token_denom,
            )
            .await
            .unwrap();

        info!(
            "[STRATEGIST] post forward position account shares balance: {:?}",
            post_fwd_position
        );

        info!("[STRATEGIST] fwd complete!");
    }

    async fn calculate_netting_amount(&self) -> Result<u32, Box<dyn Error>> {
        // 3. Find netting amount N
        //   1. query Vault for total pending withdrawals (USDC)
        //   2. query Eth deposit account for USDC balance
        //   3. N = min(deposit_bal, withdrawals_sum)
        Ok(0)
    }

    async fn calculate_total_fee(&self) -> Result<u32, Box<dyn Error>> {
        // 2. Find withdraw fee F_total
        //   1. query Vault fee from the Eth vault
        //   2. query the dex position for their fee
        //   3. F_total = F_vault + F_position
        let eth_rp = self.eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(self.vault_addr, &eth_rp);

        let fees = self
            .eth_client
            .query(valence_vault.config())
            .await
            .unwrap()
            .fees;

        info!("[STRATEGIST] vault fees: {:?}", fees);

        let pool_addr = self.pool_addr.to_string();
        let cl_pool_cfg: astroport_native_lp_token::ConfigResponse = self
            .neutron_client
            .query_contract_state(
                &pool_addr,
                astroport_native_lp_token::PoolQueryMsg::Config {},
            )
            .await
            .unwrap();
        // info!("[STRATEGIST] CL POOL CONFIG: {:?}", cl_pool_cfg);

        let pool_fee = match cl_pool_cfg.try_get_cl_params() {
            Some(cl_params) => {
                // info!("[STRATEGIST] CL POOL PARAMS: {:?}", cl_params);
                (cl_params.out_fee * Decimal::from_ratio(10000u128, 1u128))
                    .atomics()
                    .u128() as u32 // intentionally truncating
            }
            None => 0u32,
        };

        Ok(fees.platformFeeBps + pool_fee)
    }

    async fn calculate_redemption_rate(&self) -> Result<Decimal, Box<dyn Error>> {
        let eth_rp = self.eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(self.vault_addr, &eth_rp);
        let eth_usdc_erc20 = MockERC20::new(self.ethereum_usdc_erc20, &eth_rp);

        let neutron_position_acc = self
            .neutron_program_accounts
            .position_account
            .to_string()
            .unwrap();
        let noble_inbound_ica = self
            .neutron_program_accounts
            .noble_inbound_ica
            .remote_addr
            .to_string();
        let neutron_deposit_acc = self
            .neutron_program_accounts
            .deposit_account
            .to_string()
            .unwrap();
        let eth_deposit_acc = self.eth_program_accounts.deposit;

        // 1. query total shares issued from the vault
        let vault_issued_shares = self
            .eth_client
            .query(valence_vault.totalSupply())
            .await
            .unwrap()
            ._0;
        let vault_current_rate = self
            .eth_client
            .query(valence_vault.redemptionRate())
            .await
            .unwrap()
            ._0;
        info!(
            "[STRATEGIST] current vault redemption rate: {:?}",
            vault_current_rate
        );

        // 2. query shares in position account and simulate their liquidation for USDC
        let neutron_position_acc_shares = self
            .neutron_client
            .query_balance(&neutron_position_acc, &self.lp_token_denom)
            .await
            .unwrap();
        let (usdc_amount, ntrn_amount) = self
            .simulate_liquidation(
                &self.pool_addr,
                neutron_position_acc_shares,
                &self.uusdc_on_neutron_denom,
                NEUTRON_CHAIN_DENOM,
            )
            .await
            .unwrap();

        let swap_simulation_output = self
            .simulate_swap(
                &self.pool_addr,
                NEUTRON_CHAIN_DENOM,
                ntrn_amount,
                &self.uusdc_on_neutron_denom,
            )
            .await
            .unwrap();

        // 3. query pending deposits (eth deposit acc + noble inbound ica + neutron deposit acc)

        let eth_deposit_usdc = self
            .eth_client
            .query(eth_usdc_erc20.balanceOf(eth_deposit_acc))
            .await
            .unwrap()
            ._0;
        let noble_inbound_ica_usdc = self
            .noble_client
            .query_balance(&noble_inbound_ica, UUSDC_DENOM)
            .await
            .unwrap();
        let neutron_deposit_acc_usdc = self
            .neutron_client
            .query_balance(&neutron_deposit_acc, &self.uusdc_on_neutron_denom)
            .await
            .unwrap();

        //   4. R = total_shares / total_assets
        let normalized_eth_balance = Uint128::from_str(&eth_deposit_usdc.to_string()).unwrap();

        let total_assets = noble_inbound_ica_usdc
            + neutron_deposit_acc_usdc
            + normalized_eth_balance.u128()
            + usdc_amount.u128()
            + swap_simulation_output.u128();
        let normalized_shares = Uint128::from_str(&vault_issued_shares.to_string()).unwrap();

        info!("[STRATEGIST] total assets: {}USDC", total_assets);
        info!("[STRATEGIST] total shares: {}", normalized_shares.u128());
        match Decimal::checked_from_ratio(normalized_shares, total_assets) {
            Ok(ratio) => {
                info!("[STRATEGIST] redemption rate: {}", ratio);
                Ok(ratio)
            }
            Err(_) => Ok(Decimal::zero()),
        }
    }

    /// concludes the vault epoch and updates the Valence Vault state
    pub async fn vault_update(
        &self,
        rate: U256,
        withdraw_fee_bps: u32,
        netting_amount: U256,
    ) -> Result<(), Box<dyn Error>> {
        info!(
            "[STRATEGIST] Updating Ethereum Vault with:
            \nrate: {rate}
            \nwitdraw_fee_bps: {withdraw_fee_bps}
            \nnetting_amount: {netting_amount}"
        );
        let eth_rp = self.eth_client.get_request_provider().await.unwrap();

        let clamped_withdraw_fee = withdraw_fee_bps.clamp(1, 10_000);

        let valence_vault = ValenceVault::new(self.vault_addr, &eth_rp);

        let update_msg = valence_vault
            .update(rate, clamped_withdraw_fee, netting_amount)
            .into_transaction_request();

        let update_result = self.eth_client.execute_tx(update_msg).await;

        if let Err(e) = &update_result {
            info!("Update failed: {:?}", e);
            panic!("failed to update vault");
        }

        Ok(())
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
