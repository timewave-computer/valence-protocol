use std::{collections::HashSet, error::Error, str::FromStr, time::Duration};

use alloy::{
    hex::ToHexExt,
    primitives::{Address, Log, U256},
    providers::Provider,
    rpc::types::Filter,
    sol_types::SolEvent,
};
use async_trait::async_trait;

use crate::utils::{
    parse::{get_chain_field_from_local_ic_log, get_grpc_address_and_port_from_url},
    solidity_contracts::{MockERC20, MockTokenMessenger::DepositForBurn},
    worker::ValenceWorker,
    ADMIN_MNEMONIC, DEFAULT_ANVIL_RPC_ENDPOINT, NOBLE_CHAIN_ADMIN_ADDR, NOBLE_CHAIN_DENOM,
    NOBLE_CHAIN_ID, UUSDC_DENOM,
};
use bech32::{encode, Bech32};
use cosmwasm_std::{from_base64, Uint128};
use hex::FromHex;
use log::{info, warn};
use valence_domain_clients::{
    clients::ethereum::EthereumClient,
    clients::noble::NobleClient,
    cosmos::base_client::BaseClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};

const POLLING_PERIOD: Duration = Duration::from_secs(5);

pub struct MockCctpRelayerEvmNoble {
    pub state: RelayerState,
    pub runtime: RelayerRuntime,
}

pub struct RelayerRuntime {
    pub eth_client: EthereumClient,
    pub noble_client: NobleClient,
}

impl RelayerRuntime {
    async fn default() -> Result<Self, Box<dyn Error>> {
        let grpc_addr = get_chain_field_from_local_ic_log(NOBLE_CHAIN_ID, "grpc_address")?;
        let (grpc_url, grpc_port) = get_grpc_address_and_port_from_url(&grpc_addr)?;

        let noble_client = NobleClient::new(
            &grpc_url,
            &grpc_port.to_string(),
            ADMIN_MNEMONIC,
            NOBLE_CHAIN_ID,
            NOBLE_CHAIN_DENOM,
        )
        .await
        .expect("failed to create noble client");

        // TODO: used to derive signer at index 5
        let eth_client = EthereumClient::new(
            DEFAULT_ANVIL_RPC_ENDPOINT,
            "test test test test test test test test test test test junk",
        )?;

        Ok(Self {
            eth_client,
            noble_client,
        })
    }
}

pub struct RelayerState {
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

#[async_trait]
impl ValenceWorker for MockCctpRelayerEvmNoble {
    fn get_name(&self) -> String {
        "Mock CCTP Relayer: ETH-NOBLE".to_string()
    }

    /// each cctp relayer cycle will poll both noble and ethereum for events
    /// that indicate a CCTP-transfer. Once such event is picked up on the origin
    /// domain, it will mint the equivalent amount on the destination chain.
    async fn cycle(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let worker_name = self.get_name();

        if let Err(e) = self.poll_noble().await {
            warn!("{worker_name}: Noble polling error: {:?}", e);
        }

        if let Err(e) = self.poll_ethereum().await {
            warn!("{worker_name}: Ethereum polling error: {:?}", e);
        }

        tokio::time::sleep(POLLING_PERIOD).await;

        Ok(())
    }
}

impl MockCctpRelayerEvmNoble {
    pub async fn new(
        messenger: Address,
        destination_erc20: Address,
    ) -> Result<Self, Box<dyn Error>> {
        let runtime = RelayerRuntime::default().await?;

        let latest_noble_block = runtime
            .noble_client
            .latest_block_header()
            .await
            .expect("failed to get latest block header")
            .height;

        let noble_rpc_addr = get_chain_field_from_local_ic_log(NOBLE_CHAIN_ID, "rpc_address")
            .expect("Failed to find rpc_address field for noble chain");

        Ok(Self {
            runtime,
            state: RelayerState {
                noble_last_block: latest_noble_block,
                noble_rpc_addr,
                eth_processed_events: HashSet::new(),
                eth_filter: Filter::new().address(messenger),
                eth_destination_erc20: destination_erc20,
            },
        })
    }

    async fn mint_evm(
        &self,
        amount: String,
        mint_recipient: String,
        _destination_domain: String,
        destination_erc20: Address,
    ) -> Result<(), Box<dyn Error>> {
        // info!("[CCTP NOBLE] minting {amount}USDC to domain #{destination_domain} recipient {mint_recipient}");
        let eth_rp = self
            .runtime
            .eth_client
            .get_request_provider()
            .await
            .expect("failed to get eth request provider");

        let mock_erc20 = MockERC20::new(destination_erc20, &eth_rp);

        let amt = Uint128::from_str(&amount)?;
        let to = from_base64(mint_recipient)?;

        let dest_addr = Address::from_slice(&to[12..]);

        let pre_mint_balance = self
            .runtime
            .eth_client
            .query(mock_erc20.balanceOf(dest_addr))
            .await
            .expect("failed to query eth balance");

        let mint_tx = self
            .runtime
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
            .runtime
            .eth_client
            .query(mock_erc20.balanceOf(dest_addr))
            .await
            .expect("failed to query eth balance");

        let delta = post_mint_balance._0 - pre_mint_balance._0;
        info!("[CCTP NOBLE] successfully minted {delta} tokens to eth address {dest_addr}");

        Ok(())
    }

    async fn mint_noble(&self, val: Log<DepositForBurn>) -> Result<(), Box<dyn Error>> {
        let destination_addr =
            decode_mint_recipient_to_noble_address(&val.mintRecipient.encode_hex())?;

        let mint_amount = val.amount.to_string();
        let tx_response = self
            .runtime
            .noble_client
            .mint_fiat(
                NOBLE_CHAIN_ADMIN_ADDR,
                &destination_addr,
                &mint_amount,
                UUSDC_DENOM,
            )
            .await
            .expect("failed to mint usdc on noble");
        self.runtime
            .noble_client
            .poll_for_tx(&tx_response.hash)
            .await
            .expect("failed to poll for mint tx on noble");
        info!("[CCTP ETH] Minted {mint_amount}{UUSDC_DENOM} to {destination_addr}");

        Ok(())
    }

    async fn poll_noble(&mut self) -> Result<(), Box<dyn Error>> {
        // query the current block to process the delta
        let current_block = self
            .runtime
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
            .runtime
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
            .runtime
            .noble_client
            .block_results(rpc_addr, block_number)
            .await
            .expect("failed to fetch noble block results");

        if let Some(r) = results.txs_results {
            for result in r {
                for event in result.events {
                    if event.kind == "circle.cctp.v1.DepositForBurn" {
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
                            } else if key == "mint_recipient" {
                                mint_recipient = value
                                    .strip_suffix('"')
                                    .unwrap()
                                    .strip_prefix('"')
                                    .unwrap()
                                    .to_string();
                            } else if key == "destination_domain" {
                                destination_domain = value;
                            } else if key == "destination_token_messenger" {
                                destination_token_messenger = value;
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

    let stripped_hex = mint_recipient_hex
        .strip_prefix("0x")
        .unwrap_or(mint_recipient_hex);

    let bytes = Vec::from_hex(stripped_hex)?;

    let noble_address = encode::<Bech32>(hrp, &bytes)?;

    Ok(noble_address)
}
