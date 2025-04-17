use std::{error::Error, str::FromStr, time::UNIX_EPOCH};

use alloy::{
    primitives::{Address, U256},
    providers::Provider,
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use async_trait::async_trait;
use cosmwasm_std::Uint128;
use localic_utils::{NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_ID};
use log::{info, warn};
use serde::{Deserialize, Serialize};
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

pub struct Strategy {
    pub config: StrategyConfig,
    pub runtime: StrategyRuntime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub noble_cfg: noble::NobleStrategyConfig,
    pub neutron_cfg: neutron::NeutronStrategyConfig,
    pub ethereum_cfg: ethereum::EthereumStrategyConfig,
}

pub struct StrategyRuntime {
    pub eth_client: EthereumClient,
    pub noble_client: NobleClient,
    pub neutron_client: NeutronClient,
}

impl StrategyRuntime {
    pub async fn from_config(
        strategy: &StrategyConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let noble_client = NobleClient::new(
            &strategy.noble_cfg.grpc_url,
            &strategy.noble_cfg.grpc_port,
            &strategy.noble_cfg.mnemonic,
            &strategy.noble_cfg.chain_id,
            NOBLE_CHAIN_DENOM,
        )
        .await
        .expect("failed to create noble client");

        let neutron_client = NeutronClient::new(
            &strategy.neutron_cfg.grpc_url,
            &strategy.neutron_cfg.grpc_port,
            &strategy.neutron_cfg.mnemonic,
            NEUTRON_CHAIN_ID,
        )
        .await
        .expect("failed to create neutron client");

        let eth_client = EthereumClient {
            rpc_url: strategy.ethereum_cfg.rpc_url.to_string(),
            signer: MnemonicBuilder::<English>::default()
                .phrase(strategy.ethereum_cfg.mnemonic.clone())
                .index(7)? // derive the mnemonic at a different index to avoid nonce issues
                .build()?,
        };

        Ok(Self {
            eth_client,
            noble_client,
            neutron_client,
        })
    }
}

impl Strategy {
    pub async fn new(config: StrategyConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let runtime = StrategyRuntime::from_config(&config).await?;
        Ok(Self { config, runtime })
    }
}

#[async_trait]
impl ValenceWorker for Strategy {
    fn get_name(&self) -> String {
        "Valence X-Vault: ETH-NOBLE-NEUTRON".to_string()
    }

    async fn cycle(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        log::info!("{}: Starting cycle...", self.get_name());
        log::info!("{}: Waiting until next minute...", self.get_name());

        let start_time = wait_until_next_minute().await;

        log::info!(
            "{}: Wait completed at {:?}, now proceeding with cycle logic",
            self.get_name(),
            start_time.duration_since(UNIX_EPOCH).unwrap()
        );

        info!("strategist loop started at second {}", get_current_second());
        let eth_rp = self
            .runtime
            .eth_client
            .get_request_provider()
            .await
            .unwrap();
        let valence_vault = ValenceVault::new(
            Address::from_str(&self.config.ethereum_cfg.libraries.valence_vault).unwrap(),
            &eth_rp,
        );
        let eth_usdc_erc20 = MockERC20::new(
            Address::from_str(&self.config.ethereum_cfg.denoms.usdc_erc20).unwrap(),
            &eth_rp,
        );

        // 1. calculate the amount of usdc needed to fulfill
        // the active withdraw obligations
        let pending_obligations = self
            .runtime
            .eth_client
            .query(valence_vault.totalAssetsToWithdrawNextUpdate())
            .await
            .unwrap()
            ._0;

        // 2. query ethereum program accounts for their usdc balances
        let eth_deposit_acc_usdc_bal =
            self.runtime
                .eth_client
                .query(eth_usdc_erc20.balanceOf(
                    Address::from_str(&self.config.ethereum_cfg.accounts.deposit).unwrap(),
                ))
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
                &self.config.neutron_cfg.target_pool.to_string(),
                NEUTRON_CHAIN_DENOM,
                &self.config.neutron_cfg.denoms.usdc,
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
                &self.config.neutron_cfg.target_pool,
                &self.config.neutron_cfg.denoms.usdc,
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
            .runtime
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

impl ValenceWorkerTomlSerde for StrategyConfig {
    fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let contents = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&contents)?;
        Ok(config)
    }

    fn to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let toml_string = toml::to_string(self)?;
        std::fs::write(path, toml_string)?;
        Ok(())
    }
}

pub mod noble {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NobleStrategyConfig {
        pub grpc_url: String,
        pub grpc_port: String,
        pub chain_id: String,
        pub mnemonic: String,
    }
}

pub mod neutron {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NeutronStrategyConfig {
        pub grpc_url: String,
        pub grpc_port: String,
        pub chain_id: String,
        pub mnemonic: String,
        pub target_pool: String,
        pub denoms: NeutronDenoms,
        pub accounts: NeutronAccounts,
        pub libraries: NeutronLibraries,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NeutronDenoms {
        pub lp_token: String,
        pub usdc: String,
        pub ntrn: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NeutronAccounts {
        pub deposit: String,
        pub position: String,
        pub withdraw: String,
        pub liquidation: String,
        pub noble_inbound_ica: IcaAccount,
        pub noble_outbound_ica: IcaAccount,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct IcaAccount {
        pub library_account: String,
        pub remote_addr: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NeutronLibraries {
        pub neutron_ibc_transfer: String,
        pub noble_inbound_transfer: String,
        pub noble_cctp_transfer: String,
        pub astroport_lper: String,
        pub astroport_lwer: String,
        pub liquidation_forwarder: String,
        pub authorizations: String,
        pub processor: String,
    }
}

pub mod ethereum {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EthereumStrategyConfig {
        pub rpc_url: String,
        pub mnemonic: String,
        pub denoms: EthereumDenoms,
        pub accounts: EthereumAccounts,
        pub libraries: EthereumLibraries,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EthereumDenoms {
        pub usdc_erc20: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EthereumAccounts {
        pub deposit: String,
        pub withdraw: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EthereumLibraries {
        pub valence_vault: String,
        pub cctp_forwarder: String,
        pub lite_processor: String,
    }
}
