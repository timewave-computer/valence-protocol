use std::str::FromStr;

use alloy::primitives::U256;
use async_trait::async_trait;
use cosmwasm_std::Uint128;
use localic_utils::NEUTRON_CHAIN_DENOM;
use log::{info, warn};
use valence_chain_client_utils::{
    cosmos::{base_client::BaseClient, wasm_client::WasmClient},
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_e2e::utils::{
    solidity_contracts::{CCTPTransfer, MockERC20},
    UUSDC_DENOM,
};
use valence_forwarder_library::msg::UncheckedForwardingConfig;

use super::client::Strategist;

#[async_trait]
pub trait EthereumVaultRouting {
    async fn route_noble_to_eth(&self);

    async fn route_eth_to_noble(&self);

    async fn route_neutron_to_noble(&self);

    async fn route_noble_to_neutron(&self);

    async fn forward_shares_for_liquidation(&self, amount: Uint128);
}

#[async_trait]
impl EthereumVaultRouting for Strategist {
    /// calculates the amount of shares that need to be liquidated to fulfill all
    /// pending withdraw obligations and forwards those shares from the position
    /// account to the withdrawal account.
    async fn forward_shares_for_liquidation(&self, amount: Uint128) {
        if amount.is_zero() {
            info!("[STRATEGIST] zero-shares liquidation request; returning");
            return;
        }

        let new_fwd_cfgs = vec![UncheckedForwardingConfig {
            denom: valence_library_utils::denoms::UncheckedDenom::Native(
                self.lp_token_denom.to_string(),
            ),
            max_amount: amount,
        }];

        info!(
            "[STRATEGIST] updating liquidation forwarder cfg to: {:?}",
            new_fwd_cfgs
        );

        let update_cfg_msg = &valence_library_utils::msg::ExecuteMsg::<
            valence_forwarder_library::msg::FunctionMsgs,
            valence_forwarder_library::msg::LibraryConfigUpdate,
        >::UpdateConfig {
            new_config: valence_forwarder_library::msg::LibraryConfigUpdate {
                input_addr: None,
                output_addr: None,
                forwarding_configs: Some(new_fwd_cfgs),
                forwarding_constraints: None,
            },
        };

        let update_rx = self
            .neutron_client
            .execute_wasm(
                &self.neutron_program_libraries.liquidation_forwarder,
                update_cfg_msg,
                vec![],
            )
            .await
            .unwrap();

        self.neutron_client
            .poll_for_tx(&update_rx.hash)
            .await
            .unwrap();

        info!("[STRATEGIST] update cfg complete; executing forwarding");

        let pre_fwd_position = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .position_account
                    .to_string()
                    .unwrap(),
                &self.lp_token_denom,
            )
            .await
            .unwrap();

        info!(
            "[STRATEGIST] pre forward position account shares balance: {:?}",
            pre_fwd_position
        );

        let fwd_msg = &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
            valence_forwarder_library::msg::FunctionMsgs::Forward {},
        );
        let rx = self
            .neutron_client
            .execute_wasm(
                &self.neutron_program_libraries.liquidation_forwarder,
                fwd_msg,
                vec![],
            )
            .await
            .unwrap();
        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        let post_fwd_position = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .position_account
                    .to_string()
                    .unwrap(),
                &self.lp_token_denom,
            )
            .await
            .unwrap();

        info!(
            "[STRATEGIST] post forward position account shares balance: {:?}",
            post_fwd_position
        );

        info!("[STRATEGIST] fwd complete!");
    }

    /// IBC-transfers funds from Neutron withdraw account to noble outbound ica
    async fn route_neutron_to_noble(&self) {
        let noble_outbound_ica_usdc_bal = self
            .noble_client
            .query_balance(
                &self.neutron_program_accounts.noble_outbound_ica.remote_addr,
                UUSDC_DENOM,
            )
            .await
            .unwrap();
        let withdraw_account_usdc_bal = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                &self.uusdc_on_neutron_denom,
            )
            .await
            .unwrap();

        let withdraw_account_ntrn_bal = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                NEUTRON_CHAIN_DENOM,
            )
            .await
            .unwrap();

        if withdraw_account_usdc_bal == 0 {
            warn!("[STRATEGIST] withdraw account must have USDC in order to route funds to noble; returning");
            return;
        }

        // if withdraw_account_ntrn_bal == 0 {
        //     warn!("[STRATEGIST] withdraw account must have NTRN in order to route funds to noble; returning");
        //     return;
        // }

        info!("[STRATEGIST] routing USDC to noble...");
        let transfer_rx = self
            .neutron_client
            .transfer(
                &self
                    .neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                110_000,
                NEUTRON_CHAIN_DENOM,
                None,
            )
            .await
            .unwrap();
        self.neutron_client
            .poll_for_tx(&transfer_rx.hash)
            .await
            .unwrap();

        let neutron_ibc_transfer_msg =
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_neutron_ibc_transfer_library::msg::FunctionMsgs::IbcTransfer {},
            );
        let rx = self
            .neutron_client
            .execute_wasm(
                &self.neutron_program_libraries.neutron_ibc_transfer,
                neutron_ibc_transfer_msg,
                vec![],
            )
            .await
            .unwrap();
        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        info!("starting polling assertion on noble outbound ica...");
        self.noble_client
            .poll_until_expected_balance(
                &self
                    .neutron_program_accounts
                    .noble_outbound_ica
                    .remote_addr
                    .to_string(),
                UUSDC_DENOM,
                noble_outbound_ica_usdc_bal + withdraw_account_usdc_bal,
                1,
                10,
            )
            .await
            .unwrap();
    }

    /// CCTP-transfers funds from Ethereum deposit account to Noble inbound ica
    async fn route_eth_to_noble(&self) {
        info!("[STRATEGIST] CCTP forwarding USDC from Ethereum to Noble...");
        let eth_rp = self.eth_client.get_request_provider().await.unwrap();
        let erc20 = MockERC20::new(self.ethereum_usdc_erc20, &eth_rp);

        let eth_deposit_acc_usdc_bal = self
            .eth_client
            .query(erc20.balanceOf(self.eth_program_accounts.deposit))
            .await
            .unwrap()
            ._0;
        let eth_deposit_acc_usdc_u128 =
            Uint128::from_str(&eth_deposit_acc_usdc_bal.to_string()).unwrap();
        info!(
            "[STRATEGIST] Ethereum deposit account USDC balance: {:?}",
            eth_deposit_acc_usdc_u128
        );
        if eth_deposit_acc_usdc_u128 < Uint128::new(100_000) {
            info!("[STRATEGIST] Ethereum deposit account balance < 100_000, returning...");
            return;
        }

        let cctp_transfer_contract = CCTPTransfer::new(self.cctp_transfer_lib, &eth_rp);

        let remote_ica_addr = self
            .neutron_program_accounts
            .noble_inbound_ica
            .remote_addr
            .to_string();

        let pre_cctp_inbound_ica_usdc_bal = self
            .noble_client
            .query_balance(&remote_ica_addr, UUSDC_DENOM)
            .await
            .unwrap();

        let signer_addr = self.eth_client.signer.address();
        let signed_tx = cctp_transfer_contract
            .transfer()
            .into_transaction_request()
            .from(signer_addr);
        self.eth_client.execute_tx(signed_tx).await.unwrap();

        info!("starting polling assertion on the destination...");
        self.noble_client
            .poll_until_expected_balance(
                &remote_ica_addr,
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
        info!("[STRATEGIST] CCTP forwarding USDC from Noble to Ethereum...");

        let eth_rp = self.eth_client.get_request_provider().await.unwrap();
        let erc20 = MockERC20::new(self.ethereum_usdc_erc20, &eth_rp);
        let pre_cctp_ethereum_withdraw_acc_usdc_bal = self
            .eth_client
            .query(erc20.balanceOf(self.eth_program_accounts.withdraw))
            .await
            .unwrap()
            ._0;
        let pre_cctp_noble_outbound_ica_usdc_bal = self
            .noble_client
            .query_balance(
                &self.neutron_program_accounts.noble_outbound_ica.remote_addr,
                UUSDC_DENOM,
            )
            .await
            .unwrap();

        if pre_cctp_noble_outbound_ica_usdc_bal == 0 {
            warn!("[STRATEGIST] Noble outbound ICA account must have USDC in order to CCTP forward to Ethereum; returning");
            return;
        }

        let transfer_tx = self
            .neutron_client
            .transfer(
                &self
                    .neutron_program_accounts
                    .noble_outbound_ica
                    .library_account
                    .to_string()
                    .unwrap(),
                110_000,
                NEUTRON_CHAIN_DENOM,
                None,
            )
            .await
            .unwrap();
        self.neutron_client
            .poll_for_tx(&transfer_tx.hash)
            .await
            .unwrap();

        let neutron_ica_cctp_transfer_msg =
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_ica_cctp_transfer::msg::FunctionMsgs::Transfer {},
            );
        let rx = self
            .neutron_client
            .execute_wasm(
                &self.neutron_program_libraries.noble_cctp_transfer,
                neutron_ica_cctp_transfer_msg,
                vec![],
            )
            .await
            .unwrap();
        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        self.blocking_erc20_expected_balance_query(
            self.eth_program_accounts.withdraw,
            pre_cctp_ethereum_withdraw_acc_usdc_bal + U256::from(1),
            1,
            10,
        )
        .await;
    }

    /// IBC-transfers funds from noble inbound ica into neutron deposit account
    async fn route_noble_to_neutron(&self) {
        let noble_inbound_ica_balance = self
            .noble_client
            .query_balance(
                &self.neutron_program_accounts.noble_inbound_ica.remote_addr,
                UUSDC_DENOM,
            )
            .await
            .unwrap();

        info!("[STRATEGIST] noble inbound ica USDC balance: {noble_inbound_ica_balance}");

        if noble_inbound_ica_balance < 100_000 {
            warn!("Noble inbound ICA account must have enough USDC to route funds to Neutron deposit acc; returning");
            return;
        }

        info!("[STRATEGIST] updating noble inbound ica transfer cfg");

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
            },
        };

        let update_rx = self
            .neutron_client
            .execute_wasm(
                &self.neutron_program_libraries.noble_inbound_transfer,
                update_cfg_msg,
                vec![],
            )
            .await
            .unwrap();
        self.neutron_client
            .poll_for_tx(&update_rx.hash)
            .await
            .unwrap();

        info!("[STRATEGIST] update cfg complete; executing inbound transfer");

        let neutron_deposit_acc_pre_transfer_bal = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .deposit_account
                    .to_string()
                    .unwrap(),
                &self.uusdc_on_neutron_denom,
            )
            .await
            .unwrap();

        let neutron_inbound_transfer_ntrn_bal = self
            .neutron_client
            .query_balance(
                &self.neutron_program_libraries.noble_inbound_transfer,
                NEUTRON_CHAIN_DENOM,
            )
            .await
            .unwrap();
        if neutron_inbound_transfer_ntrn_bal < 100_000 {
            info!("[STRATEGIST] funding noble inbound transfer with some ntrn for ibc call");
            let transfer_rx = self
                .neutron_client
                .transfer(
                    &self.neutron_program_libraries.noble_inbound_transfer,
                    100_000,
                    NEUTRON_CHAIN_DENOM,
                    None,
                )
                .await
                .unwrap();
            self.neutron_client
                .poll_for_tx(&transfer_rx.hash)
                .await
                .unwrap();
        }

        let transfer_msg = &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
            valence_ica_ibc_transfer::msg::FunctionMsgs::Transfer {},
        );
        let rx = self
            .neutron_client
            .execute_wasm(
                &self.neutron_program_libraries.noble_inbound_transfer,
                transfer_msg,
                vec![],
            )
            .await
            .unwrap();
        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        info!("[STRATEGIST] polling neutron deposit account for USDC balance...");
        self.neutron_client
            .poll_until_expected_balance(
                &self
                    .neutron_program_accounts
                    .deposit_account
                    .to_string()
                    .unwrap(),
                &self.uusdc_on_neutron_denom,
                neutron_deposit_acc_pre_transfer_bal + noble_inbound_ica_balance,
                2,
                10,
            )
            .await
            .unwrap();
    }
}
