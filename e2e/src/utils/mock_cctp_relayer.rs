use std::{collections::HashSet, error::Error, str::FromStr, time::Duration};

use alloy::{
    hex::ToHexExt,
    primitives::{Address, Log, U256},
    providers::Provider,
    rpc::types::Filter,
    signers::local::{coins_bip39::English, MnemonicBuilder},
    sol_types::SolEvent,
};

use crate::{
    async_run,
    utils::{
        parse::get_chain_field_from_local_ic_log,
        solidity_contracts::{MockERC20, MockTokenMessenger::DepositForBurn},
        NOBLE_CHAIN_ADMIN_ADDR, NOBLE_CHAIN_ID, UUSDC_DENOM,
    },
};
use bech32::{encode, Bech32};
use cosmwasm_std::{from_base64, Uint128};
use hex::FromHex;
use log::{info, warn};
use tokio::runtime::Runtime;
use valence_chain_client_utils::{
    cosmos::base_client::BaseClient,
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
    noble::NobleClient,
};

use super::{
    parse::get_grpc_address_and_port_from_url, ADMIN_MNEMONIC, DEFAULT_ANVIL_RPC_ENDPOINT,
    NOBLE_CHAIN_DENOM,
};

const POLLING_PERIOD: Duration = Duration::from_secs(2);

pub struct MockCctpRelayer {
    eth_client: EthereumClient,
    noble_client: NobleClient,
    state: RelayerState,
}

struct RelayerState {
    // last processed block on noble
    noble_last_block: i64,
    // noble rpc address
    noble_rpc_addr: String,
    // processed events cache to avoid double processing
    eth_processed_events: HashSet<Vec<u8>>,
    // ethereum filter to poll for events
    eth_filter: Filter,
    // ethereum destination erc20 address
    eth_destination_erc20: Address,
}

impl MockCctpRelayer {
    pub fn new(
        rt: &Runtime,
        messenger: Address,
        destination_erc20: Address,
    ) -> Result<Self, Box<dyn Error>> {
        let grpc_addr = get_chain_field_from_local_ic_log(NOBLE_CHAIN_ID, "grpc_address")?;
        let (grpc_url, grpc_port) = get_grpc_address_and_port_from_url(&grpc_addr)?;

        let (noble_client, latest_noble_block) = async_run!(rt, {
            let client = NobleClient::new(
                &grpc_url,
                &grpc_port.to_string(),
                ADMIN_MNEMONIC,
                NOBLE_CHAIN_ID,
                NOBLE_CHAIN_DENOM,
            )
            .await
            .expect("failed to create noble client");
            let latest_block = client
                .latest_block_header()
                .await
                .expect("failed to get latest block header")
                .height;
            (client, latest_block)
        });

        let signer = MnemonicBuilder::<English>::default()
            .phrase("test test test test test test test test test test test junk")
            .index(5)? // derive the mnemonic at a different index to avoid nonce issues
            .build()?;

        let eth_client = EthereumClient {
            rpc_url: DEFAULT_ANVIL_RPC_ENDPOINT.to_string(),
            signer,
        };

        let noble_rpc_addr = get_chain_field_from_local_ic_log(NOBLE_CHAIN_ID, "rpc_address")
            .expect("Failed to find rpc_address field for noble chain");

        Ok(Self {
            eth_client,
            noble_client,
            state: RelayerState {
                noble_last_block: latest_noble_block,
                noble_rpc_addr,
                eth_processed_events: HashSet::new(),
                eth_filter: Filter::new().address(messenger),
                eth_destination_erc20: destination_erc20,
            },
        })
    }

    pub async fn start(mut self) {
        info!("[CCTP MOCK RELAY] Starting Eth<->Noble cctp relayer...");

        loop {
            info!("[CCTP RELAY] loop");

            if let Err(e) = self.poll_noble().await {
                warn!("[CCTP MOCK RELAY] Noble polling error: {:?}", e);
            }

            if let Err(e) = self.poll_ethereum().await {
                warn!("[CCTP MOCK RELAY] Ethereum polling error: {:?}", e);
            }

            tokio::time::sleep(POLLING_PERIOD).await;
        }
    }

    async fn mint_evm(
        &self,
        amount: String,
        mint_recipient: String,
        destination_domain: String,
        destination_erc20: Address,
    ) -> Result<(), Box<dyn Error>> {
        info!("[CCTP NOBLE] minting {amount}USDC to domain #{destination_domain} recipient {mint_recipient}");
        let eth_rp = self
            .eth_client
            .get_request_provider()
            .await
            .expect("failed to get eth request provider");

        let mock_erc20 = MockERC20::new(destination_erc20, &eth_rp);

        let amt = Uint128::from_str(&amount)?;
        let to = from_base64(mint_recipient)?;

        let dest_addr = Address::from_slice(&to[12..]);

        let pre_mint_balance = self
            .eth_client
            .query(mock_erc20.balanceOf(dest_addr))
            .await
            .expect("failed to query eth balance");

        let mint_tx = self
            .eth_client
            .execute_tx(
                mock_erc20
                    .mint(dest_addr, U256::from(amt.u128()))
                    .into_transaction_request(),
            )
            .await
            .expect("failed to mint usdc on eth");

        let _ = eth_rp
            .get_transaction_receipt(mint_tx.transaction_hash)
            .await;

        let post_mint_balance = self
            .eth_client
            .query(mock_erc20.balanceOf(dest_addr))
            .await
            .expect("failed to query eth balance");

        let delta = post_mint_balance._0 - pre_mint_balance._0;
        info!("[CCTP NOBLE] successfully minted {delta} tokens to eth address {dest_addr}");

        Ok(())
    }

    async fn mint_noble(&self, val: Log<DepositForBurn>) -> Result<(), Box<dyn Error>> {
        info!("decoded deposit for burn log: {:?}", val);
        let destination_addr =
            decode_mint_recipient_to_noble_address(&val.mintRecipient.encode_hex())?;

        let tx_response = self
            .noble_client
            .mint_fiat(
                NOBLE_CHAIN_ADMIN_ADDR,
                &destination_addr,
                &val.amount.to_string(),
                UUSDC_DENOM,
            )
            .await
            .expect("failed to mint usdc on noble");
        self.noble_client
            .poll_for_tx(&tx_response.hash)
            .await
            .expect("failed to poll for mint tx on noble");
        info!(
            "[CCTP ETH] Minted {UUSDC_DENOM} to {destination_addr}: {:?}",
            tx_response
        );

        Ok(())
    }

    async fn poll_noble(&mut self) -> Result<(), Box<dyn Error>> {
        // query the current block to process the delta
        let current_block = self
            .noble_client
            .latest_block_header()
            .await
            .unwrap()
            .height;

        // process all blocks from last processed block to current block
        for i in self.state.noble_last_block..current_block {
            self.process_noble_block(
                &self.state.noble_rpc_addr,
                i as u32,
                self.state.eth_destination_erc20,
            )
            .await?;
        }

        // update the last processed block and return
        self.state.noble_last_block = current_block;

        Ok(())
    }

    async fn poll_ethereum(&mut self) -> Result<(), Box<dyn Error>> {
        let provider = self
            .eth_client
            .get_request_provider()
            .await
            .expect("could not get provider");

        // fetch the logs
        let logs = provider.get_logs(&self.state.eth_filter).await?;

        for log in logs.iter() {
            let event_id = log
                .transaction_hash
                .expect("failed to find tx hash in eth logs")
                .to_vec();
            if self.state.eth_processed_events.insert(event_id) {
                info!("[CCTP MOCK RELAY] picked up CCTP transfer event on Ethereum");

                let alloy_log =
                    Log::new(log.address(), log.topics().into(), log.data().clone().data)
                        .unwrap_or_default();

                let deposit_for_burn_log = DepositForBurn::decode_log(&alloy_log, false)?;
                self.mint_noble(deposit_for_burn_log).await?;
            }
        }

        Ok(())
    }

    async fn process_noble_block(
        &self,
        rpc_addr: &str,
        block_number: u32,
        destination_erc20: Address,
    ) -> Result<(), Box<dyn Error>> {
        let results = self
            .noble_client
            .block_results(rpc_addr, block_number)
            .await
            .expect("failed to fetch noble block results");

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
                            let key = attribute.key_str()?.to_string();
                            let value = attribute.value_str()?.to_string();
                            if key == "amount" {
                                amount = value
                                    .strip_prefix('"')
                                    .unwrap()
                                    .strip_suffix('"')
                                    .unwrap()
                                    .to_string();
                                info!("\t[CCTP NOBLE] amount: {amount}");
                            } else if key == "mint_recipient" {
                                mint_recipient = value
                                    .strip_suffix('"')
                                    .unwrap()
                                    .strip_prefix('"')
                                    .unwrap()
                                    .to_string();
                                info!("\t[CCTP NOBLE] mint_recipient: {mint_recipient}");
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
                            self.mint_evm(
                                amount,
                                mint_recipient,
                                destination_domain,
                                destination_erc20,
                            )
                            .await?
                        }
                    }
                }
            }
        }

        Ok(())
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
