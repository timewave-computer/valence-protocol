use std::{error::Error, str::FromStr, time::Duration};

use alloy::{
    hex::ToHexExt,
    primitives::{Address, U256},
    providers::Provider,
    rpc::types::Filter,
    sol_types::SolEvent,
};

use bech32::{encode, Bech32};
use cosmwasm_std::{from_base64, Uint128};
use hex::FromHex;
use log::{info, warn};
use tokio::task::JoinHandle;
use valence_chain_client_utils::{
    cosmos::base_client::BaseClient,
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
    noble::NobleClient,
};
use valence_e2e::utils::{
    ethereum,
    parse::get_rpc_address_from_logs,
    solidity_contracts::{MockERC20, MockTokenMessenger::DepositForBurn},
    NOBLE_CHAIN_ADMIN_ADDR, NOBLE_CHAIN_ID, UUSDC_DENOM,
};

pub struct MockCctpRelayer {
    eth_client: EthereumClient,
    noble_client: NobleClient,
}

impl MockCctpRelayer {
    pub fn new(eth_client: EthereumClient, noble_client: NobleClient) -> Self {
        Self {
            eth_client,
            noble_client,
        }
    }

    pub async fn start_noble(self, destination_erc20: Address) -> JoinHandle<()> {
        let mut latest_block = self
            .noble_client
            .latest_block_header()
            .await
            .unwrap()
            .height;

        let rpc_addr = get_rpc_address_from_logs(NOBLE_CHAIN_ID).unwrap();

        tokio::spawn(async move {
            loop {
                let current_block = self
                    .noble_client
                    .latest_block_header()
                    .await
                    .unwrap()
                    .height;

                for i in latest_block..current_block {
                    info!("[CCTP NOBLE] Polling block #{i}...");

                    let results = self
                        .noble_client
                        .block_results(&rpc_addr, i as u32)
                        .await
                        .unwrap();

                    if let Some(r) = results.txs_results {
                        for result in r {
                            for event in result.events {
                                if event.kind == "circle.cctp.v1.DepositForBurn" {
                                    info!("[CCTP NOBLE] CCTP burn event detected!");

                                    let mut amount = "".to_string();
                                    let mut mint_recipient = "".to_string();
                                    let mut destination_domain = "".to_string();
                                    let mut destination_token_messenger = "".to_string();

                                    for attribute in event.attributes {
                                        let key = attribute.key_str().unwrap().to_string();
                                        let value = attribute.value_str().unwrap().to_string();
                                        if key == "amount" {
                                            amount = value;
                                            info!("\t[CCTP NOBLE] amount: {amount}");
                                        } else if key == "mint_recipient" {
                                            mint_recipient = value;
                                            info!(
                                                "\t[CCTP NOBLE] mint_recipient: {mint_recipient}"
                                            );
                                        } else if key == "destination_domain" {
                                            destination_domain = value;
                                            info!("\t[CCTP NOBLE] destination_domain: {destination_domain}");
                                        } else if key == "destination_token_messenger" {
                                            destination_token_messenger = value;
                                            info!("\t[CCTP NOBLE] destination_token_messenger: {destination_token_messenger}");
                                        }
                                    }

                                    if !amount.is_empty()
                                        && !mint_recipient.is_empty()
                                        && !destination_domain.is_empty()
                                        && !destination_token_messenger.is_empty()
                                    {
                                        info!("[CCTP NOBLE] minting {amount}USDC to domain #{destination_domain} recipient {mint_recipient}");
                                        let eth_rp =
                                            self.eth_client.get_request_provider().await.unwrap();

                                        let mock_erc20 = MockERC20::new(destination_erc20, &eth_rp);

                                        let amount_stripped = amount.strip_prefix('"').unwrap();
                                        let amount_stripped =
                                            amount_stripped.strip_suffix('"').unwrap();

                                        let recipient_stripped =
                                            mint_recipient.strip_suffix('"').unwrap();
                                        let recipient_stripped =
                                            recipient_stripped.strip_prefix('"').unwrap();

                                        let amt = Uint128::from_str(&amount_stripped).unwrap();
                                        let to = from_base64(recipient_stripped).unwrap();

                                        let address_bytes = &to[12..];

                                        let dest_addr = Address::from_slice(address_bytes);

                                        let pre_mint_balance = self
                                            .eth_client
                                            .query(mock_erc20.balanceOf(dest_addr))
                                            .await
                                            .unwrap();

                                        self.eth_client
                                            .execute_tx(
                                                mock_erc20
                                                    .mint(dest_addr, U256::from(amt.u128()))
                                                    .into_transaction_request(),
                                            )
                                            .await
                                            .unwrap();

                                        let post_mint_balance = self
                                            .eth_client
                                            .query(mock_erc20.balanceOf(dest_addr))
                                            .await
                                            .unwrap();

                                        let delta = post_mint_balance._0 - pre_mint_balance._0;
                                        info!("[CCTP NOBLE] successfully minted {delta} tokens to eth address {dest_addr}");
                                    }
                                }
                            }
                        }
                    }
                }

                latest_block = current_block;
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        })
    }

    pub async fn start_eth(self, messenger: Address) -> JoinHandle<()> {
        info!("[CCTP ETH] Getting request provider...");
        let provider = self
            .eth_client
            .get_request_provider()
            .await
            .expect("could not get provider");

        let filter = Filter::new().address(messenger);
        info!("[CCTP ETH] Created filter: {:?}", filter);

        tokio::spawn(async move {
            loop {
                info!("[CCTP ETH] polling logs...");
                match provider.get_logs(&filter).await {
                    Ok(logs) => {
                        if logs.is_empty() {
                            info!("[CCTP ETH] no logs found");
                        } else {
                            info!("[CCTP ETH] Found {} logs", logs.len());
                            for (i, log) in logs.iter().enumerate() {
                                let alloy_log = alloy::primitives::Log::new(
                                    log.address(),
                                    log.topics().into(),
                                    log.data().clone().data,
                                )
                                .unwrap_or_default();

                                match DepositForBurn::decode_log(&alloy_log, false) {
                                    Ok(val) => {
                                        info!("decoded deposit for burn log: {:?}", val);
                                        let destination_addr =
                                            decode_mint_recipient_to_noble_address(
                                                &val.mintRecipient.encode_hex(),
                                            )
                                            .unwrap();
                                        let amount = val.amount;
                                        let tx_response = self
                                            .noble_client
                                            .mint_fiat(
                                                NOBLE_CHAIN_ADMIN_ADDR,
                                                &destination_addr,
                                                &amount.to_string(),
                                                UUSDC_DENOM,
                                            )
                                            .await
                                            .unwrap();
                                        self.noble_client
                                            .poll_for_tx(&tx_response.hash)
                                            .await
                                            .unwrap();
                                        info!("[CCTP ETH] Minted {UUSDC_DENOM} to {destination_addr}: {:?}", tx_response);
                                    }
                                    Err(e) => {
                                        warn!("failed to decode the deposit for burn log: {:?}", e)
                                    }
                                }
                            }
                        }
                    }
                    Err(err) => {
                        warn!("[CCTP ETH] Error polling logs: {:?}", err);
                    }
                }

                info!("[CCTP ETH] Sleeping for 1 second before next poll");
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        })
    }
}

fn decode_mint_recipient_to_noble_address(
    mint_recipient_hex: &str,
) -> Result<String, Box<dyn Error>> {
    let (hrp, _) = bech32::decode(NOBLE_CHAIN_ADMIN_ADDR)?;

    let trimmed_hex = mint_recipient_hex.trim_start_matches('0');

    let bytes = Vec::from_hex(trimmed_hex)?;

    let noble_address = encode::<Bech32>(hrp, &bytes)?;

    Ok(noble_address)
}
