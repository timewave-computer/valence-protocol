use std::{error::Error, path::Path, str::FromStr, time::Duration};

use crate::{
    strategist::{aave_utils::get_user_position, pancake_v3_utils::calculate_max_amounts_position},
    USDC_ADDRESS_ON_BASE, WETH_ADDRESS_ON_BASE,
};
use alloy::{
    primitives::{Address, Signed, U256},
    providers::Provider,
    rpc::types::TransactionRequest,
    signers::local::{coins_bip39::English, MnemonicBuilder},
    sol,
    sol_types::SolCall,
};
use alloy_sol_types_encoder::SolValue;
use async_trait::async_trait;
use log::{info, warn};
use valence_chain_client_utils::{
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_e2e::utils::{
    solidity_contracts::{
        AavePositionManager, CCTPTransfer,
        Forwarder::{self},
        PancakeSwapV3PositionManager, StandardBridgeTransfer, ValenceVault, ERC20,
    },
    worker::{ValenceWorker, ValenceWorkerTomlSerde},
};
use valence_encoder_utils::libraries::forwarder::solidity_types::{
    ForwarderConfig, ForwardingConfig, IntervalType,
};

use super::strategy_config::StrategyConfig;

// Since we dont have a library to sawp, we are going to hardcode the price of cake here in USD so that we can consider it in our
// yield calculations
const PRICE_OF_CAKE: f64 = 2.0;

sol! {
    // AAVE V3 Pool
    function getUserAccountData(
        address user
    ) external view returns (
        uint256 totalCollateralBase,
        uint256 totalDebtBase,
        uint256 availableBorrowsBase,
        uint256 currentLiquidationThreshold,
        uint256 ltv,
        uint256 healthFactor
    );

    // AAVE V3 Oracle
    function getAssetsPrices(address[] calldata assets) view returns (uint256[] memory);

    // Pancake V3
    function slot0()
        external
        view
        returns (
            uint160 sqrtPriceX96,
            int24 tick,
            uint16 observationIndex,
            uint16 observationCardinality,
            uint16 observationCardinalityNext,
            uint32 feeProtocol,
            bool unlocked
        );

    /// Pancake V3
    /// e.g.: a tickSpacing of 3 means ticks can be initialized every 3rd tick, i.e., ..., -6, -3, 0, 3, 6, ...
    /// This value is an int24 to avoid casting even though it is always positive.
    function tickSpacing() external view returns (int24);

    /// NFT queries
    function balanceOf(address owner) view returns (uint256);
    function tokenOfOwnerByIndex(address owner, uint256 index) view returns (uint256);
}

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

        let pending_obligations;
        let current_total_shares;
        let vault_deposit_acc_weth_bal;
        {
            info!("========= Vault Current Status =========");
            current_total_shares = self.eth_client.query(valence_vault.totalSupply()).await?._0;
            info!("Current total shares: {current_total_shares}");

            // Query the amount of WETH that needs to be withdrawn
            pending_obligations = self
                .eth_client
                .query(valence_vault.totalAssetsToWithdrawNextUpdate())
                .await?
                ._0;
            info!("Pending obligations: {pending_obligations}");

            // Query vault deposit account for its WETH balance
            vault_deposit_acc_weth_bal = self
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
        }

        {
            info!("========= Withdraw Pancake Position =========");
            let pancake_position_manager = PancakeSwapV3PositionManager::new(
                Address::from_str(&self.cfg.base.libraries.pancake_position_manager)?,
                &base_rp,
            );

            // Check if there is a position to withdraw
            let masterchef = self
                .base_client
                .query(pancake_position_manager.config())
                .await?
                .masterChef;

            let position_check = balanceOfCall {
                owner: Address::from_str(&self.cfg.base.accounts.pancake_input)?,
            }
            .abi_encode();

            let result = base_rp
                .call(
                    &TransactionRequest::default()
                        .to(masterchef)
                        .input(position_check.into()),
                )
                .await?;
            let amount_of_positions = balanceOfCall::abi_decode_returns(&result, true)?._0;

            if amount_of_positions != U256::ZERO {
                info!("Get position ID...");
                let nft_call = tokenOfOwnerByIndexCall {
                    owner: Address::from_str(&self.cfg.base.accounts.pancake_input)?,
                    index: U256::ZERO,
                }
                .abi_encode();

                let result = base_rp
                    .call(
                        &TransactionRequest::default()
                            .to(masterchef)
                            .input(nft_call.into()),
                    )
                    .await?;
                let position_id = tokenOfOwnerByIndexCall::abi_decode_returns(&result, true)?._0;

                info!("Withdrawing position with ID: {position_id}");
                let tx = pancake_position_manager
                    .withdrawPosition(position_id)
                    .into_transaction_request();
                self.base_client.execute_tx(tx).await?;
                info!("Pancake position withdrawn");
            } else {
                info!("No position to withdraw");
            }
        }

        {
            info!("========= Netting amount and Redemption Rate calculation =========");
            // Calculate the netting amount and update the pending obligations
            let netting_amount = pending_obligations.min(vault_deposit_acc_weth_bal);
            info!("Netting amount: {netting_amount}");

            let updated_pending_obligations = pending_obligations
                .checked_sub(netting_amount)
                .unwrap_or_default();
            info!("Updated pending obligations: {updated_pending_obligations}");

            info!("Calculating the vault total balance in the entire program...");
            // Get balances of all our accounts so that we can calculate the redemption rate
            // We are going to calculate every single place so that if the strategist stopped working at some point and it was restarted we get the balances that might
            // have been left somewhere.

            // ETH and USDC in our AAVE input account
            let aave_input_usdc_balance = self
                .eth_client
                .query(
                    ERC20::new(Address::from_str(&self.cfg.ethereum.denoms.usdc)?, &eth_rp)
                        .balanceOf(Address::from_str(&self.cfg.ethereum.accounts.aave_input)?),
                )
                .await?
                ._0;
            let aave_input_weth_balance = self
                .eth_client
                .query(
                    ERC20::new(Address::from_str(&self.cfg.ethereum.denoms.weth)?, &eth_rp)
                        .balanceOf(Address::from_str(&self.cfg.ethereum.accounts.aave_input)?),
                )
                .await?
                ._0;

            // USDC in our Ethereum CCTP input account
            let cctp_input_usdc_balance = self
                .eth_client
                .query(
                    ERC20::new(Address::from_str(&self.cfg.ethereum.denoms.usdc)?, &eth_rp)
                        .balanceOf(Address::from_str(&self.cfg.ethereum.accounts.cctp_input)?),
                )
                .await?
                ._0;
            // WETH in our Ethereum Standard Bridge input account
            let standard_bridge_input_weth_balance = self
                .eth_client
                .query(
                    ERC20::new(Address::from_str(&self.cfg.ethereum.denoms.weth)?, &eth_rp)
                        .balanceOf(Address::from_str(
                            &self.cfg.ethereum.accounts.standard_bridge_input,
                        )?),
                )
                .await?
                ._0;
            // USDC in our Pancake input account
            let pancake_input_usdc_balance = self
                .base_client
                .query(
                    ERC20::new(Address::from_str(&self.cfg.base.denoms.usdc)?, &base_rp)
                        .balanceOf(Address::from_str(&self.cfg.base.accounts.pancake_input)?),
                )
                .await?
                ._0;
            // WETH in our Pancake input account
            let pancake_input_weth_balance = self
                .base_client
                .query(
                    ERC20::new(Address::from_str(&self.cfg.base.denoms.weth)?, &base_rp)
                        .balanceOf(Address::from_str(&self.cfg.base.accounts.pancake_input)?),
                )
                .await?
                ._0;
            // USDC in our Pancake output account
            let pancake_output_usdc_balance = self
                .base_client
                .query(
                    ERC20::new(Address::from_str(&self.cfg.base.denoms.usdc)?, &base_rp)
                        .balanceOf(Address::from_str(&self.cfg.base.accounts.pancake_output)?),
                )
                .await?
                ._0;
            // WETH in our Pancake output account
            let pancake_output_weth_balance = self
                .base_client
                .query(
                    ERC20::new(Address::from_str(&self.cfg.base.denoms.weth)?, &base_rp)
                        .balanceOf(Address::from_str(&self.cfg.base.accounts.pancake_output)?),
                )
                .await?
                ._0;
            // CAKE in our Pancake output account
            let pancake_output_cake_balance = self
                .base_client
                .query(
                    ERC20::new(Address::from_str(&self.cfg.base.denoms.cake)?, &base_rp)
                        .balanceOf(Address::from_str(&self.cfg.base.accounts.pancake_output)?),
                )
                .await?
                ._0;

            // We calculate the pancake_output_cake_balance in USDC using the price of cake
            // We need to take into account that Cake has 18 decimals and USDC has 6
            let pancake_output_cake_balance_usdc = pancake_output_cake_balance
                .checked_mul(U256::from(PRICE_OF_CAKE * 1e6))
                .unwrap_or_default()
                .checked_div(U256::from(1e18))
                .unwrap_or_default();

            // WETH in our Base Standard Bridge input account
            let standard_bridge_input_weth_balance_base = self
                .base_client
                .query(
                    ERC20::new(Address::from_str(&self.cfg.base.denoms.weth)?, &base_rp).balanceOf(
                        Address::from_str(&self.cfg.base.accounts.standard_bridge_input)?,
                    ),
                )
                .await?
                ._0;
            // USDC in our Base CCTP input account
            let cctp_input_usdc_balance_base = self
                .base_client
                .query(
                    ERC20::new(Address::from_str(&self.cfg.base.denoms.usdc)?, &base_rp)
                        .balanceOf(Address::from_str(&self.cfg.base.accounts.cctp_input)?),
                )
                .await?
                ._0;
            // Now we query the current AAVE position status
            let (total_collateral_base, total_debt_base, _, _) = get_user_position(
                &self.eth_client,
                Address::from_str(&self.cfg.ethereum.libraries.aave_position_manager)?,
                Address::from_str(&self.cfg.ethereum.accounts.aave_input)?,
            )
            .await?;
            // We will substract the total debt from the total collateral to get the net worth of the AAVE position in USD and adjust to USDC decimals (6)
            let total_aave_position_net_worth = total_collateral_base
                .checked_sub(total_debt_base)
                .unwrap_or_default()
                .checked_mul(U256::from(1e6))
                .unwrap_or_default();
            info!("Total AAVE position net worth: {total_aave_position_net_worth}");

            // Let's get the asset prices from AAVE for both WETH and USDC which are returned in USD using 8 decimals
            let asset_prices_call = getAssetsPricesCall {
                assets: vec![
                    Address::from_str(&self.cfg.ethereum.denoms.weth)?,
                    Address::from_str(&self.cfg.ethereum.denoms.usdc)?,
                ],
            }
            .abi_encode();

            let result = eth_rp
                .call(
                    &TransactionRequest::default()
                        .to(Address::from_str(&self.cfg.ethereum.contracts.aave_oracle)?)
                        .input(asset_prices_call.into()),
                )
                .await?;
            let return_data = getAssetsPricesCall::abi_decode_returns(&result, true)?._0;
            let aave_weth_price = return_data[0];
            let aave_usdc_price = return_data[1];

            // Convert my networth position to USDC
            let total_aave_position_net_worth_usdc = total_aave_position_net_worth
                .checked_mul(aave_usdc_price)
                .unwrap_or_default()
                .checked_div(U256::from(1e8))
                .unwrap_or_default();

            // Add up all the usdc balances we got to see how much USDC we have
            let total_usdc_balance = aave_input_usdc_balance
                .checked_add(cctp_input_usdc_balance)
                .unwrap_or_default()
                .checked_add(pancake_input_usdc_balance)
                .unwrap_or_default()
                .checked_add(pancake_output_usdc_balance)
                .unwrap_or_default()
                .checked_add(total_aave_position_net_worth_usdc)
                .unwrap_or_default()
                .checked_add(pancake_output_cake_balance_usdc)
                .unwrap_or_default()
                .checked_add(cctp_input_usdc_balance_base)
                .unwrap_or_default();
            info!("Total USDC balance: {total_usdc_balance}");

            // Add up all the weth balances we got to see how much WETH we have
            let total_weth_balance = aave_input_weth_balance
                .checked_add(vault_deposit_acc_weth_bal)
                .unwrap_or_default()
                .checked_add(standard_bridge_input_weth_balance)
                .unwrap_or_default()
                .checked_add(pancake_input_weth_balance)
                .unwrap_or_default()
                .checked_add(pancake_output_weth_balance)
                .unwrap_or_default()
                .checked_add(standard_bridge_input_weth_balance_base)
                .unwrap_or_default();
            info!("Total WETH balance: {total_weth_balance}");

            // Now we are going to calculate how much WETH is my USDC balance equivalent to using the AAVE price
            let total_usdc_balance_in_weth = total_usdc_balance // USDC with 6 decimals
                .checked_mul(aave_usdc_price) // Convert to USD (8 decimals)
                .unwrap_or_default()
                .checked_mul(U256::from(1e18)) // Scale to WETH decimals (18)
                .unwrap_or_default()
                .checked_div(aave_weth_price) // Convert USD to WETH
                .unwrap_or_default()
                .checked_div(U256::from(1e6)) // Adjust for AAVE price decimals and USDC decimals
                .unwrap_or_default();
            info!("Total USDC balance in WETH: {total_usdc_balance_in_weth}");

            // Now we can know the total WETH that our Vault currently has
            let total_weth_balance = total_weth_balance
                .checked_add(total_usdc_balance_in_weth)
                .unwrap_or_default();

            // From this we have to substract the pending obligations
            let total_weth_balance = total_weth_balance
                .checked_sub(updated_pending_obligations)
                .unwrap_or_default();

            info!("Total WETH balance after pending obligations: {total_weth_balance}");
            info!("Current total shares: {current_total_shares}");

            // And now we can calculate the redemption rate by dividing the total WETH by the total shares, but scaling
            // it first so that we have enough precision
            let total_weth_balance_scaled = total_weth_balance
                .checked_mul(U256::from(1e6))
                .unwrap_or_default();
            info!("Total WETH balance scaled: {total_weth_balance_scaled}");
            let redemption_rate_scaled = total_weth_balance_scaled
                .checked_div(current_total_shares)
                .unwrap_or_default();
            info!("Redemption rate scaled: {redemption_rate_scaled}");
            // Now we need to scale it back down but give it the decimals of the token
            let redemption_rate = redemption_rate_scaled
                .checked_mul(U256::from(1e18))
                .unwrap_or_default()
                .checked_div(U256::from(1e6))
                .unwrap_or_default();

            info!("Redemption rate calculated: {redemption_rate}");

            info!("========= Unwind assets to meet pending obligations =========");
            if updated_pending_obligations > U256::ZERO {
                // We are going to bridge back the pending obligations, half in WETH and half in USDC
                let pending_obligations_in_weth = updated_pending_obligations
                    .checked_div(U256::from(2))
                    .unwrap_or_default();
                let pending_obligations_in_weth_from_aave =
                    updated_pending_obligations.saturating_sub(pending_obligations_in_weth);

                // We know the equivalent in USD of half of the WETH, for that we are going to use the AAVE price previously calculated
                // Taking into account the WETH is in 18 decimals and the AAVE USDC price is in 8 decimals
                let pending_obligations_weth_bridged_in_usd = pending_obligations_in_weth_from_aave
                    .checked_mul(aave_weth_price)
                    .unwrap_or_default()
                    .checked_div(U256::from(1e10))
                    .unwrap_or_default();
                info!("Pending obligations WETH bridged in USD: {pending_obligations_weth_bridged_in_usd}");

                // Now we need to convert this into USDC because there's a small difference between USDC and USD on AAVE
                // Also taking into account USDC has 6 decimals and AAVE USDC price has 8 decimals
                let pending_obligations_weth_bridged_in_usdc =
                    pending_obligations_weth_bridged_in_usd
                        .checked_mul(U256::from(1e8))
                        .unwrap_or_default()
                        .checked_div(aave_usdc_price)
                        .unwrap_or_default()
                        .checked_div(U256::from(1e2))
                        .unwrap_or_default();
                info!("Pending obligations WETH bridged in USDC: {pending_obligations_weth_bridged_in_usdc}");

                // Now we need to bridge back the WETH using the standard bridge
                // and the USDC using the CCTP bridge, for that we are going to update the amounts of the forwaders
                // to forward the right amount
                let forwarder_pancake_output_to_standard_bridge_config = ForwarderConfig {
                    inputAccount: alloy_primitives_encoder::Address::from_str(
                        &self.cfg.base.accounts.pancake_output,
                    )?,
                    outputAccount: alloy_primitives_encoder::Address::from_str(
                        &self.cfg.base.accounts.standard_bridge_input,
                    )?,
                    // Strategist will update this to forward the right amount
                    forwardingConfigs: vec![ForwardingConfig {
                        tokenAddress: alloy_primitives_encoder::Address::from_str(
                            WETH_ADDRESS_ON_BASE,
                        )?,
                        maxAmount: pending_obligations_in_weth,
                    }],
                    intervalType: IntervalType::TIME,
                    minInterval: 0,
                }
                .abi_encode();

                let forwarder_to_standard_input = Forwarder::new(
                    Address::from_str(
                        &self
                            .cfg
                            .base
                            .libraries
                            .pancake_output_to_standard_bridge_input_forwarder,
                    )?,
                    &base_rp,
                );
                info!("Updating forwarder from Pancake to Standard Bridge...");
                let tx = forwarder_to_standard_input
                    .updateConfig(forwarder_pancake_output_to_standard_bridge_config.into())
                    .into_transaction_request();
                self.base_client.execute_tx(tx).await?;
                info!("Forwarder to Standard Bridge updated");

                // Do the same for USDC to CCTP input
                let forwarder_pancake_output_to_cctp_input_config = ForwarderConfig {
                    inputAccount: alloy_primitives_encoder::Address::from_str(
                        &self.cfg.base.accounts.pancake_output,
                    )?,
                    outputAccount: alloy_primitives_encoder::Address::from_str(
                        &self.cfg.base.accounts.cctp_input,
                    )?,
                    // Strategist will update this to forward the right amount
                    forwardingConfigs: vec![ForwardingConfig {
                        tokenAddress: alloy_primitives_encoder::Address::from_str(
                            USDC_ADDRESS_ON_BASE,
                        )?,
                        maxAmount: pending_obligations_weth_bridged_in_usdc,
                    }],
                    intervalType: IntervalType::TIME,
                    minInterval: 0,
                }
                .abi_encode();

                let forwarder_to_cctp_input = Forwarder::new(
                    Address::from_str(
                        &self
                            .cfg
                            .base
                            .libraries
                            .pancake_output_to_cctp_input_forwarder,
                    )?,
                    &base_rp,
                );

                info!("Updating forwarder from Pancake to CCTP input...");
                let tx = forwarder_to_cctp_input
                    .updateConfig(forwarder_pancake_output_to_cctp_input_config.into())
                    .into_transaction_request();
                self.base_client.execute_tx(tx).await?;
                info!("Forwarder to CCTP input updated");

                // Now trigger the forwards
                let tx_forward = forwarder_to_standard_input
                    .forward()
                    .into_transaction_request();
                self.base_client.execute_tx(tx_forward).await?;
                info!("Forward from Pancake to Standard Bridge executed");

                let tx_forward = forwarder_to_cctp_input.forward().into_transaction_request();
                self.base_client.execute_tx(tx_forward).await?;
                info!("Forward from Pancake to CCTP input executed");

                // Get the balance of Standard bridge input account
                let standard_bridge_input_weth_balance = self
                    .base_client
                    .query(
                        ERC20::new(Address::from_str(&self.cfg.base.denoms.weth)?, &base_rp)
                            .balanceOf(Address::from_str(
                                &self.cfg.base.accounts.standard_bridge_input,
                            )?),
                    )
                    .await?
                    ._0;

                info!(
                    "Standard bridge input account WETH balance: {:?}",
                    standard_bridge_input_weth_balance
                );

                if standard_bridge_input_weth_balance > U256::ZERO {
                    // Get the balance of the vault before
                    let vault_withdraw_account_weth_balance_before = self
                        .eth_client
                        .query(
                            ERC20::new(Address::from_str(&self.cfg.ethereum.denoms.weth)?, &eth_rp)
                                .balanceOf(Address::from_str(
                                    &self.cfg.ethereum.accounts.vault_withdraw,
                                )?),
                        )
                        .await?
                        ._0;

                    // Now we need to trigger the Standard Bridge transfer
                    let standard_bridge_transfer = StandardBridgeTransfer::new(
                        Address::from_str(&self.cfg.base.libraries.standard_bridge_transfer)?,
                        &base_rp,
                    );
                    let tx = standard_bridge_transfer
                        .transfer()
                        .into_transaction_request();
                    self.base_client.execute_tx(tx).await?;

                    while {
                        // Check if the vault withdraw account has the WETH
                        let vault_withdraw_account_weth_balance_after = self
                            .eth_client
                            .query(
                                ERC20::new(
                                    Address::from_str(&self.cfg.ethereum.denoms.weth)?,
                                    &eth_rp,
                                )
                                .balanceOf(Address::from_str(
                                    &self.cfg.ethereum.accounts.vault_withdraw,
                                )?),
                            )
                            .await?
                            ._0;
                        info!(
                            "Vault withdraw account WETH balance: {:?}",
                            vault_withdraw_account_weth_balance_after
                        );
                        vault_withdraw_account_weth_balance_after
                            < vault_withdraw_account_weth_balance_before
                                + standard_bridge_input_weth_balance
                    } {
                        info!("Waiting for Standard Bridge transfer to complete...");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                    info!("Standard Bridge transfer completed!");
                } else {
                    info!("No WETH to bridge");
                }

                // Do exactly the same for CCTP
                let cctp_input_usdc_balance = self
                    .base_client
                    .query(
                        ERC20::new(Address::from_str(&self.cfg.base.denoms.usdc)?, &base_rp)
                            .balanceOf(Address::from_str(&self.cfg.base.accounts.cctp_input)?),
                    )
                    .await?
                    ._0;

                info!(
                    "CCTP input account USDC balance: {:?}",
                    cctp_input_usdc_balance
                );

                if cctp_input_usdc_balance > U256::ZERO {
                    // Get the balance of aave_input account before the transfer
                    let aave_input_account_usdc_balance_before = self
                        .eth_client
                        .query(
                            ERC20::new(Address::from_str(&self.cfg.ethereum.denoms.usdc)?, &eth_rp)
                                .balanceOf(Address::from_str(
                                    &self.cfg.ethereum.accounts.aave_input,
                                )?),
                        )
                        .await?
                        ._0;

                    // Now we need to trigger the CCTP transfer
                    let cctp_transfer = CCTPTransfer::new(
                        Address::from_str(&self.cfg.base.libraries.cctp_transfer)?,
                        &base_rp,
                    );
                    let tx = cctp_transfer.transfer().into_transaction_request();
                    self.base_client.execute_tx(tx).await?;

                    while {
                        // Check if aave input account has the USDC
                        let aave_input_account_usdc_balance_after = self
                            .eth_client
                            .query(
                                ERC20::new(
                                    Address::from_str(&self.cfg.ethereum.denoms.usdc)?,
                                    &eth_rp,
                                )
                                .balanceOf(Address::from_str(
                                    &self.cfg.ethereum.accounts.aave_input,
                                )?),
                            )
                            .await?
                            ._0;

                        info!(
                            "AAVE input account USDC balance: {:?}",
                            aave_input_account_usdc_balance_after
                        );

                        aave_input_account_usdc_balance_after
                            < aave_input_account_usdc_balance_before + cctp_input_usdc_balance
                    } {
                        info!("Waiting for CCTP transfer to complete...");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                    info!("CCTP transfer completed!");
                } else {
                    info!("No USDC to bridge");
                }

                info!("========= REPAY and WITHDRAW from AAVE =========");
                // Now we need to repay the AAVE position with the USDC we just bridged
                let aave_position_manager = AavePositionManager::new(
                    Address::from_str(&self.cfg.ethereum.libraries.aave_position_manager)?,
                    &eth_rp,
                );
                let aave_input_account_usdc_balance = self
                    .eth_client
                    .query(
                        ERC20::new(Address::from_str(&self.cfg.ethereum.denoms.usdc)?, &eth_rp)
                            .balanceOf(Address::from_str(&self.cfg.ethereum.accounts.aave_input)?),
                    )
                    .await?
                    ._0;
                info!(
                    "AAVE input account USDC balance: {:?}",
                    aave_input_account_usdc_balance
                );

                let tx = aave_position_manager
                    .repay(aave_input_account_usdc_balance)
                    .into_transaction_request();
                self.eth_client.execute_tx(tx).await?;
                info!("AAVE repay transaction executed");

                // Now we need to withdraw the equivalent WETH from AAVE
                let vault_withdraw_account_balance_before_withdraw = self
                    .eth_client
                    .query(
                        ERC20::new(Address::from_str(&self.cfg.ethereum.denoms.weth)?, &eth_rp)
                            .balanceOf(Address::from_str(
                                &self.cfg.ethereum.accounts.vault_withdraw,
                            )?),
                    )
                    .await?
                    ._0;
                info!(
                    "Vault withdraw account WETH balance before AAVE withdraw: {:?}",
                    vault_withdraw_account_balance_before_withdraw
                );

                let tx = aave_position_manager
                    .withdraw(pending_obligations_in_weth_from_aave)
                    .into_transaction_request();
                self.eth_client.execute_tx(tx).await?;

                let vault_withdraw_account_balance_after_withdraw = self
                    .eth_client
                    .query(
                        ERC20::new(Address::from_str(&self.cfg.ethereum.denoms.weth)?, &eth_rp)
                            .balanceOf(Address::from_str(
                                &self.cfg.ethereum.accounts.vault_withdraw,
                            )?),
                    )
                    .await?
                    ._0;

                info!(
                    "Vault withdraw account WETH balance after AAVE withdraw: {:?}",
                    vault_withdraw_account_balance_after_withdraw
                );

                // Finally now we can update the vault with the new redemption rate
                let tx = valence_vault
                    .update(redemption_rate, 100, netting_amount)
                    .into_transaction_request();
                self.eth_client.execute_tx(tx).await?;

                info!("Vault updated with new redemption rate!");
            } else {
                info!("No Pending obligations to meet");
            }
        }

        {
            info!("========= Forwarder Funds Pancake Output to Input =========");
            // Get the balances of Pancake output account
            let pancake_output_weth_balance = self
                .base_client
                .query(
                    ERC20::new(Address::from_str(&self.cfg.base.denoms.weth)?, &base_rp)
                        .balanceOf(Address::from_str(&self.cfg.base.accounts.pancake_output)?),
                )
                .await?
                ._0;
            info!(
                "Pancake output account WETH balance: {:?}",
                pancake_output_weth_balance
            );
            let pancake_output_usdc_balance = self
                .base_client
                .query(
                    ERC20::new(Address::from_str(&self.cfg.base.denoms.usdc)?, &base_rp)
                        .balanceOf(Address::from_str(&self.cfg.base.accounts.pancake_output)?),
                )
                .await?
                ._0;
            info!(
                "Pancake output account USDC balance: {:?}",
                pancake_output_usdc_balance
            );

            // If there is any USDC or WETH balance, we are going to forward it back to the input account
            if pancake_output_weth_balance > U256::ZERO || pancake_output_usdc_balance > U256::ZERO
            {
                info!("Forwarding funds from Pancake output account to pancake input account ...");
                let forwader_pancake_output_to_pancake_input = Forwarder::new(
                    Address::from_str(&self.cfg.base.libraries.pancake_output_to_input_forwarder)?,
                    &base_rp,
                );

                let tx = forwader_pancake_output_to_pancake_input
                    .forward()
                    .into_transaction_request();
                self.base_client.execute_tx(tx).await?;
                info!("Forwarded funds back from Pancake output account to pancake input account");
            }
        }

        {
            info!("========= Forwarder Setup =========");
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

            if weth_to_supply == U256::ZERO {
                info!("No WETH to supply");
            } else {
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

                // Now let's trigger the forwards
                let tx_forward = forwarder_to_aave.forward().into_transaction_request();
                self.eth_client.execute_tx(tx_forward).await?;
            }

            if weth_to_bridge == U256::ZERO {
                info!("No WETH to bridge");
            } else {
                // Update the forwarder to forward the right amount to the Standard bridge input account
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

                let tx_forward = forwarder_to_standard_bridge
                    .forward()
                    .into_transaction_request();
                self.eth_client.execute_tx(tx_forward).await?;
            }
        }

        {
            info!(" ========= AAVE Supply =========");
            // Check balances
            let aave_input_weth_bal = self
                .eth_client
                .query(
                    eth_weth.balanceOf(Address::from_str(&self.cfg.ethereum.accounts.aave_input)?),
                )
                .await?
                ._0;
            info!("AAVE input account WETH balance: {:?}", aave_input_weth_bal);

            if aave_input_weth_bal > U256::ZERO {
                // Trigger the AAVE supply
                let aave_position_manager = AavePositionManager::new(
                    Address::from_str(&self.cfg.ethereum.libraries.aave_position_manager)?,
                    &eth_rp,
                );
                let tx = aave_position_manager
                    .supply(aave_input_weth_bal)
                    .into_transaction_request();
                self.eth_client.execute_tx(tx).await?;
                info!("AAVE supply transaction executed");
            } else {
                info!("No WETH to supply");
            }
        }

        {
            info!("========= AAVE Borrow =========");
            info!("Borrow up to 50% of the WETH supplied");
            let (total_collateral_base, total_debt_base, available_borrows_base, health_factor) =
                get_user_position(
                    &self.eth_client,
                    Address::from_str(&self.cfg.ethereum.libraries.aave_position_manager)?,
                    Address::from_str(&self.cfg.ethereum.accounts.aave_input)?,
                )
                .await?;

            info!("Total collateral base: {total_collateral_base}");
            info!("Total debt base: {total_debt_base}");
            info!("Available borrows base: {available_borrows_base}");
            info!("Health factor: {health_factor}");

            // This will be in f64 format so we need to convert it to U256 with 18 decimals
            // because that is how AAVE returns it
            let min_health_factor = &self
                .cfg
                .ethereum
                .parameters
                .min_aave_health_factor
                .to_string();

            let min_health_factor_adjusted =
                min_health_factor.parse::<f64>().unwrap_or_default() * 1e18;

            if health_factor < U256::from(min_health_factor_adjusted) {
                warn!("Health factor is too low! Need to trigger unwind");
                // Here call to trigger emergency unwind mechanism will be triggered
            }

            // We are going to borrow up to 50% of the collateral, considering the current debt;
            // Check how much should be the total borrowed
            let total_to_be_borrowed = total_collateral_base
                .checked_div(U256::from(2))
                .unwrap_or_default();

            // Substract this from what we already have borrowed
            let borrow_amount = total_to_be_borrowed
                .checked_sub(total_debt_base)
                .unwrap_or_default();
            info!("Borrowing: {borrow_amount} USDC");

            if borrow_amount > U256::ZERO {
                // We adjust the borrow amount to USDC precision
                let borrow_amount = borrow_amount
                    .checked_mul(U256::from(1e6))
                    .unwrap_or_default();

                // Sometimes the borrow silently fails, so we need to check if the borrow amount was successfully received,
                // otherwise we try again
                let aave_position_manager = AavePositionManager::new(
                    Address::from_str(&self.cfg.ethereum.libraries.aave_position_manager)?,
                    &eth_rp,
                );
                while {
                    let tx = aave_position_manager
                        .borrow(borrow_amount)
                        .into_transaction_request();
                    self.eth_client.execute_tx(tx).await?;
                    let usdc =
                        ERC20::new(Address::from_str(&self.cfg.ethereum.denoms.usdc)?, &eth_rp);
                    let usdc_balance =
                        self.eth_client
                            .query(usdc.balanceOf(Address::from_str(
                                &self.cfg.ethereum.accounts.aave_input,
                            )?))
                            .await?
                            ._0;
                    info!(
                        "AAVE input account USDC balance after borrow: {:?}",
                        usdc_balance
                    );
                    usdc_balance < borrow_amount
                } {
                    info!("Didn't receive borrow amount, try again.");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            } else {
                info!("No borrow needed");
            }
        }

        {
            info!("========= Forward from AAVE input to CCTP input =========");
            // Check if there's usdc in the aave input account to forward
            let usdc = ERC20::new(Address::from_str(&self.cfg.ethereum.denoms.usdc)?, &eth_rp);
            let usdc_balance = self
                .eth_client
                .query(usdc.balanceOf(Address::from_str(&self.cfg.ethereum.accounts.aave_input)?))
                .await?
                ._0;
            info!("AAVE input account USDC balance: {:?}", usdc_balance);

            // Trigger the forward
            if usdc_balance > U256::ZERO {
                let forwarder_aave_to_cctp = Forwarder::new(
                    Address::from_str(
                        &self
                            .cfg
                            .ethereum
                            .libraries
                            .forwarder_aave_input_to_cctp_input,
                    )?,
                    &eth_rp,
                );
                let tx_forward = forwarder_aave_to_cctp.forward().into_transaction_request();
                self.eth_client.execute_tx(tx_forward).await?;
                info!("Forward from AAVE input to CCTP input executed");
            } else {
                info!("No USDC to forward");
            }
        }

        {
            info!("========= CCTP Transfer =========");
            // Check if there's something in the cctp input account to transfer
            let usdc = ERC20::new(Address::from_str(&self.cfg.ethereum.denoms.usdc)?, &eth_rp);
            let usdc_balance = self
                .eth_client
                .query(usdc.balanceOf(Address::from_str(&self.cfg.ethereum.accounts.cctp_input)?))
                .await?
                ._0;
            info!("CCTP input account USDC balance: {:?}", usdc_balance);

            if usdc_balance > U256::ZERO {
                // Trigger the CCTP transfer
                let cctp_transfer = CCTPTransfer::new(
                    Address::from_str(&self.cfg.ethereum.libraries.cctp_transfer)?,
                    &eth_rp,
                );

                let usdc_on_base =
                    ERC20::new(Address::from_str(&self.cfg.base.denoms.usdc)?, &base_rp);
                // Get the balance of the pancake input before the transfer
                let pancake_input_usdc_balance_before = self
                    .base_client
                    .query(
                        usdc_on_base
                            .balanceOf(Address::from_str(&self.cfg.base.accounts.pancake_input)?),
                    )
                    .await?
                    ._0;
                info!(
                    "Pancake input account USDC balance before transfer: {:?}",
                    pancake_input_usdc_balance_before
                );
                let tx = cctp_transfer.transfer().into_transaction_request();
                self.eth_client.execute_tx(tx).await?;
                info!("CCTP transfer triggered");
                // Wait until the transfer is completed
                while {
                    let pancake_input_usdc_balance_after =
                        self.base_client
                            .query(usdc_on_base.balanceOf(Address::from_str(
                                &self.cfg.base.accounts.pancake_input,
                            )?))
                            .await?
                            ._0;
                    info!(
                        "Pancake input account USDC balance after transfer: {:?}",
                        pancake_input_usdc_balance_after
                    );
                    pancake_input_usdc_balance_after
                        < pancake_input_usdc_balance_before + usdc_balance
                } {
                    info!("Waiting for CCTP transfer to complete...");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
                info!("CCTP transfer completed!");
            } else {
                info!("No USDC to transfer");
            }
        }

        {
            info!(" ======== Standard Bridge Transfer =========");
            // Check if there's something in the standard bridge input account to transfer
            let weth = ERC20::new(Address::from_str(&self.cfg.ethereum.denoms.weth)?, &eth_rp);
            let weth_balance = self
                .eth_client
                .query(weth.balanceOf(Address::from_str(
                    &self.cfg.ethereum.accounts.standard_bridge_input,
                )?))
                .await?
                ._0;
            info!(
                "Standard bridge input account WETH balance: {:?}",
                weth_balance
            );
            if weth_balance > U256::ZERO {
                // Trigger the Standard Bridge transfer
                let standard_bridge_transfer = StandardBridgeTransfer::new(
                    Address::from_str(&self.cfg.ethereum.libraries.standard_bridge_transfer)?,
                    &eth_rp,
                );
                // Get the balance of the pancake input before the transfer
                let weth_on_base =
                    ERC20::new(Address::from_str(&self.cfg.base.denoms.weth)?, &base_rp);
                let pancake_input_weth_balance_before = self
                    .base_client
                    .query(
                        weth_on_base
                            .balanceOf(Address::from_str(&self.cfg.base.accounts.pancake_input)?),
                    )
                    .await?
                    ._0;
                info!(
                    "Pancake input account WETH balance before transfer: {:?}",
                    pancake_input_weth_balance_before
                );
                let tx = standard_bridge_transfer
                    .transfer()
                    .into_transaction_request();
                self.eth_client.execute_tx(tx).await?;
                info!("Standard Bridge transfer triggered");
                // Wait until the transfer is completed
                while {
                    let pancake_input_weth_balance_after =
                        self.base_client
                            .query(weth_on_base.balanceOf(Address::from_str(
                                &self.cfg.base.accounts.pancake_input,
                            )?))
                            .await?
                            ._0;
                    info!(
                        "Pancake input account WETH balance after transfer: {:?}",
                        pancake_input_weth_balance_after
                    );
                    pancake_input_weth_balance_after
                        < pancake_input_weth_balance_before + weth_balance
                } {
                    info!("Waiting for Standard Bridge transfer to complete...");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
                info!("Standard Bridge transfer completed!");
            } else {
                info!("No WETH to transfer");
            }
        }
        {
            info!("========= Pancake Position Manager =========");
            // Check if there is something in the pancake input account to provide
            let pancake_input_usdc_balance = self
                .base_client
                .query(
                    ERC20::new(Address::from_str(&self.cfg.base.denoms.usdc)?, &base_rp)
                        .balanceOf(Address::from_str(&self.cfg.base.accounts.pancake_input)?),
                )
                .await?
                ._0;
            info!(
                "Pancake input account USDC balance: {:?}",
                pancake_input_usdc_balance
            );
            let pancake_input_weth_balance = self
                .base_client
                .query(
                    ERC20::new(Address::from_str(&self.cfg.base.denoms.weth)?, &base_rp)
                        .balanceOf(Address::from_str(&self.cfg.base.accounts.pancake_input)?),
                )
                .await?
                ._0;
            info!(
                "Pancake input account WETH balance: {:?}",
                pancake_input_weth_balance
            );

            if pancake_input_usdc_balance > U256::ZERO && pancake_input_weth_balance > U256::ZERO {
                // First let's query the slot0 information of the pool
                let slot0_data = slot0Call {}.abi_encode();

                let result = base_rp
                    .call(
                        &TransactionRequest::default()
                            .to(Address::from_str(&self.cfg.base.contracts.pancake_pool)?)
                            .input(slot0_data.into()),
                    )
                    .await?;
                let return_data = slot0Call::abi_decode_returns(&result, true)?;
                let sqrt_price_x96 = return_data.sqrtPriceX96;
                let tick = return_data.tick;

                info!("Slot0 data: sqrtPriceX96: {sqrt_price_x96}, tick: {tick}");

                // Get the tick spacing
                let tick_spacing = tickSpacingCall {}.abi_encode();

                let result = base_rp
                    .call(
                        &TransactionRequest::default()
                            .to(Address::from_str(&self.cfg.base.contracts.pancake_pool)?)
                            .input(tick_spacing.into()),
                    )
                    .await?;
                let return_data = tickSpacingCall::abi_decode_returns(&result, true)?;
                let tick_spacing = return_data._0;
                info!("Tick spacing: {tick_spacing}");

                // Now we are going to calculate the amount of USDC and WETH that we can use
                let (lower_tick, upper_tick, amount_weth, amount_usdc) =
                    calculate_max_amounts_position(
                        U256::from(pancake_input_weth_balance),
                        U256::from(pancake_input_usdc_balance),
                        sqrt_price_x96,
                        tick.as_i32(),
                        tick_spacing.as_i32(),
                        f64::from_str(&self.cfg.base.parameters.tick_price_range_percent)?,
                    )?;
                info!("Amount WETH to create position with: {amount_weth}");
                info!("Amount USDC to create position with: {amount_usdc}");

                // Create the position
                let pancake_position_manager = PancakeSwapV3PositionManager::new(
                    Address::from_str(&self.cfg.base.libraries.pancake_position_manager)?,
                    &base_rp,
                );
                info!("Creating position...");
                let tx = pancake_position_manager
                    .createPosition(
                        Signed::<24, 1>::from_str(&lower_tick.to_string())?,
                        Signed::<24, 1>::from_str(&upper_tick.to_string())?,
                        amount_weth,
                        amount_usdc,
                    )
                    .into_transaction_request();
                self.base_client.execute_tx(tx).await?;

                // Get the position ID that we created, we can only have 1 position so we query the first index on the MasterChef
                // First we get the masterchef address
                let masterchef = self
                    .base_client
                    .query(pancake_position_manager.config())
                    .await?
                    .masterChef;

                let nft_call = tokenOfOwnerByIndexCall {
                    owner: Address::from_str(&self.cfg.base.accounts.pancake_input)?,
                    index: U256::ZERO,
                }
                .abi_encode();

                let result = base_rp
                    .call(
                        &TransactionRequest::default()
                            .to(masterchef)
                            .input(nft_call.into()),
                    )
                    .await?;
                let position_id = tokenOfOwnerByIndexCall::abi_decode_returns(&result, true)?._0;
                info!("Pancake Position created with ID: {position_id}");

                // Check the balance left in the pancake input account
                let pancake_input_usdc_balance = self
                    .base_client
                    .query(
                        ERC20::new(Address::from_str(&self.cfg.base.denoms.usdc)?, &base_rp)
                            .balanceOf(Address::from_str(&self.cfg.base.accounts.pancake_input)?),
                    )
                    .await?
                    ._0;
                info!(
                    "Pancake input account USDC balance after position creation: {:?}",
                    pancake_input_usdc_balance
                );
                let pancake_input_weth_balance = self
                    .base_client
                    .query(
                        ERC20::new(Address::from_str(&self.cfg.base.denoms.weth)?, &base_rp)
                            .balanceOf(Address::from_str(&self.cfg.base.accounts.pancake_input)?),
                    )
                    .await?
                    ._0;
                info!(
                    "Pancake input account WETH balance after position creation: {:?}",
                    pancake_input_weth_balance
                );
            } else {
                info!("No USDC or WETH to provide");
            }
        }

        info!("{worker_name}: Cycle completed, sleep for 15 seconds...");
        tokio::time::sleep(Duration::from_secs(15)).await;

        Ok(())
    }
}
