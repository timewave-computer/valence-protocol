use std::{collections::HashSet, error::Error, str::FromStr, sync::Arc, time::Duration};

use alloy::{
    hex::ToHexExt,
    primitives::{Address, Log, U256},
    providers::Provider,
    rpc::types::Filter,
    sol_types::SolEvent,
};

use crate::{
    async_run,
    utils::{
        parse::get_rpc_address_from_logs,
        solidity_contracts::{MockERC20, MockTokenMessenger::DepositForBurn},
        NOBLE_CHAIN_ADMIN_ADDR, NOBLE_CHAIN_ID, UUSDC_DENOM,
    },
};
use bech32::{encode, Bech32};
use cosmwasm_std::{from_base64, Uint128};
use hex::FromHex;
use log::{info, warn};
use tokio::{runtime::Runtime, sync::Mutex, task::JoinHandle};
use valence_chain_client_utils::{
    cosmos::base_client::BaseClient,
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
    noble::NobleClient,
};

use super::{
    parse::get_grpc_address_and_port_from_logs, ADMIN_MNEMONIC, DEFAULT_ANVIL_RPC_ENDPOINT,
    NOBLE_CHAIN_DENOM,
};

pub struct MockCctpRelayer {
    eth_client: EthereumClient,
    eth_acc: Address,
    noble_client: NobleClient,
    state: Arc<Mutex<RelayerState>>,
}

struct RelayerState {
    last_noble_block: i64,
    // processed events cache to avoid double processing
    processed_events: HashSet<Vec<u8>>,
}

impl MockCctpRelayer {
    pub fn new(rt: &Runtime, eth_acc: Address) -> Self {
        let (grpc_url, grpc_port) = get_grpc_address_and_port_from_logs(NOBLE_CHAIN_ID).unwrap();
        let (noble_client, latest_noble_block) = async_run!(rt, {
            let client = NobleClient::new(
                &grpc_url,
                &grpc_port.to_string(),
                ADMIN_MNEMONIC,
                NOBLE_CHAIN_ID,
                NOBLE_CHAIN_DENOM,
            )
            .await
            .unwrap();
            let latest_block = client.latest_block_header().await.unwrap().height;
            (client, latest_block)
        });

        Self {
            eth_client: valence_chain_client_utils::ethereum::EthereumClient::new(
                DEFAULT_ANVIL_RPC_ENDPOINT,
                "test test test test test test test test test test test junk",
            )
            .unwrap(),
            eth_acc,
            noble_client,
            state: Arc::new(Mutex::new(RelayerState {
                last_noble_block: latest_noble_block,
                processed_events: HashSet::new(),
            })),
        }
    }

    pub async fn start_relay(
        self,
        destination_erc20: Address,
        messenger: Address,
    ) -> JoinHandle<()> {
        info!("[CCTP MOCK RELAY] Starting...");
        let filter = Filter::new().address(messenger);

        let rpc_addr = get_rpc_address_from_logs(NOBLE_CHAIN_ID).unwrap();
        let poll_interval = Duration::from_secs(5);

        // spawn a thread that relays noble <-> ethereum
        tokio::spawn(async move {
            loop {
                // process both chains in sequence rather than in parallel threads
                let _ = self.poll_noble(&rpc_addr, destination_erc20).await.unwrap();
                let _ = self.poll_ethereum(&filter).await.unwrap();
                // sleep for a bit
                tokio::time::sleep(poll_interval).await;
            }
        })
    }

    async fn mint_evm(
        &self,
        amount: String,
        mint_recipient: String,
        destination_domain: String,
        destination_erc20: Address,
    ) {
        info!("[CCTP NOBLE] minting {amount}USDC to domain #{destination_domain} recipient {mint_recipient}");
        let eth_rp = self.eth_client.get_request_provider().await.unwrap();

        let mock_erc20 = MockERC20::new(destination_erc20, &eth_rp);

        let amount_stripped = amount.strip_prefix('"').unwrap().strip_suffix('"').unwrap();

        let recipient_stripped = mint_recipient
            .strip_suffix('"')
            .unwrap()
            .strip_prefix('"')
            .unwrap();

        let amt = Uint128::from_str(&amount_stripped).unwrap();
        let to = from_base64(recipient_stripped).unwrap();

        let address_bytes = &to[12..];

        let dest_addr = Address::from_slice(address_bytes);

        let pre_mint_balance = self
            .eth_client
            .query(mock_erc20.balanceOf(dest_addr))
            .await
            .unwrap();

        let mint_tx = self
            .eth_client
            .execute_tx(
                mock_erc20
                    .mint(dest_addr, U256::from(amt.u128()))
                    .into_transaction_request()
                    .from(self.eth_acc),
            )
            .await
            .unwrap();

        let _receipt = eth_rp
            .get_transaction_receipt(mint_tx.transaction_hash)
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

    async fn mint_noble(&self, val: Log<DepositForBurn>) {
        info!("decoded deposit for burn log: {:?}", val);
        let destination_addr =
            decode_mint_recipient_to_noble_address(&val.mintRecipient.encode_hex()).unwrap();

        let tx_response = self
            .noble_client
            .mint_fiat(
                NOBLE_CHAIN_ADMIN_ADDR,
                &destination_addr,
                &val.amount.to_string(),
                UUSDC_DENOM,
            )
            .await
            .unwrap();
        self.noble_client
            .poll_for_tx(&tx_response.hash)
            .await
            .unwrap();
        info!(
            "[CCTP ETH] Minted {UUSDC_DENOM} to {destination_addr}: {:?}",
            tx_response
        );
    }

    pub async fn poll_noble(
        &self,
        rpc: &str,
        eth_destination: Address,
    ) -> Result<(), Box<dyn Error>> {
        // get last processed block from state
        let mut state = self.state.lock().await;

        // query the current block to process the delta
        let current_block = self
            .noble_client
            .latest_block_header()
            .await
            .unwrap()
            .height;

        // process all blocks from last processed block to current block
        for i in state.last_noble_block..current_block {
            self.process_noble_block(rpc, i as u32, eth_destination)
                .await
                .unwrap();
        }

        // update the last processed block and return
        state.last_noble_block = current_block;

        Ok(())
    }

    pub async fn poll_ethereum(&self, filter: &Filter) -> Result<(), Box<dyn Error>> {
        let mut state = self.state.lock().await;

        let provider = self
            .eth_client
            .get_request_provider()
            .await
            .expect("could not get provider");

        // fetch the logs
        let logs = provider.get_logs(&filter).await.unwrap();

        for log in logs.iter() {
            let event_id = log.transaction_hash.unwrap().to_vec();
            if state.processed_events.insert(event_id) {
                warn!("[CCTP MOCK RELAY] picked up CCTP transfer event on Ethereum");

                let alloy_log = alloy::primitives::Log::new(
                    log.address(),
                    log.topics().into(),
                    log.data().clone().data,
                )
                .unwrap_or_default();

                let deposit_for_burn_log = DepositForBurn::decode_log(&alloy_log, false).unwrap();
                self.mint_noble(deposit_for_burn_log).await;
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
                            .await
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
