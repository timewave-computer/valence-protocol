use std::{error::Error, path::Path, str::FromStr, time::Duration};

use alloy::{
    primitives::{Address, U256},
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use async_trait::async_trait;
use log::info;
use valence_chain_client_utils::{
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_e2e::utils::{
    solidity_contracts::{ValenceVault, ERC20},
    worker::{ValenceWorker, ValenceWorkerTomlSerde},
};

use super::strategy_config::StrategyConfig;

pub struct Strategy {
    pub cfg: StrategyConfig,

    pub(crate) eth_client: EthereumClient,
    pub(crate) base_client: EthereumClient,
}

impl Strategy {
    // async constructor which initializes the clients baesd on the StrategyConfig
    pub async fn new(cfg: StrategyConfig) -> Result<Self, Box<dyn Error>> {
        let eth_client = EthereumClient {
            rpc_url: cfg.ethereum.rpc_url.to_string(),
            signer: MnemonicBuilder::<English>::default()
                .phrase(cfg.ethereum.mnemonic.clone())
                .index(7)? // derive the mnemonic at a different index to avoid nonce issues
                .build()?,
        };

        let base_client = EthereumClient {
            rpc_url: cfg.base.rpc_url.to_string(),
            signer: MnemonicBuilder::<English>::default()
                .phrase(cfg.base.mnemonic.clone())
                .index(8)? // derive the mnemonic at a different index to avoid nonce issues
                .build()?,
        };

        Ok(Strategy {
            cfg,
            eth_client,
            base_client,
        })
    }

    // initialization helper that parses StrategyConfig from a file and calls the
    // default constructor (`Strategy::new`)
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let strategy_cfg = StrategyConfig::from_file(path)?;
        Self::new(strategy_cfg).await
    }
}

#[async_trait]
impl ValenceWorker for Strategy {
    fn get_name(&self) -> String {
        "Valence Vault: ETH-BASE".to_string()
    }

    async fn cycle(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let worker_name = self.get_name();
        info!("{worker_name}: Starting cycle...");
        info!("{worker_name}: Waiting 30 seconds...");
        tokio::time::sleep(Duration::from_secs(30)).await;
        info!("{worker_name}: Worker loop started");

        let eth_rp = self.eth_client.get_request_provider().await?;
        let base_rp = self.base_client.get_request_provider().await?;

        let valence_vault = ValenceVault::new(
            Address::from_str(&self.cfg.ethereum.libraries.vault)?,
            &eth_rp,
        );
        let eth_weth = ERC20::new(Address::from_str(&self.cfg.ethereum.denoms.weth)?, &eth_rp);

        // 1. Query the amount of WETH that needs to be withdrawn
        let pending_obligations = self
            .eth_client
            .query(valence_vault.totalAssetsToWithdrawNextUpdate())
            .await?
            ._0;

        info!("Pending obligations: {pending_obligations}");

        // 2. Query vault deposit account for its WETH balance
        let vault_deposit_acc_weth_bal = self
            .eth_client
            .query(eth_weth.balanceOf(Address::from_str(
                &self.cfg.ethereum.accounts.vault_deposit,
            )?))
            .await?
            ._0;

        info!(
            "Vault deposit account balance: {:?}",
            vault_deposit_acc_weth_bal
        );

        // 3. Calculate the netting amount and update the pending obligations
        let netting_amount = pending_obligations.min(vault_deposit_acc_weth_bal);
        info!("Netting amount: {netting_amount}");

        let pending_obligations = pending_obligations
            .checked_sub(netting_amount)
            .unwrap_or_default();
        info!("Updated pending obligations: {pending_obligations}");

        // TODO: Deal with withdraws and netting

        // See how much WETH we have left in the vault deposit account
        let vault_deposit_acc_weth_bal = self
            .eth_client
            .query(eth_weth.balanceOf(Address::from_str(
                &self.cfg.ethereum.accounts.vault_deposit,
            )?))
            .await?
            ._0;
        info!(
            "Vault deposit account balance to provide: {:?}",
            vault_deposit_acc_weth_bal
        );

        // 2/3s of this amount needs to be supplied in AAVE and 1/3rd needs to be bridged to Base
        let weth_to_supply = vault_deposit_acc_weth_bal
            .checked_div(U256::from(3))
            .unwrap_or_default()
            .checked_mul(U256::from(2))
            .unwrap_or_default();
        info!("WETH to supply: {weth_to_supply}");
        let weth_to_bridge = vault_deposit_acc_weth_bal
            .checked_div(U256::from(3))
            .unwrap_or_default();
        info!("WETH to bridge: {weth_to_bridge}");

        Ok(())
    }
}
