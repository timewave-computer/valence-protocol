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
use valence_e2e::utils::{
    solidity_contracts::{CCTPTransfer, MockERC20},
    UUSDC_DENOM,
};
use valence_forwarder_library::msg::UncheckedForwardingConfig;
use valence_library_utils::denoms::UncheckedDenom;

use super::strategy::Strategy;

#[async_trait]
pub(crate) trait EthereumVaultRouting {
    async fn ensure_neutron_account_fees_coverage(&self, acc: String);

    async fn route_eth_to_neutron(&self);

    async fn route_neutron_to_eth(&self);

    async fn forward_shares_for_liquidation(&self, amount: Uint128);
}

#[async_trait]
impl EthereumVaultRouting for Strategy {
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
        unimplemented!()
    }

    async fn route_neutron_to_eth(&self) {
        unimplemented!()
    }
}
