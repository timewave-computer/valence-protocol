use std::str::FromStr;

use alloy::primitives::{Address, U256};
use async_trait::async_trait;
use cosmwasm_std::Uint128;
use localic_utils::NEUTRON_CHAIN_DENOM;
use log::{error, info, warn};
use valence_chain_client_utils::{
    cosmos::{base_client::BaseClient, wasm_client::WasmClient},
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_e2e::utils::solidity_contracts::{IBCEurekaTransfer, IEurekaHandler, MockERC20};
use valence_forwarder_library::msg::UncheckedForwardingConfig;
use valence_ibc_utils::types::EurekaFee;
use valence_library_utils::denoms::UncheckedDenom;

use super::strategy::Strategy;

#[async_trait]
pub(crate) trait EurekaVaultRouting {
    async fn ensure_neutron_account_fees_coverage(&self, acc: String);

    async fn route_eth_to_neutron(&self);

    async fn route_neutron_to_eth(&self);

    async fn forward_shares_for_liquidation(&self, amount: Uint128);
}

#[async_trait]
impl EurekaVaultRouting for Strategy {
    async fn ensure_neutron_account_fees_coverage(&self, acc: String) {
        let account_ntrn_balance = self
            .neutron_client
            .query_balance(&acc, NEUTRON_CHAIN_DENOM)
            .await
            .unwrap();

        if account_ntrn_balance < 10_000 {
            let delta = 10_000 - account_ntrn_balance;

            info!("Funding neutron account with {delta}untrn for ibc tx fees...");
            let transfer_rx = self
                .neutron_client
                .transfer(&acc, delta, NEUTRON_CHAIN_DENOM, None)
                .await
                .unwrap();
            self.neutron_client
                .poll_for_tx(&transfer_rx.hash)
                .await
                .unwrap();
        }
    }

    /// calculates the amount of shares that need to be liquidated to fulfill all
    /// pending withdraw obligations and forwards those shares from the position
    /// account to the withdrawal account.
    async fn forward_shares_for_liquidation(&self, amount: Uint128) {
        if amount.is_zero() {
            warn!("zero-shares liquidation request; returning");
            return;
        } else {
            let pre_fwd_position = self
                .neutron_client
                .query_balance(
                    &self.cfg.neutron.accounts.position,
                    &self.cfg.neutron.denoms.lp_token,
                )
                .await
                .unwrap();

            if pre_fwd_position < amount.u128() {
                error!("position account shares balance is insufficient: {pre_fwd_position} < {amount}. returning.");
                return;
            }

            info!("forwarding {amount}shares from position to withdraw account for liquidation");
        }

        let updated_share_fwd_cfg = UncheckedForwardingConfig {
            denom: UncheckedDenom::Native(self.cfg.neutron.denoms.lp_token.to_string()),
            max_amount: amount,
        };
        let update_cfg_msg = &valence_library_utils::msg::ExecuteMsg::<
            valence_forwarder_library::msg::FunctionMsgs,
            valence_forwarder_library::msg::LibraryConfigUpdate,
        >::UpdateConfig {
            new_config: valence_forwarder_library::msg::LibraryConfigUpdate {
                input_addr: None,
                output_addr: None,
                forwarding_configs: Some(vec![updated_share_fwd_cfg]),
                forwarding_constraints: None,
            },
        };

        info!("updating liquidation forwarder config to route {amount}shares");
        let update_rx = self
            .neutron_client
            .execute_wasm(
                &self.cfg.neutron.libraries.liquidation_forwarder,
                update_cfg_msg,
                vec![],
                None,
            )
            .await
            .unwrap();

        self.neutron_client
            .poll_for_tx(&update_rx.hash)
            .await
            .unwrap();

        let fwd_msg = &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
            valence_forwarder_library::msg::FunctionMsgs::Forward {},
        );

        let rx = self
            .neutron_client
            .execute_wasm(
                &self.cfg.neutron.libraries.liquidation_forwarder,
                fwd_msg,
                vec![],
                None,
            )
            .await
            .unwrap();
        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();
        info!("shares forwarding complete");
    }

    async fn route_eth_to_neutron(&self) {
        info!("Eureka forwarding WBTC from Ethereum to Neutron...");
        let eth_rp = self.eth_client.get_request_provider().await.unwrap();
        let erc20 = MockERC20::new(
            Address::from_str(&self.cfg.ethereum.denoms.wbtc).unwrap(),
            &eth_rp,
        );

        let eureka_transfer_lib = IBCEurekaTransfer::new(
            Address::from_str(&self.cfg.ethereum.libraries.eureka_transfer).unwrap(),
            &eth_rp,
        );

        let eth_deposit_acc_wbtc_bal = self
            .eth_client
            .query(erc20.balanceOf(Address::from_str(&self.cfg.ethereum.accounts.deposit).unwrap()))
            .await
            .unwrap()
            ._0;
        info!("[routing] eth_deposit_acc_wbtc_bal: {eth_deposit_acc_wbtc_bal}");

        let eth_deposit_acc_wbtc_u128 =
            Uint128::from_str(&eth_deposit_acc_wbtc_bal.to_string()).unwrap();
        info!("[routing] eth_deposit_acc_wbtc_u128: {eth_deposit_acc_wbtc_u128}");

        if eth_deposit_acc_wbtc_u128 < Uint128::new(10_000) {
            info!("Ethereum deposit account balance < 10_000, returning...");
            return;
        } else {
            info!("Ethereum deposit account WBTC balance: {eth_deposit_acc_wbtc_u128}");
        }

        let pre_eureka_neutron_deposit_acc_wbtc_bal = self
            .neutron_client
            .query_balance(
                &self.cfg.neutron.accounts.deposit,
                &self.cfg.neutron.denoms.wbtc,
            )
            .await
            .unwrap();

        let eureka_fees_cfg = IEurekaHandler::Fees {
            relayFee: U256::from(1),
            relayFeeRecipient: Address::from_str(&self.cfg.ethereum.accounts.deposit).unwrap(), // TODO: fix this
            quoteExpiry: 100,
        };
        let eureka_transfer_msg = eureka_transfer_lib
            .transfer(eureka_fees_cfg, "memo".into())
            .into_transaction_request();

        self.eth_client
            .execute_tx(eureka_transfer_msg)
            .await
            .unwrap();

        info!("starting polling assertion on the destination...");
        self.neutron_client
            .poll_until_expected_balance(
                &self.cfg.neutron.accounts.deposit,
                &self.cfg.neutron.denoms.wbtc,
                pre_eureka_neutron_deposit_acc_wbtc_bal + eth_deposit_acc_wbtc_u128.u128(),
                3,
                10,
            )
            .await
            .unwrap();
    }

    async fn route_neutron_to_eth(&self) {
        let withdraw_account_wbtc_bal = self
            .neutron_client
            .query_balance(
                &self.cfg.neutron.accounts.withdraw,
                &self.cfg.neutron.denoms.wbtc,
            )
            .await
            .unwrap();

        if withdraw_account_wbtc_bal == 0 {
            warn!(
                "Neutron withdraw account holds no WBTC; skipping routing from neutron to ethereum"
            );
            return;
        } else {
            info!("Routing WBTC from Neutron to Ethereum");
        }

        self.ensure_neutron_account_fees_coverage(self.cfg.neutron.accounts.withdraw.to_string())
            .await;

        let eureka_fee = EurekaFee {
            coin: cosmwasm_std::Coin {
                denom: "TODO".to_string(),
                amount: Uint128::zero(),
            },
            receiver: "TODO".to_string(),
            timeout_timestamp: u64::MIN,
        };
        info!("Initiating neutron ibc transfer");
        let neutron_ibc_transfer_msg =
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_neutron_ibc_transfer_library::msg::FunctionMsgs::EurekaTransfer {
                    eureka_fee,
                },
            );
        let rx = self
            .neutron_client
            .execute_wasm(
                &self.cfg.neutron.libraries.neutron_ibc_transfer,
                neutron_ibc_transfer_msg,
                vec![],
                None,
            )
            .await
            .unwrap();
        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        let eth_rp = self.eth_client.get_request_provider().await.unwrap();
        let erc20 = MockERC20::new(
            Address::from_str(&self.cfg.ethereum.denoms.wbtc).unwrap(),
            &eth_rp,
        );

        let pre_eureka_ethereum_withdraw_acc_wbtc_bal = self
            .eth_client
            .query(
                erc20.balanceOf(Address::from_str(&self.cfg.ethereum.accounts.withdraw).unwrap()),
            )
            .await
            .unwrap()
            ._0;

        info!("starting polling assertion on ethereum withdraw account...");
        match self
            .eth_client
            .blocking_query(
                erc20.balanceOf(Address::from_str(&self.cfg.ethereum.accounts.withdraw).unwrap()),
                |resp| resp._0 >= pre_eureka_ethereum_withdraw_acc_wbtc_bal + U256::from(1),
                3,
                10,
            )
            .await
        {
            Ok(_) => info!("eth withdraw account credited; continue..."),
            Err(_) => warn!("failed to credit eth withdraw account; continue..."),
        }
    }
}
