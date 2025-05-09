use std::{error::Error, str::FromStr};

use alloy::{
    primitives::{Address, U256},
    transports::http::reqwest,
};
use async_trait::async_trait;
use cosmwasm_std::{Coin, Uint128};
use localic_utils::{GAIA_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM};
use log::{error, info, warn};
use valence_chain_client_utils::{
    cosmos::{base_client::BaseClient, wasm_client::WasmClient},
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_e2e::utils::solidity_contracts::{IBCEurekaTransfer, IEurekaHandler, MockERC20};
use valence_forwarder_library::msg::UncheckedForwardingConfig;
use valence_ibc_utils::types::{
    eureka_types::{SkipEurekaRouteResponse, SmartRelayFeeQuote},
    EurekaFee,
};
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
        let wbtc_contract_address = Address::from_str(&self.cfg.ethereum.denoms.wbtc).unwrap();
        let eth_deposit_account_address =
            Address::from_str(&self.cfg.ethereum.accounts.deposit).unwrap();
        let eureka_transfer_address =
            Address::from_str(&self.cfg.ethereum.libraries.eureka_transfer).unwrap();

        let eth_rp = self.eth_client.get_request_provider().await.unwrap();

        let erc20 = MockERC20::new(wbtc_contract_address, &eth_rp);
        let eureka_transfer_lib = IBCEurekaTransfer::new(eureka_transfer_address, &eth_rp);

        let eth_deposit_acc_wbtc_bal = self
            .eth_client
            .query(erc20.balanceOf(eth_deposit_account_address))
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

        let skip_response = query_skip_eureka_route(
            "1",
            "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599",
            "cosmoshub-4",
            "ibc/D742E8566B0B8CC8F569D950051C09CF57988A88F0E45574BFB3079D41DE6462",
            eth_deposit_acc_wbtc_u128.to_string(),
        )
        .await
        .unwrap();
        let relative_timeout_secs = skip_response.timeout / 1_000_000_000;

        let rly_fee_recipient_address =
            Address::from_str(&skip_response.smart_relay_fee_quote.fee_payment_address).unwrap();

        let expiration_seconds =
            chrono::DateTime::parse_from_rfc3339(&skip_response.smart_relay_fee_quote.expiration)
                .unwrap()
                .timestamp() as u64;

        let eureka_fees_cfg = IEurekaHandler::Fees {
            relayFee: U256::from_str(&skip_response.smart_relay_fee_quote.fee_amount).unwrap(),
            relayFeeRecipient: rly_fee_recipient_address,
            quoteExpiry: expiration_seconds,
        };

        // build the eureka route request body
        let hub_to_neutron_pfm = serde_json::json!({
            "dest_callback":{
                "address":"cosmos1lqu9662kd4my6dww4gzp3730vew0gkwe0nl9ztjh0n5da0a8zc4swsvd22"
            },
            "wasm":{
                "contract":"cosmos1clswlqlfm8gpn7n5wu0ypu0ugaj36urlhj7yz30hn7v7mkcm2tuqy9f8s5",
                "msg":{
                    "action":{
                        "action":{
                            "ibc_transfer":{
                                "ibc_info":{
                                    "memo":"",
                                    "receiver": self.cfg.neutron.accounts.deposit,
                                    "recover_address": GAIA_CHAIN_ADMIN_ADDR,
                                    "source_channel":"channel-569" //gaia-ntrn transfer channel
                                }
                            }
                        },
                        "exact_out":false,
                        "timeout_timestamp": relative_timeout_secs.to_string()
                    }
                }
            }
        });

        info!(
            "hub to neutron pfm string:{:?}",
            hub_to_neutron_pfm.to_string()
        );

        info!("eureka fees cfg: {:?}", eureka_fees_cfg);

        let eureka_fee_u128 = Uint128::from_str(&eureka_fees_cfg.relayFee.to_string())
            .unwrap()
            .u128();

        let eureka_transfer_msg = eureka_transfer_lib
            .transfer(eureka_fees_cfg, hub_to_neutron_pfm.to_string())
            .into_transaction_request();

        match self.eth_client.execute_tx(eureka_transfer_msg).await {
            Ok(resp) => info!(
                "success executing eureka transfer: {:?}",
                resp.transaction_hash
            ),
            Err(e) => warn!("failed to execute eureka transfer: {:?}", e),
        };

        info!("starting polling assertion on the destination...");
        self.neutron_client
            .poll_until_expected_balance(
                &self.cfg.neutron.accounts.deposit,
                &self.cfg.neutron.denoms.wbtc,
                pre_eureka_neutron_deposit_acc_wbtc_bal + eth_deposit_acc_wbtc_u128.u128()
                    - eureka_fee_u128,
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

        let skip_response = query_skip_eureka_route(
            "cosmoshub-4",
            "ibc/D742E8566B0B8CC8F569D950051C09CF57988A88F0E45574BFB3079D41DE6462",
            "1",
            "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599",
            "100000",
            // withdraw_account_wbtc_bal.to_string(), this can be too small for eureka
            // api so hardcoding the value for now
        )
        .await
        .unwrap();

        info!(
            "[routing] Eureka Route Skip API response: {:?}",
            skip_response
        );

        let expiration_seconds =
            chrono::DateTime::parse_from_rfc3339(&skip_response.smart_relay_fee_quote.expiration)
                .unwrap()
                .timestamp() as u64;

        let eureka_fee = EurekaFee {
            coin: Coin {
                denom: skip_response.smart_relay_fee_quote.fee_denom,
                amount: Uint128::from_str(&skip_response.smart_relay_fee_quote.fee_amount).unwrap(),
            },
            receiver: skip_response.smart_relay_fee_quote.fee_payment_address,
            timeout_timestamp: expiration_seconds,
        };

        let pre_transfer_withdraw_acc_bals = self
            .neutron_client
            .query_balance(
                &self.cfg.neutron.accounts.withdraw,
                &self.cfg.neutron.denoms.wbtc,
            )
            .await
            .unwrap_or_default();

        info!("pre-ibc_transfer neutron withdraw account {} wbtc balance: {pre_transfer_withdraw_acc_bals}", self.cfg.neutron.accounts.withdraw);

        info!("Initiating neutron ibc transfer");
        let neutron_ibc_transfer_msg =
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_neutron_ibc_transfer_library::msg::FunctionMsgs::EurekaTransfer {
                    eureka_fee,
                },
            );
        match self
            .neutron_client
            .execute_wasm(
                &self.cfg.neutron.libraries.neutron_ibc_transfer,
                neutron_ibc_transfer_msg,
                vec![],
                None,
            )
            .await
        {
            Ok(rx) => {
                info!("rx hash: {}", rx.hash);
                self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();
            }
            Err(e) => {
                warn!("failed to initiate neutron ibc transfer: {:?}", e)
            }
        };

        let post_transfer_withdraw_acc_bals = self
            .neutron_client
            .query_balance(
                &self.cfg.neutron.accounts.withdraw,
                &self.cfg.neutron.denoms.wbtc,
            )
            .await
            .unwrap_or_default();

        info!("post-ibc_transfer neutron withdraw account wbtc balance: {post_transfer_withdraw_acc_bals}");

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

pub(crate) async fn query_skip_eureka_route(
    src_chain_id: &str,
    src_asset_denom: &str,
    dest_chain_id: &str,
    dest_chain_denom: &str,
    amount: impl Into<String>,
) -> Result<SkipEurekaRouteResponse, Box<dyn Error>> {
    let skip_api_url = "https://go.skip.build/api/skip/v2/fungible/route";

    // build the eureka route request body
    let skip_request_body = serde_json::json!({
        "source_asset_chain_id": src_chain_id,
        "source_asset_denom": src_asset_denom,
        "dest_asset_chain_id": dest_chain_id,
        "dest_asset_denom": dest_chain_denom,
        "amount_in": amount.into(),
        "allow_unsafe": true,
        "allow_multi_tx": true,
        "go_fast": true,
        "smart_relay": true,
        "smart_swap_options": {
            "split_routes": true,
            "evm_swaps": true
        },
        "experimental_features": [
            "eureka"
        ]
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(skip_api_url)
        .header("Content-Type", "application/json")
        .json(&skip_request_body)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    let op = &resp["operations"]
        .get(0)
        .ok_or("no ops in skip api response")?;
    let transfer = &op["eureka_transfer"];

    let fee_quote: SmartRelayFeeQuote =
        serde_json::from_value(transfer["smart_relay_fee_quote"].clone())?;

    let source_client = transfer["source_client"]
        .as_str()
        .ok_or("missing source_client in eureka_transfer")?
        .to_string();

    let callback_adapter_contract_address = transfer["callback_adapter_contract_address"]
        .as_str()
        .ok_or("missing callback_adapter_contract_address")?
        .to_string();

    let entry_contract_address = transfer["entry_contract_address"]
        .as_str()
        .ok_or("missing entry_contract_address")?
        .to_string();

    let secs = resp["estimated_route_duration_seconds"]
        .as_u64()
        .ok_or("missing estimated_route_duration_seconds")?;
    let timeout = secs.checked_mul(1_000_000_000).ok_or("duration overflow")?;

    Ok(SkipEurekaRouteResponse {
        smart_relay_fee_quote: fee_quote,
        timeout,
        source_client,
        callback_adapter_contract_address,
        entry_contract_address,
    })
}
