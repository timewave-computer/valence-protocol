use std::{error::Error, time::Duration};

use alloy::{
    hex::ToHexExt,
    primitives::{Address, FixedBytes, IntoLogData},
    providers::Provider,
    rpc::types::Filter,
    sol_types::SolEvent,
};

use bech32::{encode, Bech32};
use hex::FromHex;
use log::{debug, info, warn};
use tokio::task::JoinHandle;
use valence_chain_client_utils::{
    cosmos::base_client::BaseClient, ethereum::EthereumClient,
    evm::request_provider_client::RequestProviderClient, noble::NobleClient,
};
use valence_e2e::utils::{
    solidity_contracts::MockTokenMessenger::DepositForBurn, NOBLE_CHAIN_ADMIN_ADDR, UUSDC_DENOM,
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

    pub async fn start_eth(self, messenger: Address) -> JoinHandle<()> {
        info!("[MOCK CCTP RELAYER] Getting request provider...");
        let provider = self
            .eth_client
            .get_request_provider()
            .await
            .expect("could not get provider");

        let filter = Filter::new().address(messenger);
        info!("[MOCK CCTP RELAYER] Created filter: {:?}", filter);

        tokio::spawn(async move {
            loop {
                info!("[MOCK RELAYER] polling logs...");
                match provider.get_logs(&filter).await {
                    Ok(logs) => {
                        if logs.is_empty() {
                            info!("[MOCK RELAYER] no logs found");
                        } else {
                            info!("[MOCK RELAYER] Found {} logs", logs.len());
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
                                        info!("[MOCK RELAYER] Minted {UUSDC_DENOM} to {destination_addr}: {:?}", tx_response);
                                    }
                                    Err(e) => {
                                        warn!("failed to decode the deposit for burn log: {:?}", e)
                                    }
                                }
                            }
                        }
                    }
                    Err(err) => {
                        warn!("[MOCK RELAYER] Error polling logs: {:?}", err);
                    }
                }

                info!("[MOCK RELAYER] Sleeping for 1 second before next poll");
                tokio::time::sleep(Duration::from_secs(1)).await;
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
