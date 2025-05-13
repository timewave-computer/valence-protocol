use std::{collections::HashSet, error::Error, time::Duration};

use alloy::{
    eips::BlockNumberOrTag,
    primitives::{Address, Log, U256},
    providers::Provider,
    rpc::types::Filter,
    sol,
    sol_types::SolEvent,
};
use async_trait::async_trait;

use crate::utils::{solidity_contracts::ERC20, worker::ValenceWorker};
use log::{info, warn};
use valence_domain_clients::{
    clients::ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};

const POLLING_PERIOD: Duration = Duration::from_secs(5);

sol! {
    event ERC20DepositInitiated(
        address indexed l1Token,
        address indexed l2Token,
        address indexed from,
        address to,
        uint256 amount,
        bytes extraData
    );

    event WithdrawalInitiated(
        address indexed l1Token,
        address indexed l2Token,
        address indexed from,
        address to,
        uint256 amount,
        bytes extraData
    );
}

pub struct MockStandardBridgeRelayer {
    pub state: RelayerState,
    pub runtime: RelayerRuntime,
}

pub struct RelayerRuntime {
    pub evm_client_a: EthereumClient,
    pub evm_client_b: EthereumClient,
}

impl RelayerRuntime {
    async fn new(endpoint_a: String, endpoint_b: String) -> Result<Self, Box<dyn Error>> {
        let evm_client_a = EthereumClient::new(
            &endpoint_a,
            "test test test test test test test test test test test junk",
            Some(6),
        )?;

        let evm_client_b = EthereumClient::new(
            &endpoint_b,
            "test test test test test test test test test test test junk",
            Some(6),
        )?;

        Ok(Self {
            evm_client_a,
            evm_client_b,
        })
    }
}

pub struct RelayerState {
    evm_a_last_block_processed: Option<u64>,
    evm_a_processed_events: HashSet<Vec<u8>>,
    evm_a_filter: Filter,
    evm_a_destination_erc20: Address,
    evm_b_last_block_processed: Option<u64>,
    evm_b_processed_events: HashSet<Vec<u8>>,
    evm_b_filter: Filter,
    evm_b_destination_erc20: Address,
}

#[async_trait]
impl ValenceWorker for MockStandardBridgeRelayer {
    fn get_name(&self) -> String {
        "Mock Standard Bridge Relayer".to_string()
    }

    async fn cycle(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let worker_name = self.get_name();

        if let Err(e) = self.poll_evm_a().await {
            warn!("{worker_name}: Evm A polling error: {:?}", e);
        }

        if let Err(e) = self.poll_evm_b().await {
            warn!("{worker_name}: Evm B polling error: {:?}", e);
        }

        tokio::time::sleep(POLLING_PERIOD).await;

        Ok(())
    }
}

impl MockStandardBridgeRelayer {
    pub async fn new(
        endpoint_a: String,
        endpoint_b: String,
        standard_bridge_a: Address,
        destination_erc20_a: Address,
        standard_bridge_b: Address,
        destination_erc20_b: Address,
    ) -> Result<Self, Box<dyn Error>> {
        let runtime = RelayerRuntime::new(endpoint_a, endpoint_b).await?;

        Ok(Self {
            runtime,
            state: RelayerState {
                evm_a_last_block_processed: None,
                evm_a_processed_events: HashSet::new(),
                evm_a_filter: Filter::new().address(standard_bridge_a),
                evm_a_destination_erc20: destination_erc20_a,
                evm_b_last_block_processed: None,
                evm_b_processed_events: HashSet::new(),
                evm_b_filter: Filter::new().address(standard_bridge_b),
                evm_b_destination_erc20: destination_erc20_b,
            },
        })
    }

    async fn send_on_evm_b(&self, amount: U256, recipient: Address) -> Result<(), Box<dyn Error>> {
        let evm_b_rp = self
            .runtime
            .evm_client_b
            .get_request_provider()
            .await
            .expect("failed to get evm B request provider");

        let erc20 = ERC20::new(self.state.evm_b_destination_erc20, &evm_b_rp);

        let pre_send_balance = self
            .runtime
            .evm_client_b
            .query(erc20.balanceOf(recipient))
            .await
            .expect("failed to query evm B balance");

        let send_tx = self
            .runtime
            .evm_client_b
            .execute_tx(erc20.transfer(recipient, amount).into_transaction_request())
            .await
            .expect("failed to send tokens on evm B");

        let _ = evm_b_rp
            .get_transaction_receipt(send_tx.transaction_hash)
            .await;

        let post_send_balance = self
            .runtime
            .evm_client_b
            .query(erc20.balanceOf(recipient))
            .await
            .expect("failed to query evm B balance");

        let delta = post_send_balance._0 - pre_send_balance._0;
        info!("[Standard Bridge] successfully sent {delta} tokens to evm B address {recipient}");

        Ok(())
    }

    async fn send_on_evm_a(&self, amount: U256, recipient: Address) -> Result<(), Box<dyn Error>> {
        let evm_a_rp = self
            .runtime
            .evm_client_a
            .get_request_provider()
            .await
            .expect("failed to get evm A request provider");

        let erc20 = ERC20::new(self.state.evm_a_destination_erc20, &evm_a_rp);

        let pre_send_balance = self
            .runtime
            .evm_client_a
            .query(erc20.balanceOf(recipient))
            .await
            .expect("failed to query evm A balance");

        let send_tx = self
            .runtime
            .evm_client_a
            .execute_tx(erc20.transfer(recipient, amount).into_transaction_request())
            .await
            .expect("failed to send tokens on evm A");

        let _ = evm_a_rp
            .get_transaction_receipt(send_tx.transaction_hash)
            .await;

        let post_send_balance = self
            .runtime
            .evm_client_a
            .query(erc20.balanceOf(recipient))
            .await
            .expect("failed to query evm A balance");

        let delta = post_send_balance._0 - pre_send_balance._0;
        info!("[Standard Bridge] successfully sent {delta} tokens to evm A address {recipient}");

        Ok(())
    }

    async fn poll_evm_a(&mut self) -> Result<(), Box<dyn Error>> {
        let provider = self
            .runtime
            .evm_client_a
            .get_request_provider()
            .await
            .expect("could not get evm A provider");

        let current_block = provider.get_block_number().await?;
        let last_block = self
            .state
            .evm_a_last_block_processed
            .unwrap_or(current_block);
        let filter = self.state.evm_a_filter.clone();
        let filter = filter
            .from_block(BlockNumberOrTag::Number(last_block))
            .to_block(BlockNumberOrTag::Number(current_block));

        // fetch the logs
        let logs = provider.get_logs(&filter).await?;

        for log in logs.iter() {
            let event_id = log
                .transaction_hash
                .expect("failed to find tx hash in evm A logs")
                .to_vec();
            if self.state.evm_a_processed_events.insert(event_id) {
                let alloy_log =
                    Log::new(log.address(), log.topics().into(), log.data().clone().data)
                        .unwrap_or_default();

                // Depending if it's L1 or L2, the event will be different
                let (amount, to) = {
                    match ERC20DepositInitiated::decode_log(&alloy_log, false) {
                        Ok(erc20_deposit_initiated_log) => (
                            erc20_deposit_initiated_log.amount,
                            erc20_deposit_initiated_log.to,
                        ),
                        Err(_) => {
                            // If it's not an ERC20DepositInitiated event, it might be a WithdrawalInitiated event
                            let withdrawal_initiated_log =
                                WithdrawalInitiated::decode_log(&alloy_log, false)?;
                            (withdrawal_initiated_log.amount, withdrawal_initiated_log.to)
                        }
                    }
                };

                // send on EVM B when an event is detected on EVM A
                self.send_on_evm_b(amount, to).await?;
            }
        }

        // update the last block processed
        self.state.evm_a_last_block_processed = Some(current_block);

        Ok(())
    }

    async fn poll_evm_b(&mut self) -> Result<(), Box<dyn Error>> {
        let provider = self
            .runtime
            .evm_client_b
            .get_request_provider()
            .await
            .expect("could not get evm B provider");

        let current_block = provider.get_block_number().await?;
        let last_block = self
            .state
            .evm_b_last_block_processed
            .unwrap_or(current_block);
        let filter = self.state.evm_b_filter.clone();
        let filter = filter
            .from_block(BlockNumberOrTag::Number(last_block))
            .to_block(BlockNumberOrTag::Number(current_block));
        // fetch the logs
        let logs = provider.get_logs(&filter).await?;

        for log in logs.iter() {
            let event_id = log
                .transaction_hash
                .expect("failed to find tx hash in evm B logs")
                .to_vec();
            if self.state.evm_b_processed_events.insert(event_id) {
                let alloy_log =
                    Log::new(log.address(), log.topics().into(), log.data().clone().data)
                        .unwrap_or_default();

                // Depending if it's L1 or L2, the event will be different
                let (amount, to) = {
                    match ERC20DepositInitiated::decode_log(&alloy_log, false) {
                        Ok(erc20_deposit_initiated_log) => (
                            erc20_deposit_initiated_log.amount,
                            erc20_deposit_initiated_log.to,
                        ),
                        Err(_) => {
                            // If it's not an ERC20DepositInitiated event, it might be a WithdrawalInitiated event
                            let withdrawal_initiated_log =
                                WithdrawalInitiated::decode_log(&alloy_log, false)?;
                            (withdrawal_initiated_log.amount, withdrawal_initiated_log.to)
                        }
                    }
                };

                // send on EVM A when an event is detected on EVM B
                self.send_on_evm_a(amount, to).await?;
            }
        }
        // update the last block processed
        self.state.evm_b_last_block_processed = Some(current_block);

        Ok(())
    }
}
