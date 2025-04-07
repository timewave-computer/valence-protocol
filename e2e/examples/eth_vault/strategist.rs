use std::{error::Error, str::FromStr};

use alloy::{
    primitives::{Address, U256},
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use cosmwasm_std::{to_json_binary, CosmosMsg, Decimal, Uint128, WasmMsg};
use localic_utils::{NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_ID};
use log::{info, warn};
use tokio::runtime::Runtime;
use valence_astroport_utils::{
    astroport_cw20_lp_token::SimulationResponse,
    astroport_native_lp_token::{self, Asset, AssetInfo},
};
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
        solidity_contracts::{CCTPTransfer, MockERC20, ValenceVault},
        ADMIN_MNEMONIC, DEFAULT_ANVIL_RPC_ENDPOINT, NOBLE_CHAIN_DENOM, NOBLE_CHAIN_ID, UUSDC_DENOM,
    },
};

use crate::{
    evm::EthereumProgramAccounts,
    program::{NeutronProgramAccounts, NeutronProgramLibraries},
};

pub struct Strategist {
    eth_client: EthereumClient,
    noble_client: NobleClient,
    neutron_client: NeutronClient,
    neutron_program_accounts: NeutronProgramAccounts,
    neutron_program_libraries: NeutronProgramLibraries,
    eth_program_accounts: EthereumProgramAccounts,
    uusdc_on_neutron_denom: String,
    lp_token_denom: String,
    pool_addr: String,
    cctp_transfer_lib: Address,
    vault_addr: Address,
    ethereum_usdc_erc20: Address,
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

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        let mut i = 0;
        loop {
            info!("[STRATEGIST] loop #{i}, sleeping for 5sec...");
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
            info!("[STRATEGIST] returned redemption rate: {redemption_rate}");
            let r = U256::from(redemption_rate.atomics().u128());
            info!("[STRATEGIST] vault converted redemption rate: {r}");
            match self
                .vault_update(r, total_fee, U256::from(netting_amount))
                .await
            {
                Ok(resp) => {
                    info!("[STRATEGIST] vault update response: {:?}", resp);
                }
                Err(err) => warn!("[STRATEGIST] vault update error: {:?}", err),
            };
            // 4. Update the Vault with R, F_total, N

            // STEP 3. pulling funds due for deposit from origin to position domain
            //   1. cctp transfer eth deposit acc -> noble inbound ica
            //   2. ica ibc transfer noble inbound ica -> neutron deposit acc

            // STEP 4. enter the position with funds available in neutron deposit acc

            // STEP 5. exit the

            i += 1;
        }
    }

    async fn simulate_liquidation(
        &self,
        pool_addr: &str,
        shares_amount: u128,
        denom_1: &str,
        denom_2: &str,
    ) -> Result<(Uint128, Uint128), Box<dyn Error>> {
        if shares_amount == 0 {
            info!("[STRATEGIST] shares amount is zero, skipping withdraw liquidation simulation");
            return Ok((Uint128::zero(), Uint128::zero()));
        }

        let share_liquidation_response: Vec<Asset> = self
            .neutron_client
            .query_contract_state(
                pool_addr,
                astroport_native_lp_token::PoolQueryMsg::Share {
                    amount: Uint128::from(shares_amount),
                },
            )
            .await
            .unwrap();

        let output_coins: Vec<cosmwasm_std::Coin> = share_liquidation_response
            .iter()
            .map(|c| c.as_coin().unwrap())
            .collect();

        info!(
            "[STRATEGIST] Share liquidation for {shares_amount} on the pool respnose: {:?}",
            output_coins
        );

        let coin_1 = output_coins.iter().find(|c| c.denom == denom_1).unwrap();
        let coin_2 = output_coins.iter().find(|c| c.denom == denom_2).unwrap();

        Ok((coin_1.amount, coin_2.amount))
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
        info!("[STRATEGIST] CL POOL CONFIG: {:?}", cl_pool_cfg);

        let pool_fee = match cl_pool_cfg.try_get_cl_params() {
            Some(cl_params) => {
                info!("[STRATEGIST] CL POOL PARAMS: {:?}", cl_params);
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

    async fn simulate_swap(
        &self,
        pool_addr: &str,
        offer_denom: &str,
        offer_amount: Uint128,
        ask_denom: &str,
    ) -> Result<Uint128, Box<dyn Error>> {
        if offer_amount == Uint128::zero() {
            info!("[STRATEGIST] offer amount is zero, skipping swap simulation");
            return Ok(Uint128::zero());
        }

        let share_liquidation_response: SimulationResponse = self
            .neutron_client
            .query_contract_state(
                pool_addr,
                astroport_native_lp_token::PoolQueryMsg::Simulation {
                    offer_asset: Asset {
                        info: AssetInfo::NativeToken {
                            denom: offer_denom.to_string(),
                        },
                        amount: offer_amount,
                    },
                    ask_asset_info: Some(AssetInfo::NativeToken {
                        denom: ask_denom.to_string(),
                    }),
                },
            )
            .await
            .unwrap();

        info!("[STRATEGIST] swap simulation of {offer_amount}{offer_denom} -> {ask_denom} response: {:?}", share_liquidation_response);

        Ok(share_liquidation_response.return_amount)
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

    /// IBC-transfers funds from noble inbound ica into neutron deposit account
    pub async fn route_noble_to_neutron(&self, transfer_amount: u128) {
        let noble_inbound_ica_balance = self
            .noble_client
            .query_balance(
                &self.neutron_program_accounts.noble_inbound_ica.remote_addr,
                UUSDC_DENOM,
            )
            .await
            .unwrap();

        if noble_inbound_ica_balance < transfer_amount {
            warn!("Noble inbound ICA account must have enough USDC to route funds to Neutron deposit acc; returning");
            return;
        }

        let init_bal = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .deposit_account
                    .to_string()
                    .unwrap(),
                &self.uusdc_on_neutron_denom,
            )
            .await
            .unwrap();

        let transfer_msg = &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
            valence_ica_ibc_transfer::msg::FunctionMsgs::Transfer {},
        );
        let rx = self
            .neutron_client
            .execute_wasm(
                &self.neutron_program_libraries.noble_inbound_transfer,
                transfer_msg,
                vec![],
            )
            .await
            .unwrap();
        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        info!("starting polling assertion on the destination...");
        self.neutron_client
            .poll_until_expected_balance(
                &self
                    .neutron_program_accounts
                    .deposit_account
                    .to_string()
                    .unwrap(),
                &self.uusdc_on_neutron_denom,
                init_bal + transfer_amount,
                1,
                10,
            )
            .await
            .unwrap();
    }

    /// IBC-transfers funds from Neutron withdraw account to noble outbound ica
    pub async fn route_neutron_to_noble(&self) {
        let noble_outbound_ica_usdc_bal = self
            .noble_client
            .query_balance(
                &self.neutron_program_accounts.noble_outbound_ica.remote_addr,
                UUSDC_DENOM,
            )
            .await
            .unwrap();
        let withdraw_account_usdc_bal = self
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

        let withdraw_account_ntrn_bal = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                NEUTRON_CHAIN_DENOM,
            )
            .await
            .unwrap();

        if withdraw_account_usdc_bal == 0 {
            warn!("[STRATEGIST] withdraw account must have USDC in order to route funds to noble; returning");
            return;
        }

        // if withdraw_account_ntrn_bal == 0 {
        //     warn!("[STRATEGIST] withdraw account must have NTRN in order to route funds to noble; returning");
        //     return;
        // }

        info!("[STRATEGIST] routing USDC to noble...");
        let transfer_rx = self
            .neutron_client
            .transfer(
                &self
                    .neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                110_000,
                NEUTRON_CHAIN_DENOM,
                None,
            )
            .await
            .unwrap();
        self.neutron_client
            .poll_for_tx(&transfer_rx.hash)
            .await
            .unwrap();

        let neutron_ibc_transfer_msg =
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_neutron_ibc_transfer_library::msg::FunctionMsgs::IbcTransfer {},
            );
        let rx = self
            .neutron_client
            .execute_wasm(
                &self.neutron_program_libraries.neutron_ibc_transfer,
                neutron_ibc_transfer_msg,
                vec![],
            )
            .await
            .unwrap();
        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        info!("starting polling assertion on noble outbound ica...");
        self.noble_client
            .poll_until_expected_balance(
                &self
                    .neutron_program_accounts
                    .noble_outbound_ica
                    .remote_addr
                    .to_string(),
                UUSDC_DENOM,
                noble_outbound_ica_usdc_bal + withdraw_account_usdc_bal,
                1,
                10,
            )
            .await
            .unwrap();
    }

    /// CCTP-transfers funds from Ethereum deposit account to Noble inbound ica
    pub async fn route_eth_to_noble(&self) {
        info!("[STRATEGIST] CCTP forwarding USDC from Ethereum to Noble...");
        let eth_rp = self.eth_client.get_request_provider().await.unwrap();

        let cctp_transfer_contract = CCTPTransfer::new(self.cctp_transfer_lib, &eth_rp);

        let pre_cctp_inbound_ica_usdc_bal = self
            .noble_client
            .query_balance(
                &self.neutron_program_accounts.noble_inbound_ica.remote_addr,
                UUSDC_DENOM,
            )
            .await
            .unwrap();

        let signer_addr = self.eth_client.signer.address();
        let signed_tx = cctp_transfer_contract
            .transfer()
            .into_transaction_request()
            .from(signer_addr);

        let cctp_transfer_rx = self.eth_client.execute_tx(signed_tx).await.unwrap();

        info!(
            "cctp transfer tx hash: {:?}",
            cctp_transfer_rx.transaction_hash
        );

        let remote_ica_addr = self
            .neutron_program_accounts
            .noble_inbound_ica
            .remote_addr
            .to_string();

        info!("starting polling assertion on the destination...");
        self.noble_client
            .poll_until_expected_balance(
                &remote_ica_addr,
                UUSDC_DENOM,
                pre_cctp_inbound_ica_usdc_bal + 1,
                1,
                10,
            )
            .await
            .unwrap();
    }

    /// CCTP-transfers funds from Noble outbound ica to Ethereum withdraw account
    pub async fn route_noble_to_eth(&self) {
        info!("[STRATEGIST] CCTP forwarding USDC from Noble to Ethereum...");

        let eth_rp = self.eth_client.get_request_provider().await.unwrap();
        let erc20 = MockERC20::new(self.ethereum_usdc_erc20, &eth_rp);
        let pre_cctp_ethereum_withdraw_acc_usdc_bal = self
            .eth_client
            .query(erc20.balanceOf(self.eth_program_accounts.withdraw))
            .await
            .unwrap()
            ._0;
        let pre_cctp_noble_outbound_ica_usdc_bal = self
            .noble_client
            .query_balance(
                &self.neutron_program_accounts.noble_outbound_ica.remote_addr,
                UUSDC_DENOM,
            )
            .await
            .unwrap();

        if pre_cctp_noble_outbound_ica_usdc_bal == 0 {
            warn!("[STRATEGIST] Noble outbound ICA account must have USDC in order to CCTP forward to Ethereum; returning");
            return;
        }

        let transfer_tx = self
            .neutron_client
            .transfer(
                &self
                    .neutron_program_accounts
                    .noble_outbound_ica
                    .library_account
                    .to_string()
                    .unwrap(),
                110_000,
                NEUTRON_CHAIN_DENOM,
                None,
            )
            .await
            .unwrap();
        self.neutron_client
            .poll_for_tx(&transfer_tx.hash)
            .await
            .unwrap();

        let neutron_ica_cctp_transfer_msg =
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_ica_cctp_transfer::msg::FunctionMsgs::Transfer {},
            );
        let rx = self
            .neutron_client
            .execute_wasm(
                &self.neutron_program_libraries.noble_cctp_transfer,
                neutron_ica_cctp_transfer_msg,
                vec![],
            )
            .await
            .unwrap();
        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        self.blocking_erc20_expected_balance_query(
            self.eth_program_accounts.withdraw,
            pre_cctp_ethereum_withdraw_acc_usdc_bal + U256::from(1),
            1,
            10,
        )
        .await;
    }

    async fn blocking_erc20_expected_balance_query(
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

    /// enters the position on astroport
    pub async fn enter_position(&self) {
        info!("[STRATEGIST] entering LP position...");
        let deposit_account_usdc_bal = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .deposit_account
                    .to_string()
                    .unwrap(),
                &self.uusdc_on_neutron_denom,
            )
            .await
            .unwrap();

        if deposit_account_usdc_bal == 0 {
            warn!("[STRATEGIST] Deposit account must have USDC in order to LP; returning");
            return;
        }

        let provide_liquidity_msg =
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_astroport_lper::msg::FunctionMsgs::ProvideSingleSidedLiquidity {
                    asset: self.uusdc_on_neutron_denom.to_string(),
                    limit: None,
                    expected_pool_ratio_range: None,
                },
            );
        let rx = self
            .neutron_client
            .execute_wasm(
                &self.neutron_program_libraries.astroport_lper,
                provide_liquidity_msg,
                vec![],
            )
            .await
            .unwrap();
        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        let output_acc_bal = self
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
        info!("position account LP token balance: {output_acc_bal}");
        assert_ne!(0, output_acc_bal);
    }

    /// exits the position on astroport
    pub async fn exit_position(&self) {
        info!("[STRATEGIST] exiting LP position...");

        let position_account_shares_bal = self
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

        if position_account_shares_bal == 0 {
            warn!(
                "[STRATEGIST] Position account must have LP shares in order to exit LP; returning"
            );
            return;
        }

        let withdraw_liquidity_msg =
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_astroport_withdrawer::msg::FunctionMsgs::WithdrawLiquidity {
                    expected_pool_ratio_range: None,
                },
            );
        let rx = self
            .neutron_client
            .execute_wasm(
                &self.neutron_program_libraries.astroport_lwer,
                withdraw_liquidity_msg,
                vec![],
            )
            .await
            .unwrap();
        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        let withdraw_acc_usdc_bal = self
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
        let withdraw_acc_ntrn_bal = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                NEUTRON_CHAIN_DENOM,
            )
            .await
            .unwrap();
        info!("withdraw account USDC token balance: {withdraw_acc_usdc_bal}",);
        info!("withdraw account NTRN token balance: {withdraw_acc_ntrn_bal}",);
        assert_ne!(0, withdraw_acc_usdc_bal);
        assert_ne!(0, withdraw_acc_ntrn_bal);
    }

    /// swaps counterparty denom into usdc
    pub async fn swap_ntrn_into_usdc(&self) {
        info!("[STRATEGIST] swapping NTRN into USDC...");
        let withdraw_account_ntrn_bal = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                NEUTRON_CHAIN_DENOM,
            )
            .await
            .unwrap();

        if withdraw_account_ntrn_bal <= 1_000_000 {
            warn!("[STRATEGIST] Withdraw account must have NTRN in order to swap into USDC; returning");
            return;
        }

        let swap_amount = withdraw_account_ntrn_bal - 1_000_000;

        let swap_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.pool_addr.to_string(),
            msg: to_json_binary(
                &valence_astroport_utils::astroport_native_lp_token::ExecuteMsg::Swap {
                    offer_asset: Asset {
                        info: AssetInfo::NativeToken {
                            denom: NEUTRON_CHAIN_DENOM.to_string(),
                        },
                        amount: swap_amount.into(),
                    },
                    max_spread: None,
                    belief_price: None,
                    to: None,
                    ask_asset_info: None,
                },
            )
            .unwrap(),
            funds: vec![cosmwasm_std::coin(
                swap_amount,
                NEUTRON_CHAIN_DENOM.to_string(),
            )],
        });

        let base_account_execute_msgs = valence_account_utils::msg::ExecuteMsg::ExecuteMsg {
            msgs: vec![swap_msg],
        };

        let rx = self
            .neutron_client
            .execute_wasm(
                &self
                    .neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                base_account_execute_msgs,
                vec![],
            )
            .await
            .unwrap();

        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        let withdraw_acc_usdc_bal = self
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
        let withdraw_acc_ntrn_bal = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                NEUTRON_CHAIN_DENOM,
            )
            .await
            .unwrap();
        info!("withdraw account USDC token balance: {withdraw_acc_usdc_bal}",);
        info!("withdraw account NTRN token balance: {withdraw_acc_ntrn_bal}",);
        assert_ne!(0, withdraw_acc_usdc_bal);
        assert_eq!(
            1_000_000, withdraw_acc_ntrn_bal,
            "neutron withdraw account should have 1_000_000untrn left to cover ibc transfer fees"
        );
    }
}
