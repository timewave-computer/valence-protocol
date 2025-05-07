use std::{cmp::max, error::Error, path::Path, str::FromStr};

use alloy::{
    primitives::{Address, U256},
    providers::Provider,
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use async_trait::async_trait;
use cosmwasm_std::{Decimal, Uint128};
use cosmwasm_std_old::Uint256;
use localic_utils::{NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_ID};
use log::{info, warn};

use valence_chain_client_utils::{
    cosmos::{base_client::BaseClient, wasm_client::WasmClient},
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
    neutron::NeutronClient,
    noble::NobleClient,
};
use valence_e2e::utils::{
    solidity_contracts::{MockERC20, ValenceVault},
    vault::time::{get_current_second, wait_until_next_minute},
    worker::{ValenceWorker, ValenceWorkerTomlSerde},
    NOBLE_CHAIN_DENOM,
};

use crate::strategist::{astroport::AstroportOps, routing::EthereumVaultRouting};

use super::strategy_config::StrategyConfig;

// main strategy struct that wraps around the StrategyConfig
// and stores the initialized clients
pub struct Strategy {
    pub cfg: StrategyConfig,

    pub(crate) eth_client: EthereumClient,
    pub(crate) noble_client: NobleClient,
    pub(crate) neutron_client: NeutronClient,
}

impl Strategy {
    // async constructor which initializes the clients baesd on the StrategyConfig
    pub async fn new(cfg: StrategyConfig) -> Result<Self, Box<dyn Error>> {
        let noble_client = NobleClient::new(
            &cfg.noble.grpc_url,
            &cfg.noble.grpc_port,
            &cfg.noble.mnemonic,
            &cfg.noble.chain_id,
            NOBLE_CHAIN_DENOM,
        )
        .await?;

        let neutron_client = NeutronClient::new(
            &cfg.neutron.grpc_url,
            &cfg.neutron.grpc_port,
            &cfg.neutron.mnemonic,
            NEUTRON_CHAIN_ID,
        )
        .await?;

        let eth_client = EthereumClient {
            rpc_url: cfg.ethereum.rpc_url.to_string(),
            signer: MnemonicBuilder::<English>::default()
                .phrase(cfg.ethereum.mnemonic.clone())
                .index(7)? // derive the mnemonic at a different index to avoid nonce issues
                .build()?,
        };

        Ok(Strategy {
            cfg,
            // store the initialized clients
            eth_client,
            noble_client,
            neutron_client,
        })
    }

    // initialization helper that parses StrategyConfig from a file and calls the
    // default constructor (`Strategy::new`)
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let strategy_cfg = StrategyConfig::from_file(path)?;
        Self::new(strategy_cfg).await
    }
}

// implement the ValenceWorker trait for the Strategy struct.
// This trait defines the main loop of the strategy and inherits
// the default implementation for spawning the worker.
#[async_trait]
impl ValenceWorker for Strategy {
    fn get_name(&self) -> String {
        "Valence X-Vault: ETH-NOBLE-NEUTRON".to_string()
    }

    async fn cycle(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let worker_name = self.get_name();
        info!("{worker_name}: Starting cycle...");
        info!("{worker_name}: Waiting until next minute...");
        wait_until_next_minute().await;
        let eth_block = self.eth_client.latest_block_height().await?;
        info!(
            "{worker_name}: worker loop started at second {} at evm block: {eth_block}",
            get_current_second()
        );

        let eth_vault_address = Address::from_str(&self.cfg.ethereum.libraries.valence_vault)?;
        let eth_usdc_address = Address::from_str(&self.cfg.ethereum.denoms.usdc_erc20)?;
        let eth_deposit_acc_address = Address::from_str(&self.cfg.ethereum.accounts.deposit)?;

        let eth_rp = self.eth_client.get_request_provider().await?;
        let valence_vault = ValenceVault::new(eth_vault_address, &eth_rp);
        let eth_usdc_erc20 = MockERC20::new(eth_usdc_address, &eth_rp);

        // ================ query epoch start vault state  ====================
        // 1. query the eth vault to for total obligations and the issued shares
        let assets_to_withdraw_response = self
            .eth_client
            .query(valence_vault.totalAssetsToWithdrawNextUpdate())
            .await?;
        let vault_issued_shares_response =
            self.eth_client.query(valence_vault.totalSupply()).await?;
        let vault_current_rate = self
            .eth_client
            .query(valence_vault.redemptionRate())
            .await?
            ._0;
        let vault_config = self.eth_client.query(valence_vault.config()).await?;

        // 2. query the eth deposit account to get the deposited tokens amount
        let deposit_acc_usdc_bal_response = self
            .eth_client
            .query(eth_usdc_erc20.balanceOf(eth_deposit_acc_address))
            .await?;

        // 3. query the neutron position account to get the currently held amount of shares
        let neutron_position_acc_shares = self
            .neutron_client
            .query_balance(
                &self.cfg.neutron.accounts.position,
                &self.cfg.neutron.denoms.lp_token,
            )
            .await?;

        // // 4. query the target pool configuration
        let cl_pool_cfg: valence_astroport_utils::astroport_native_lp_token::ConfigResponse = self
            .neutron_client
            .query_contract_state(
                &self.cfg.neutron.target_pool,
                valence_astroport_utils::astroport_native_lp_token::PoolQueryMsg::Config {},
            )
            .await
            .unwrap();
        let cl_pool_params = cl_pool_cfg.try_get_cl_params().unwrap();

        let pending_obligations_uint256 =
            Uint256::from_be_bytes(assets_to_withdraw_response._0.to_be_bytes());
        let eth_deposit_acc_usdc_bal =
            Uint256::from_be_bytes(deposit_acc_usdc_bal_response._0.to_be_bytes());

        let eth_deposit_acc_usdc_bal_u128 =
            Uint128::from_str(&eth_deposit_acc_usdc_bal.to_string())?;
        let vault_issued_shares =
            Uint256::from_be_bytes(vault_issued_shares_response._0.to_be_bytes());
        let vault_issued_shares_u128 = Uint128::from_str(&vault_issued_shares.to_string())?;

        info!("[CYCLE] vault issued shares Uint256: {vault_issued_shares}");
        info!("[CYCLE] vault issued shares Uint128: {vault_issued_shares_u128}");
        info!("[CYCLE] vault current rate: {vault_current_rate}");
        info!("[CYCLE] vault cfg: {:?}", vault_config);
        info!("[CYCLE] vault pending obligations: {pending_obligations_uint256}");
        info!("[CYCLE] eth deposit acc usdc Uint256: {eth_deposit_acc_usdc_bal}");
        info!("[CYCLE] eth deposit acc usdc Uint128: {eth_deposit_acc_usdc_bal_u128}");
        info!("[CYCLE] neutron position acc shares: {neutron_position_acc_shares}");
        info!("[CYCLE] target astroport pool params: {:?}", cl_pool_params);

        // ========================== netting =================================
        // 1. find the netting amount
        let netting_amount = pending_obligations_uint256.min(eth_deposit_acc_usdc_bal);
        let netting_amount_u128 = Uint128::from_str(&netting_amount.to_string())?;

        // 2. update the pending obligations to take netting into account
        let effective_pending_obligations =
            pending_obligations_uint256.checked_sub(netting_amount)?;

        info!("[CYCLE] netting amount Uint256: {netting_amount}");
        info!("[CYCLE] netting amount Uint128: {netting_amount_u128}");
        info!("[CYCLE] effective pending obligations: {effective_pending_obligations}");

        // =================== calculate withdraw amt =========================
        // 1. half the pending obligations to estimate the amount of neutron needed
        // to obtain it
        let halved_pending_obligations = effective_pending_obligations / Uint256::from_u128(2);
        let halved_pending_obligations_u128 =
            Uint128::from_str(&halved_pending_obligations.to_string())?;

        // 2. simulate the swap from untrn into the halved amount of usdc
        let expected_untrn_amount = self
            .reverse_simulate_swap(
                &self.cfg.neutron.target_pool.to_string(),
                NEUTRON_CHAIN_DENOM,
                &self.cfg.neutron.denoms.usdc,
                halved_pending_obligations_u128,
            )
            .await
            .unwrap();

        // 3. simulate liquidity provision with the 1/2 usdc amount and the equivalent untrn amount.
        // this will give us the amount of shares that are equivalent to those tokens.
        let shares_to_liquidate = self
            .simulate_provide_liquidity(
                &self.cfg.neutron.target_pool,
                &self.cfg.neutron.denoms.usdc,
                halved_pending_obligations_u128,
                NEUTRON_CHAIN_DENOM,
                expected_untrn_amount,
            )
            .await
            .unwrap();

        info!("[CYCLE] shares to liquidate: {shares_to_liquidate}");

        // =================== calculate total assets =========================
        // 1. subtract the shares to be liquidated in order to fulfill the withdraw
        // obligations from the neutron position account shares balance to get the
        // effective shares balance
        let effective_position_shares =
            Uint128::from(neutron_position_acc_shares).checked_sub(shares_to_liquidate)?;

        info!("[CYCLE] effective position shares: {effective_position_shares}");

        // 2. simulate the effective shares liquidation to get the equivalent
        // untrn + usdc balances
        let (position_usdc_amount, position_ntrn_amount) = self
            .simulate_liquidation(
                &self.cfg.neutron.target_pool,
                effective_position_shares.u128(),
                &self.cfg.neutron.denoms.usdc,
                NEUTRON_CHAIN_DENOM,
            )
            .await
            .unwrap();

        // 3. simulate the resulting liquidation untrn -> usdc swap
        let ntrn_to_usdc_swap_simulation_output = self
            .simulate_swap(
                &self.cfg.neutron.target_pool,
                NEUTRON_CHAIN_DENOM,
                position_ntrn_amount,
                &self.cfg.neutron.denoms.usdc,
            )
            .await
            .unwrap();

        // 4. get the total position usdc value
        let total_active_position_usdc = position_usdc_amount + ntrn_to_usdc_swap_simulation_output;

        info!("[CYCLE] total active position usdc: {total_active_position_usdc}");

        // 5. total effective vault assets is equal to the position account value plus
        // the pending deposits minus the amount to be netted
        let total_effective_assets =
            total_active_position_usdc + eth_deposit_acc_usdc_bal_u128 - netting_amount_u128;

        info!("[CYCLE] total effective assets: {total_effective_assets}");

        // =============== calculate the redemption rate ======================
        // rate =  effective_total_assets / (effective_vault_shares * scaling_factor)
        let redemption_rate = Decimal::from_ratio(
            total_effective_assets,
            // multiplying the denominator by the scaling factor
            vault_issued_shares_u128.checked_mul(1_000_000_000_000u128.into())?,
        );

        info!("[CYCLE] redemption rate  {total_effective_assets}usdc / {vault_issued_shares_u128}shares = {redemption_rate}");

        let r = U256::from(redemption_rate.atomics().u128());
        info!("[CYCLE] r = {r}");

        // ====================== update the vault ============================

        // for simplicity taking the max between the two fees for now
        let pool_fee_decimal = max(cl_pool_params.mid_fee, cl_pool_params.out_fee);

        let scaled_pool_fee = pool_fee_decimal * Decimal::from_ratio(10_000u128, 1u128);

        let fee_bps = scaled_pool_fee.to_string().parse::<u32>().unwrap_or(100);
        info!("[CYCLE] bps_converted: {fee_bps}");

        // all withdraws are subject to a base fee buffer of 0.01%
        let fee_buffer = 1u32;

        // TODO: remove the subtraction for mainnet deployments
        let total_fee = fee_bps + fee_buffer - fee_bps;

        info!(
            "[CYCLE] Updating Ethereum Vault with:
                rate: {r}
                witdraw_fee_bps: {total_fee}
                netting_amount: {netting_amount}"
        );

        let update_result = self
            .eth_client
            .execute_tx(
                valence_vault
                    .update(
                        r,
                        total_fee,
                        U256::from_be_bytes(netting_amount.to_be_bytes()),
                    )
                    .into_transaction_request(),
            )
            .await?;
        eth_rp
            .get_transaction_receipt(update_result.transaction_hash)
            .await?;

        // ====================================================================

        // ================== route the funds eth->ntrn =======================
        //   1. cctp transfer eth deposit acc -> noble inbound ica
        self.route_eth_to_noble().await;

        //   2. ica ibc transfer noble inbound ica -> neutron deposit acc
        self.route_noble_to_neutron().await;
        // ====================================================================

        // ======================= enter the position =========================
        // funds should already be in the deposit account so we are ready to
        // provide them into the LP
        match self.enter_position().await {
            Ok(_) => (),
            Err(e) => warn!("error entering position: {:?}", e),
        };
        // ====================================================================

        // ======================= exit the position ==========================
        // 1. forward the shares to be liquidated from the position account to the withdraw account
        self.forward_shares_for_liquidation(shares_to_liquidate)
            .await;

        // 2. liquidate the forwarded shares to get USDC+NTRN
        match self.exit_position().await {
            Ok(_) => (),
            Err(e) => warn!("error exiting position: {:?}", e),
        };

        // 3. swap NTRN into USDC to obtain the full obligation amount
        match self.swap_ntrn_into_usdc().await {
            Ok(_) => (),
            Err(e) => warn!("error swapping ntrn into usdc: {:?}", e),
        };

        // ====================================================================

        // ================== route the funds ntrn->eth =======================
        //   1. ibc transfer neutron withdraw acc -> noble outbound ica
        self.route_neutron_to_noble().await;

        //   2. cctp transfer noble outbound ica -> eth withdraw acc
        self.route_noble_to_eth().await;
        // ====================================================================

        info!(
            "strategist loop completed at second {}",
            get_current_second()
        );

        Ok(())
    }
}
