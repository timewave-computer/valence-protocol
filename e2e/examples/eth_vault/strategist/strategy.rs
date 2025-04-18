use std::{error::Error, path::Path, str::FromStr};

use alloy::{
    primitives::{Address, U256},
    providers::Provider,
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use async_trait::async_trait;
use cosmwasm_std::Uint128;
use localic_utils::{NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_ID};
use log::{info, warn};

use valence_chain_client_utils::{
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
    neutron::NeutronClient,
    noble::NobleClient,
};
use valence_e2e::utils::{
    solidity_contracts::{MockERC20, ValenceVault},
    worker::{ValenceWorker, ValenceWorkerTomlSerde},
    NOBLE_CHAIN_DENOM,
};

use crate::{
    strategist::{astroport::AstroportOps, routing::EthereumVaultRouting, vault::EthereumVault},
    utils::{get_current_second, wait_until_next_minute},
};

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
        .await
        .expect("failed to create noble client");

        let neutron_client = NeutronClient::new(
            &cfg.neutron.grpc_url,
            &cfg.neutron.grpc_port,
            &cfg.neutron.mnemonic,
            NEUTRON_CHAIN_ID,
        )
        .await
        .expect("failed to create neutron client");

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

    // e2e test helper to parse the StrategyConfig from a file
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
        info!(
            "{worker_name}: worker loop started at second {}",
            get_current_second()
        );

        let eth_rp = self.eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(
            Address::from_str(&self.cfg.ethereum.libraries.valence_vault).unwrap(),
            &eth_rp,
        );
        let eth_usdc_erc20 = MockERC20::new(
            Address::from_str(&self.cfg.ethereum.denoms.usdc_erc20).unwrap(),
            &eth_rp,
        );

        // 1. calculate the amount of usdc needed to fulfill
        // the active withdraw obligations
        let pending_obligations = self
            .eth_client
            .query(valence_vault.totalAssetsToWithdrawNextUpdate())
            .await
            .unwrap()
            ._0;

        // 2. query ethereum program accounts for their usdc balances
        let eth_deposit_acc_usdc_bal = self
            .eth_client
            .query(
                eth_usdc_erc20
                    .balanceOf(Address::from_str(&self.cfg.ethereum.accounts.deposit).unwrap()),
            )
            .await
            .unwrap()
            ._0;

        // 3. see if pending obligations can be netted and update the pending
        // obligations accordingly
        let netting_amount = pending_obligations.min(eth_deposit_acc_usdc_bal);
        info!("netting amount: {netting_amount}");

        let pending_obligations = pending_obligations.checked_sub(netting_amount).unwrap();
        info!("updated pending obligations: {pending_obligations}");

        // 4. lp shares to be liquidated will yield untrn+uusdc. to figure out
        // the amount of ntrn needed to get 1/2 of the obligations, we half the
        // usdc amount
        let missing_usdc_amount: u128 = pending_obligations
            .try_into()
            .map_err(|_| "Pending obligations U256 Value too large for u128")
            .unwrap();
        info!("total to withdraw: {missing_usdc_amount}USDC");

        let halved_usdc_obligation_amt = Uint128::new(missing_usdc_amount / 2);
        info!("halved usdc obligation amount: {halved_usdc_obligation_amt}");

        // 5. simulate how many untrn we need to obtain half of the
        // missing usdc obligation amount
        let expected_untrn_amount = self
            .reverse_simulate_swap(
                &self.cfg.neutron.target_pool.to_string(),
                NEUTRON_CHAIN_DENOM,
                &self.cfg.neutron.denoms.usdc,
                halved_usdc_obligation_amt,
            )
            .await
            .unwrap();
        info!("reverse swap simulation response: {expected_untrn_amount}untrn => {halved_usdc_obligation_amt}usdc");

        // 6. simulate liquidity provision with the 1/2 usdc amount and the equivalent untrn amount.
        // this will give us the amount of shares that are equivalent to those tokens.
        // TODO: think if this simulation makes sense here as the order is reversed.
        let shares_to_liquidate = self
            .simulate_provide_liquidity(
                &self.cfg.neutron.target_pool,
                &self.cfg.neutron.denoms.usdc,
                halved_usdc_obligation_amt,
                NEUTRON_CHAIN_DENOM,
                expected_untrn_amount,
            )
            .await
            .unwrap();

        // 7. forward the shares to be liquidated from the position account to the withdraw account
        self.forward_shares_for_liquidation(shares_to_liquidate)
            .await;

        // 8. liquidate the forwarded shares to get USDC+NTRN
        self.exit_position().await;

        // 9. swap NTRN into USDC to obtain the full obligation amount
        self.swap_ntrn_into_usdc().await;

        // 10. update the vault to conclude the previous epoch. we already derived
        // the netting amount in step #3, so we need to find the redemption rate and
        // total fee.
        let redemption_rate = self.calculate_redemption_rate().await.unwrap();
        let total_fee = self.calculate_total_fee().await.unwrap();
        let r = U256::from(redemption_rate.atomics().u128());

        let clamped_withdraw_fee = total_fee.clamp(1, 10_000);

        info!(
            "Updating Ethereum Vault with:
            rate: {r}
            witdraw_fee_bps: {clamped_withdraw_fee}
            netting_amount: {netting_amount}"
        );

        let update_result = self
            .eth_client
            .execute_tx(
                valence_vault
                    .update(r, clamped_withdraw_fee, netting_amount)
                    .into_transaction_request(),
            )
            .await;

        if let Err(e) = &update_result {
            info!("Update failed: {:?}", e);
            panic!("failed to update vault");
        }

        if eth_rp
            .get_transaction_receipt(update_result.unwrap().transaction_hash)
            .await
            .map_err(|e| warn!("Error: {:?}", e))
            .unwrap()
            .is_none()
        {
            warn!("Failed to get update_vault tx receipt")
        };

        // 11. pull the funds due for deposit from origin to position domain
        //   1. cctp transfer eth deposit acc -> noble inbound ica
        //   2. ica ibc transfer noble inbound ica -> neutron deposit acc
        self.route_eth_to_noble().await;
        self.route_noble_to_neutron().await;

        // 12. enter the position with funds available in neutron deposit acc
        self.enter_position().await;

        // 13. pull the funds due for withdrawal from position to origin domain
        //   1. ibc transfer neutron withdraw acc -> noble outbound ica
        //   2. cctp transfer noble outbound ica -> eth withdraw acc
        self.route_neutron_to_noble().await;
        self.route_noble_to_eth().await;

        info!(
            "strategist loop completed at second {}",
            get_current_second()
        );

        Ok(())
    }
}
