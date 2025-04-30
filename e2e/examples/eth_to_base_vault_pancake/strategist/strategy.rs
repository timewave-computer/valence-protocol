use std::{error::Error, path::Path, str::FromStr, time::Duration};

use alloy::{
    primitives::{Address, U256},
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use alloy_sol_types_encoder::SolValue;
use async_trait::async_trait;
use log::info;
use valence_chain_client_utils::{
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_e2e::utils::{
    solidity_contracts::{
        AavePositionManager, CCTPTransfer,
        Forwarder::{self},
        StandardBridgeTransfer, ValenceVault, ERC20,
    },
    worker::{ValenceWorker, ValenceWorkerTomlSerde},
};
use valence_encoder_utils::libraries::forwarder::solidity_types::{
    ForwarderConfig, ForwardingConfig, IntervalType,
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
                .index(7)? // derive the mnemonic at a different index to avoid nonce issues
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
        info!("{worker_name}: Waiting 5 seconds...");
        tokio::time::sleep(Duration::from_secs(5)).await;
        info!("{worker_name}: Worker loop started");

        let eth_rp = self.eth_client.get_request_provider().await?;
        let base_rp = self.base_client.get_request_provider().await?;

        let valence_vault = ValenceVault::new(
            Address::from_str(&self.cfg.ethereum.libraries.vault)?,
            &eth_rp,
        );
        let eth_weth = ERC20::new(Address::from_str(&self.cfg.ethereum.denoms.weth)?, &eth_rp);

        // Query the amount of WETH that needs to be withdrawn
        let pending_obligations = self
            .eth_client
            .query(valence_vault.totalAssetsToWithdrawNextUpdate())
            .await?
            ._0;

        info!("Pending obligations: {pending_obligations}");

        // Query vault deposit account for its WETH balance
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

        // Calculate the netting amount and update the pending obligations
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

        // Update the forwarder to forward the right amount to the AAVE input account and Standard bridge input account
        let forwarder_vault_to_aave_config = ForwarderConfig {
            inputAccount: alloy_primitives_encoder::Address::from_str(
                &self.cfg.ethereum.accounts.vault_deposit,
            )?,
            outputAccount: alloy_primitives_encoder::Address::from_str(
                &self.cfg.ethereum.accounts.aave_input,
            )?,
            // Strategist will update this to forward the right amount
            forwardingConfigs: vec![ForwardingConfig {
                tokenAddress: alloy_primitives_encoder::Address::from_str(
                    &self.cfg.ethereum.denoms.weth,
                )?,
                maxAmount: weth_to_supply,
            }],
            intervalType: IntervalType::TIME,
            minInterval: 0,
        }
        .abi_encode();
        let forwarder_to_aave = Forwarder::new(
            Address::from_str(
                &self
                    .cfg
                    .ethereum
                    .libraries
                    .forwarder_vault_deposit_to_aave_input,
            )?,
            &eth_rp,
        );
        info!("Updating forwarder to AAVE...");
        let tx = forwarder_to_aave
            .updateConfig(forwarder_vault_to_aave_config.into())
            .into_transaction_request();
        self.eth_client.execute_tx(tx).await?;
        info!("Forwarder to AAVE updated");

        let forwarder_vault_to_standard_bridge_config = ForwarderConfig {
            inputAccount: alloy_primitives_encoder::Address::from_str(
                &self.cfg.ethereum.accounts.vault_deposit,
            )?,
            outputAccount: alloy_primitives_encoder::Address::from_str(
                &self.cfg.ethereum.accounts.standard_bridge_input,
            )?,
            // Strategist will update this to forward the right amount
            forwardingConfigs: vec![ForwardingConfig {
                tokenAddress: alloy_primitives_encoder::Address::from_str(
                    &self.cfg.ethereum.denoms.weth,
                )?,
                maxAmount: weth_to_bridge,
            }],
            intervalType: IntervalType::TIME,
            minInterval: 0,
        }
        .abi_encode();
        let forwarder_to_standard_bridge = Forwarder::new(
            Address::from_str(
                &self
                    .cfg
                    .ethereum
                    .libraries
                    .forwarder_vault_deposit_to_standard_bridge_input,
            )?,
            &eth_rp,
        );
        info!("Updating forwarder to Standard Bridge...");
        let tx = forwarder_to_standard_bridge
            .updateConfig(forwarder_vault_to_standard_bridge_config.into())
            .into_transaction_request();
        self.eth_client.execute_tx(tx).await?;

        // Now let's trigger the forwards
        let tx_forward = forwarder_to_aave.forward().into_transaction_request();
        self.eth_client.execute_tx(tx_forward).await?;
        let tx_forward = forwarder_to_standard_bridge
            .forward()
            .into_transaction_request();
        self.eth_client.execute_tx(tx_forward).await?;

        // Check balances
        let aave_input_weth_bal = self
            .eth_client
            .query(eth_weth.balanceOf(Address::from_str(&self.cfg.ethereum.accounts.aave_input)?))
            .await?
            ._0;
        info!("AAVE input account balance: {:?}", aave_input_weth_bal);
        let standard_bridge_input_weth_bal = self
            .eth_client
            .query(eth_weth.balanceOf(Address::from_str(
                &self.cfg.ethereum.accounts.standard_bridge_input,
            )?))
            .await?
            ._0;
        info!(
            "Standard bridge input account balance: {:?}",
            standard_bridge_input_weth_bal
        );

        // Supply the WETH to AAVE
        let aave_position_manager = AavePositionManager::new(
            Address::from_str(&self.cfg.ethereum.libraries.aave_position_manager)?,
            &eth_rp,
        );
        let tx = aave_position_manager.supply(U256::ZERO).into_transaction_request();
        self.eth_client.execute_tx(tx).await?;
        info!("AAVE supply transaction executed");

        // Borrow USDC equivalent to half of the WETH supplied
        



        // Trigger the bridge transfers
        let standard_bridge_transfer_eth = StandardBridgeTransfer::new(
            Address::from_str(&self.cfg.ethereum.libraries.standard_bridge_transfer)?,
            &eth_rp,
        );
        let _cctp_transfer_eth = CCTPTransfer::new(
            Address::from_str(&self.cfg.ethereum.libraries.cctp_transfer)?,
            &eth_rp,
        );
        let tx = standard_bridge_transfer_eth
            .transfer()
            .into_transaction_request();
        self.eth_client.execute_tx(tx).await?;
        /*let tx = cctp_transfer_eth.transfer().into_transaction_request();
        self.eth_client.execute_tx(tx).await?;*/
        info!("Bridge transfers triggered");

        // Sleep enough time for relayers to pick up the transfers
        info!("{worker_name}: Waiting 8 seconds for relayers to pick up the transfers...");
        tokio::time::sleep(Duration::from_secs(8)).await;

        // Check the balances on Base
        let base_weth = ERC20::new(Address::from_str(&self.cfg.base.denoms.weth)?, &base_rp);
        let pancake_input_weth_balance = self
            .base_client
            .query(base_weth.balanceOf(Address::from_str(&self.cfg.base.accounts.pancake_input)?))
            .await?
            ._0;
        info!(
            "Pancake input account balance: {:?}",
            pancake_input_weth_balance
        );
        let base_usdc = ERC20::new(Address::from_str(&self.cfg.base.denoms.usdc)?, &base_rp);
        let pancake_input_usdc_balance = self
            .base_client
            .query(base_usdc.balanceOf(Address::from_str(&self.cfg.base.accounts.pancake_input)?))
            .await?
            ._0;
        info!(
            "Pancake input account USDC balance: {:?}",
            pancake_input_usdc_balance
        );

        info!("{worker_name}: Cycle completed, wait 30 seconds...");
        tokio::time::sleep(Duration::from_secs(30)).await;

        Ok(())
    }
}
