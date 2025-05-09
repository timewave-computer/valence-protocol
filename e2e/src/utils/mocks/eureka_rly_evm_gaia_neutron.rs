use std::{collections::HashSet, error::Error, str::FromStr, time::Duration};

use crate::utils::{
    parse::{get_chain_field_from_local_ic_log, get_grpc_address_and_port_from_url},
    solidity_contracts::IBCEurekaTransfer::EurekaTransfer,
    worker::ValenceWorker,
    ADMIN_MNEMONIC, DEFAULT_ANVIL_RPC_ENDPOINT,
};
use alloy::{
    hex::ToHexExt,
    primitives::{Address, Log},
    providers::Provider,
    rpc::types::Filter,
    signers::local::{coins_bip39::English, MnemonicBuilder},
    sol_types::SolEvent,
};
use async_trait::async_trait;
use bech32::{encode, Bech32};
use cosmwasm_std::Uint128;
use hex::FromHex;
use localic_utils::{
    GAIA_CHAIN_ADMIN_ADDR, GAIA_CHAIN_DENOM, GAIA_CHAIN_ID, NEUTRON_CHAIN_ADMIN_ADDR,
    NEUTRON_CHAIN_ID,
};
use log::{info, warn};
use valence_chain_client_utils::{
    cosmos::base_client::BaseClient, ethereum::EthereumClient,
    evm::request_provider_client::RequestProviderClient, gaia::CosmosHubClient,
    neutron::NeutronClient,
};

const POLLING_PERIOD: Duration = Duration::from_secs(5);

pub struct MockEurekaRelayerEvmNeutron {
    pub state: RelayerState,
    pub runtime: RelayerRuntime,
}

pub struct RelayerRuntime {
    pub eth_client: EthereumClient,
    pub gaia_client: CosmosHubClient,
    pub neutron_client: NeutronClient,
}

pub struct RelayerState {
    // target receiver address on the hub, owned by the relayer
    hub_receiver_addr: String,
    // subdenom to perform the tokenfactory mint
    destination_chain_subdenom: String,
    destination_chain_denom_on_hub: String,

    // processed events cache to avoid double processing
    eth_processed_events: HashSet<Vec<u8>>,
    // ethereum filter to poll for events
    eth_filter: Filter,
    // ethereum destination erc20 address
    eth_destination_erc20: Address,
    // eureka transfer valence lib address
    eth_eureka_transfer_addr: Address,
}

#[async_trait]
impl ValenceWorker for MockEurekaRelayerEvmNeutron {
    fn get_name(&self) -> String {
        "Mock Eureka Relayer: ETH-NEUTRON".to_string()
    }

    /// each eureka relayer cycle will poll both gaia and ethereum for events
    /// that indicate an IBC Eureka transfer. Once such event is picked up on the origin
    /// domain, it will mint the equivalent amount on the destination chain.
    async fn cycle(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let worker_name = self.get_name();

        if let Err(e) = self.poll_gaia().await {
            warn!("{worker_name}: Gaia polling error: {:?}", e);
        }

        if let Err(e) = self.poll_ethereum().await {
            warn!("{worker_name}: Ethereum polling error: {:?}", e);
        }

        tokio::time::sleep(POLLING_PERIOD).await;

        Ok(())
    }
}

impl RelayerRuntime {
    async fn default() -> Result<Self, Box<dyn Error>> {
        let grpc_addr = get_chain_field_from_local_ic_log(GAIA_CHAIN_ID, "grpc_address")?;
        let (grpc_url, grpc_port) = get_grpc_address_and_port_from_url(&grpc_addr)?;

        let hub_client = CosmosHubClient::new(
            &grpc_url,
            &grpc_port.to_string(),
            ADMIN_MNEMONIC,
            GAIA_CHAIN_ID,
            GAIA_CHAIN_DENOM,
        )
        .await
        .expect("failed to create cosmoshub client");

        let grpc_addr = get_chain_field_from_local_ic_log(NEUTRON_CHAIN_ID, "grpc_address")?;
        let (grpc_url, grpc_port) = get_grpc_address_and_port_from_url(&grpc_addr)?;

        let neutron_client = NeutronClient::new(
            &grpc_url,
            &grpc_port.to_string(),
            ADMIN_MNEMONIC,
            NEUTRON_CHAIN_ID,
        )
        .await
        .expect("failed to create neutron client");

        let signer = MnemonicBuilder::<English>::default()
            .phrase("test test test test test test test test test test test junk")
            .index(5)? // derive the mnemonic at a different index to avoid nonce issues
            .build()?;

        let eth_client = EthereumClient {
            rpc_url: DEFAULT_ANVIL_RPC_ENDPOINT.to_string(),
            signer,
        };

        Ok(Self {
            eth_client,
            gaia_client: hub_client,
            neutron_client,
        })
    }
}

impl MockEurekaRelayerEvmNeutron {
    pub async fn new(
        eureka_transfer_lib: Address,
        token_erc20: &Address,
        dest_chain_subdenom: String,
        dest_chain_denom_on_hub: String,
    ) -> Result<Self, Box<dyn Error>> {
        let runtime = RelayerRuntime::default().await?;

        Ok(Self {
            state: RelayerState {
                hub_receiver_addr: GAIA_CHAIN_ADMIN_ADDR.to_string(),
                eth_processed_events: HashSet::new(),
                eth_filter: Filter::new().address(eureka_transfer_lib),
                eth_destination_erc20: *token_erc20,
                eth_eureka_transfer_addr: eureka_transfer_lib,
                destination_chain_subdenom: dest_chain_subdenom,
                destination_chain_denom_on_hub: dest_chain_denom_on_hub,
            },
            runtime,
        })
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
                match EurekaTransfer::decode_log(&alloy_log, false) {
                    Ok(eureka_transfer_event) => {
                        info!(
                            "[MOCK EUREKA RLY] decoded eureka transfer event: {:?}",
                            eureka_transfer_event
                        );

                        self.mint_neutron_side(eureka_transfer_event).await?;
                    }
                    Err(e) => {
                        warn!(
                            "[MOCK EUREKA RLY] failed to decode eureka transfer log: {:?}",
                            e
                        )
                    }
                };
            }
        }

        Ok(())
    }

    // gaia mock works by querying the specified account balance, moving those
    // tokens to another address, and minting the specified denom on evm
    async fn poll_gaia(&mut self) -> Result<(), Box<dyn Error>> {
        let balance = self
            .runtime
            .gaia_client
            .query_balance(
                &self.state.hub_receiver_addr,
                &self.state.destination_chain_denom_on_hub,
            )
            .await?;

        if balance > 0 {
            info!(
                "[MOCK EUREKA RLY] gaia polling address {} balance: {balance}",
                self.state.destination_chain_denom_on_hub
            );

            // 1. transfer the funds out from the account into another one to avoid
            // double counting
            let burner_addr = "cosmos1p0var04vhr03r2j8zwv4jfrz73rxgjt5v29x49".to_string();
            match self
                .runtime
                .gaia_client
                .transfer(
                    &burner_addr,
                    balance,
                    &self.state.destination_chain_denom_on_hub,
                    None,
                )
                .await
            {
                Ok(_) => info!("[MOCK EUREKA RLY] burned gaia addr tokens"),
                Err(_) => warn!("[MOCK EUREKA RLY] failed to burn gaia addr tokens"),
            };

            // 2. do the mint
            self.mint_evm_side(balance).await?;
        }

        Ok(())
    }

    /// on successful finding of `EurekaTransfer` event, we mint the funds straight
    /// into the destination address decoded from the log. This bypasses gaia entirely.
    async fn mint_neutron_side(&self, val: Log<EurekaTransfer>) -> Result<(), Box<dyn Error>> {
        let mint_amount = val.amount.to_string();

        let tf_mint_rx = self
            .runtime
            .neutron_client
            .mint_tokenfactory_tokens(
                &self.state.destination_chain_subdenom,
                Uint128::from_str(&mint_amount)?.u128(),
                Some(&val.recipient),
            )
            .await?;
        self.runtime
            .neutron_client
            .poll_for_tx(&tf_mint_rx.hash)
            .await?;

        info!(
            "[MOCK EUREKA RLY] minted {mint_amount}{} to {}",
            self.state.destination_chain_subdenom, val.recipient
        );
        Ok(())
    }

    async fn mint_evm_side(&self, amount: u128) -> Result<(), Box<dyn Error>> {
        let eth_rp = self
            .runtime
            .eth_client
            .get_request_provider()
            .await
            .expect("failed to get eth request provider");

        Ok(())
    }
}

fn decode_mint_recipient_to_address(mint_recipient_hex: &str) -> Result<String, Box<dyn Error>> {
    let (hrp, _) = bech32::decode(NEUTRON_CHAIN_ADMIN_ADDR)?;

    let stripped_hex = mint_recipient_hex
        .strip_prefix("0x")
        .unwrap_or(mint_recipient_hex);

    let bytes = Vec::from_hex(stripped_hex)?;

    let neutron_address = encode::<Bech32>(hrp, &bytes)?;

    Ok(neutron_address)
}
