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
pub trait EthereumVaultRouting {
    async fn ensure_neutron_account_fees_coverage(&self, acc: String);

    async fn route_noble_to_eth(&self);

    async fn route_eth_to_noble(&self);

    async fn route_neutron_to_noble(&self);

    async fn route_noble_to_neutron(&self);

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

        if account_ntrn_balance < self.cfg.neutron.min_ibc_fee.u128() {
            let delta = self.cfg.neutron.min_ibc_fee.u128() - account_ntrn_balance;

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

    /// IBC-transfers funds from Neutron withdraw account to noble outbound ica
    async fn route_neutron_to_noble(&self) {
        let withdraw_account_usdc_bal = self
            .neutron_client
            .query_balance(
                &self.cfg.neutron.accounts.withdraw,
                &self.cfg.neutron.denoms.usdc,
            )
            .await
            .unwrap();

        if withdraw_account_usdc_bal == 0 {
            warn!("Neutron withdraw account holds no USDC; skipping routing from neutron to noble");
            return;
        } else {
            info!("Routing USDC from Neutron to Noble");
        }

        self.ensure_neutron_account_fees_coverage(self.cfg.neutron.accounts.withdraw.to_string())
            .await;

        let noble_outbound_ica_usdc_bal = self
            .noble_client
            .query_balance(
                &self.cfg.neutron.accounts.noble_outbound_ica.remote_addr,
                UUSDC_DENOM,
            )
            .await
            .unwrap();

        info!("Initiating neutron ibc transfer");
        let neutron_ibc_transfer_msg =
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_neutron_ibc_transfer_library::msg::FunctionMsgs::IbcTransfer {},
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

        info!("starting polling assertion on noble outbound ica...");
        self.noble_client
            .poll_until_expected_balance(
                &self.cfg.neutron.accounts.noble_outbound_ica.remote_addr,
                UUSDC_DENOM,
                noble_outbound_ica_usdc_bal + withdraw_account_usdc_bal,
                3,
                10,
            )
            .await
            .unwrap();
    }

    /// CCTP-transfers funds from Ethereum deposit account to Noble inbound ica
    async fn route_eth_to_noble(&self) {
        info!("CCTP forwarding USDC from Ethereum to Noble...");
        let eth_rp = self.eth_client.get_request_provider().await.unwrap();
        let erc20 = MockERC20::new(
            Address::from_str(&self.cfg.ethereum.denoms.usdc_erc20).unwrap(),
            &eth_rp,
        );
        let cctp_transfer_contract = CCTPTransfer::new(
            Address::from_str(&self.cfg.ethereum.libraries.cctp_forwarder).unwrap(),
            &eth_rp,
        );

        let eth_deposit_acc_usdc_bal = self
            .eth_client
            .query(erc20.balanceOf(Address::from_str(&self.cfg.ethereum.accounts.deposit).unwrap()))
            .await
            .unwrap()
            ._0;
        info!("[routing] eth_deposit_acc_usdc_bal: {eth_deposit_acc_usdc_bal}");

        let eth_deposit_acc_usdc_u128 =
            Uint128::from_str(&eth_deposit_acc_usdc_bal.to_string()).unwrap();
        info!("[routing] eth_deposit_acc_usdc_u128: {eth_deposit_acc_usdc_u128}");

        if eth_deposit_acc_usdc_u128 < Uint128::new(10_000) {
            info!("Ethereum deposit account balance < 10_000, returning...");
            return;
        } else {
            info!("Ethereum deposit account USDC balance: {eth_deposit_acc_usdc_u128}");
        }

        let pre_cctp_inbound_ica_usdc_bal = self
            .noble_client
            .query_balance(
                &self.cfg.neutron.accounts.noble_inbound_ica.remote_addr,
                UUSDC_DENOM,
            )
            .await
            .unwrap();

        self.eth_client
            .execute_tx(cctp_transfer_contract.transfer().into_transaction_request())
            .await
            .unwrap();

        info!("starting polling assertion on the destination...");
        self.noble_client
            .poll_until_expected_balance(
                &self.cfg.neutron.accounts.noble_inbound_ica.remote_addr,
                UUSDC_DENOM,
                pre_cctp_inbound_ica_usdc_bal + eth_deposit_acc_usdc_u128.u128(),
                3,
                10,
            )
            .await
            .unwrap();
    }

    /// CCTP-transfers funds from Noble outbound ica to Ethereum withdraw account
    async fn route_noble_to_eth(&self) {
        let pre_cctp_noble_outbound_ica_usdc_bal = self
            .noble_client
            .query_balance(
                &self.cfg.neutron.accounts.noble_outbound_ica.remote_addr,
                UUSDC_DENOM,
            )
            .await
            .unwrap();

        if pre_cctp_noble_outbound_ica_usdc_bal == 0 {
            warn!("Noble outbound ICA account must have USDC in order to CCTP forward to Ethereum; returning");
            return;
        } else {
            info!("CCTP forwarding USDC from Noble to Ethereum...");
        }

        let eth_rp = self.eth_client.get_request_provider().await.unwrap();
        let erc20 = MockERC20::new(
            Address::from_str(&self.cfg.ethereum.denoms.usdc_erc20).unwrap(),
            &eth_rp,
        );

        let pre_cctp_ethereum_withdraw_acc_usdc_bal = self
            .eth_client
            .query(
                erc20.balanceOf(Address::from_str(&self.cfg.ethereum.accounts.withdraw).unwrap()),
            )
            .await
            .unwrap()
            ._0;

        info!("updating noble inbound ica cctp routing cfg");

        let update_cfg_msg = &valence_library_utils::msg::ExecuteMsg::<
            valence_ica_cctp_transfer::msg::FunctionMsgs,
            valence_ica_cctp_transfer::msg::LibraryConfigUpdate,
        >::UpdateConfig {
            new_config: valence_ica_cctp_transfer::msg::LibraryConfigUpdate {
                input_addr: None,
                amount: Some(pre_cctp_noble_outbound_ica_usdc_bal.into()),
                denom: None,
                destination_domain_id: None,
                mint_recipient: None,
            },
        };

        let update_rx = self
            .neutron_client
            .execute_wasm(
                &self.cfg.neutron.libraries.noble_cctp_transfer,
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

        info!("NOBLE->ETH cctp transfer update cfg complete; executing outbound transfer");

        self.ensure_neutron_account_fees_coverage(
            self.cfg
                .neutron
                .accounts
                .noble_outbound_ica
                .library_account
                .to_string(),
        )
        .await;

        let neutron_ica_cctp_transfer_msg =
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_ica_cctp_transfer::msg::FunctionMsgs::Transfer {},
            );
        let rx = self
            .neutron_client
            .execute_wasm(
                &self.cfg.neutron.libraries.noble_cctp_transfer,
                neutron_ica_cctp_transfer_msg,
                vec![],
                None,
            )
            .await
            .unwrap();
        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        self.eth_client
            .blocking_query(
                erc20.balanceOf(Address::from_str(&self.cfg.ethereum.accounts.withdraw).unwrap()),
                |resp| resp._0 >= pre_cctp_ethereum_withdraw_acc_usdc_bal + U256::from(1),
                3,
                10,
            )
            .await
            .unwrap();
    }

    /// IBC-transfers funds from noble inbound ica into neutron deposit account
    async fn route_noble_to_neutron(&self) {
        let noble_inbound_ica_balance = self
            .noble_client
            .query_balance(
                &self.cfg.neutron.accounts.noble_inbound_ica.remote_addr,
                UUSDC_DENOM,
            )
            .await
            .unwrap();

        if noble_inbound_ica_balance == 0 {
            warn!("Noble inbound ICA account must have enough USDC to route funds to Neutron deposit acc; returning");
            return;
        } else {
            info!("noble inbound ica USDC balance: {noble_inbound_ica_balance}");
        }

        info!("updating noble inbound ica transfer cfg");

        let update_cfg_msg = &valence_library_utils::msg::ExecuteMsg::<
            valence_ica_ibc_transfer::msg::FunctionMsgs,
            valence_ica_ibc_transfer::msg::LibraryConfigUpdate,
        >::UpdateConfig {
            new_config: valence_ica_ibc_transfer::msg::LibraryConfigUpdate {
                input_addr: None,
                amount: Some(noble_inbound_ica_balance.into()),
                denom: None,
                receiver: None,
                memo: None,
                remote_chain_info: None,
                denom_to_pfm_map: None,
                eureka_config: valence_library_utils::OptionUpdate::Set(None),
            },
        };

        let update_rx = self
            .neutron_client
            .execute_wasm(
                &self.cfg.neutron.libraries.noble_inbound_transfer,
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

        info!("update cfg complete; executing inbound transfer");

        let neutron_deposit_acc_pre_transfer_bal = self
            .neutron_client
            .query_balance(
                &self.cfg.neutron.accounts.deposit,
                &self.cfg.neutron.denoms.usdc,
            )
            .await
            .unwrap();

        self.ensure_neutron_account_fees_coverage(
            self.cfg
                .neutron
                .accounts
                .noble_inbound_ica
                .library_account
                .to_string(),
        )
        .await;

        let transfer_msg = &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
            valence_ica_ibc_transfer::msg::FunctionMsgs::Transfer {},
        );
        let rx = self
            .neutron_client
            .execute_wasm(
                &self.cfg.neutron.libraries.noble_inbound_transfer,
                transfer_msg,
                vec![],
                None,
            )
            .await
            .unwrap();
        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        info!("polling neutron deposit account for USDC balance...");
        self.neutron_client
            .poll_until_expected_balance(
                &self.cfg.neutron.accounts.deposit,
                &self.cfg.neutron.denoms.usdc,
                neutron_deposit_acc_pre_transfer_bal + noble_inbound_ica_balance,
                3,
                10,
            )
            .await
            .unwrap();
    }
}
