use std::{error::Error, path::Path, str::FromStr, time::Duration};

use crate::strategist::pancake_v3_utils::calculate_max_amounts_position;
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

sol! {
    // AAVE getUserAccountData function
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
            let aave_position_manager = AavePositionManager::new(
                Address::from_str(&self.cfg.ethereum.libraries.aave_position_manager)?,
                &eth_rp,
            );

            let pool_address = self
                .eth_client
                .query(aave_position_manager.config())
                .await?
                .poolAddress;

            let user_account_data = getUserAccountDataCall {
                user: Address::from_str(&self.cfg.ethereum.accounts.aave_input)?,
            }
            .abi_encode();

            let result = eth_rp
                .call(
                    &TransactionRequest::default()
                        .to(pool_address)
                        .input(user_account_data.into()),
                )
                .await?;
            let return_data = getUserAccountDataCall::abi_decode_returns(&result, true)?;

            // Divide all values by 10^8 and health factor by 10^18 because that's how AAVE returns them
            let total_collateral_base = return_data
                .totalCollateralBase
                .checked_div(U256::from(1e8))
                .unwrap_or_default();
            let total_debt_base = return_data
                .totalDebtBase
                .checked_div(U256::from(1e8))
                .unwrap_or_default();
            let available_borrows_base = return_data
                .availableBorrowsBase
                .checked_div(U256::from(1e8))
                .unwrap_or_default();
            let health_factor = return_data
                .healthFactor
                .checked_div(U256::from(1e18))
                .unwrap_or_default();

            info!("Total collateral base: {total_collateral_base}");
            info!("Total debt base: {total_debt_base}");
            info!("Available borrows base: {available_borrows_base}");
            info!("Health factor: {health_factor}");

            let healthfactor_parsed =
                U256::from_str(&self.cfg.ethereum.parameters.min_aave_health_factor)?;
            // Since health factor has a 12 value, representing 1.2, we need to multiple it to what AAVE uses for healthfactor, which is 10^18
            let health_factor_adjusted = healthfactor_parsed
                .checked_mul(U256::from(1e17))
                .unwrap_or_default();

            if return_data.healthFactor < health_factor_adjusted {
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
                let tx = aave_position_manager
                    .borrow(borrow_amount)
                    .into_transaction_request();
                self.eth_client.execute_tx(tx).await?;
                info!("AAVE borrow transaction executed");
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
                    pancake_input_usdc_balance_before == pancake_input_usdc_balance_after
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
                    pancake_input_weth_balance_before == pancake_input_weth_balance_after
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

        info!("{worker_name}: Cycle completed, sleep for 60 seconds...");
        tokio::time::sleep(Duration::from_secs(60)).await;

        Ok(())
    }
}
