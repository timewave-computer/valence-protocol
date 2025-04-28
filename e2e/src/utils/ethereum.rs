use std::{collections::HashMap, error::Error};

use alloy::primitives::U256;
use bollard::{
    container::{Config, CreateContainerOptions, ListContainersOptions, StartContainerOptions},
    image::CreateImageOptions,
    Docker,
};
use futures_util::StreamExt;
use log::{error, info};
use valence_chain_client_utils::{
    ethereum::EthereumClient, evm::request_provider_client::RequestProviderClient,
};

const ANVIL_IMAGE_URL: &str = "ghcr.io/foundry-rs/foundry:latest";
pub const ANVIL_NAME: &str = "anvil";
pub const DEFAULT_ANVIL_PORT: &str = "8545";

/// macro for executing async code in a blocking context
#[macro_export]
macro_rules! async_run {
    ($rt:expr, $($body:tt)*) => {
        $rt.block_on(async { $($body)* })
    }
}

pub async fn set_up_anvil_container(
    container_name: &str,
    port: &str,
    fork_url: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    // Connect to the Docker daemon
    let docker = Docker::connect_with_local_defaults()?;

    let mut filters = HashMap::new();
    filters.insert("name", vec![container_name]);
    let options = ListContainersOptions {
        all: true,
        filters,
        ..Default::default()
    };

    // Check if container already exists
    let containers = docker.list_containers(Some(options)).await?;
    if !containers.is_empty() {
        for container in containers {
            info!("found an existing Anvil container: {:?}", container);
            info!("attempting to kill the container...");

            let container_id = &container.id.unwrap();

            match docker.kill_container::<String>(container_id, None).await {
                Ok(_) => info!("killed existing container: {container_id}"),
                Err(e) => error!("failed to kill container {container_id}: {e}"),
            }

            docker
                .remove_container(container_id, None)
                .await
                .map_err(|e| error!("Failed to remove container {container_id}: {e}"))
                .unwrap();

            info!("removed old Anvil container");
        }
    }

    // Pull image if it doesn't exist
    let mut pull_stream = docker.create_image(
        Some(CreateImageOptions {
            from_image: ANVIL_IMAGE_URL,
            ..Default::default()
        }),
        None,
        None,
    );

    // Pull the image and process the progress stream
    info!("Pulling image: {ANVIL_IMAGE_URL}");

    while let Some(result) = pull_stream.next().await {
        match result {
            Ok(output) => {
                if let Some(status) = output.status {
                    info!("Status: {status}");
                }
                if let Some(progress) = output.progress {
                    info!("Progress: {progress}");
                }
            }
            Err(e) => error!("Error pulling image: {}", e),
        }
    }

    let config = Config {
        image: Some(ANVIL_IMAGE_URL),
        cmd: Some(if let Some(url) = fork_url {
            vec!["--fork-url", url]
        } else {
            vec![]
        }),
        env: Some(vec!["ANVIL_IP_ADDR=0.0.0.0"]),
        entrypoint: Some(vec![ANVIL_NAME]),
        exposed_ports: {
            // Anvil always runs on port 8545 inside the container
            let ports = HashMap::from_iter([("8545/tcp", HashMap::new())]);
            Some(ports)
        },
        host_config: Some(bollard::service::HostConfig {
            port_bindings: {
                let mut port_bindings = HashMap::new();
                port_bindings.insert(
                    "8545/tcp".to_string(),
                    Some(vec![bollard::service::PortBinding {
                        host_ip: Some("0.0.0.0".to_string()),
                        host_port: Some(port.to_string()),
                    }]),
                );
                Some(port_bindings)
            },
            ..Default::default()
        }),
        ..Default::default()
    };

    // Create container
    let options = Some(CreateContainerOptions {
        name: container_name,
        platform: None,
    });
    let container = docker.create_container(options, config).await?;

    // Start container
    docker
        .start_container(&container.id, None::<StartContainerOptions<String>>)
        .await?;

    info!("Anvil container started successfully");

    info!("Waiting for Anvil JSON-RPC to be ready...");
    wait_for_anvil_ready(port, 30).await?;

    info!("Anvil container ready!");
    Ok(())
}

async fn wait_for_anvil_ready(port: &str, timeout_secs: u64) -> Result<(), Box<dyn Error>> {
    use std::time::{Duration, Instant};
    use tokio::time::sleep;

    let client = alloy::transports::http::reqwest::Client::new();
    let start = Instant::now();
    let url = format!("http://localhost:{}", port);

    while start.elapsed() < Duration::from_secs(timeout_secs) {
        let poll_rx = client
            .post(&url)
            .timeout(Duration::from_secs(1))
            .body("{}")
            .send()
            .await;

        if let Ok(resp) = poll_rx {
            if resp.status().is_success() {
                return Ok(());
            }
        }

        sleep(Duration::from_millis(500)).await;
    }

    Err(format!("timed out waiting for Anvil to be ready on port {}", port).into())
}

pub mod valence_account {
    use std::error::Error;

    use alloy::primitives::Address;
    use log::info;
    use valence_chain_client_utils::{
        ethereum::EthereumClient,
        evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
    };

    use crate::utils::solidity_contracts::BaseAccount;

    pub fn setup_valence_account(
        rt: &tokio::runtime::Runtime,
        eth_client: &EthereumClient,
        admin: Address,
    ) -> Result<Address, Box<dyn Error>> {
        info!("Deploying base account on Ethereum...");
        async_run!(rt, {
            let eth_rp = eth_client.get_request_provider().await.unwrap();
            let base_account_tx =
                BaseAccount::deploy_builder(&eth_rp, admin, vec![]).into_transaction_request();

            let base_account_rx = eth_client
                .execute_tx(base_account_tx.clone())
                .await
                .unwrap();

            let base_account_addr = base_account_rx.contract_address.unwrap();

            Ok(base_account_addr)
        })
    }

    pub fn approve_library(
        rt: &tokio::runtime::Runtime,
        eth_client: &EthereumClient,
        account_addr: Address,
        library_addr: Address,
    ) {
        async_run!(rt, {
            let eth_rp = eth_client.get_request_provider().await.unwrap();
            let deposit_account = BaseAccount::new(account_addr, &eth_rp);

            eth_client
                .execute_tx(
                    deposit_account
                        .approveLibrary(library_addr)
                        .into_transaction_request(),
                )
                .await
                .unwrap();
        });
    }
}

pub mod mock_erc20 {
    use std::error::Error;

    use alloy::primitives::{Address, U256};
    use log::info;
    use valence_chain_client_utils::{
        ethereum::EthereumClient,
        evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
    };

    use crate::utils::solidity_contracts::MockERC20;

    pub fn setup_deposit_erc20(
        rt: &tokio::runtime::Runtime,
        eth_client: &EthereumClient,
        name: &str,
        denom: &str,
        precision: u8,
    ) -> Result<Address, Box<dyn Error>> {
        async_run!(rt, {
            info!("Deploying MockERC20 contract...");

            let eth_rp = eth_client.get_request_provider().await.unwrap();

            let evm_vault_deposit_token_tx =
                MockERC20::deploy_builder(&eth_rp, name.to_string(), denom.to_string(), precision)
                    .into_transaction_request();

            let evm_vault_deposit_token_rx =
                valence_chain_client_utils::evm::base_client::EvmBaseClient::execute_tx(
                    eth_client,
                    evm_vault_deposit_token_tx,
                )
                .await
                .unwrap();

            let valence_vault_deposit_token_address =
                evm_vault_deposit_token_rx.contract_address.unwrap();

            Ok(valence_vault_deposit_token_address)
        })
    }

    pub fn mint(
        rt: &tokio::runtime::Runtime,
        eth_client: &EthereumClient,
        erc20_addr: Address,
        to: Address,
        amount: U256,
    ) {
        async_run!(rt, {
            let eth_rp = eth_client.get_request_provider().await.unwrap();

            let mock_erc20 = MockERC20::new(erc20_addr, &eth_rp);

            eth_client
                .execute_tx(mock_erc20.mint(to, amount).into_transaction_request())
                .await
                .unwrap();
        });
    }

    pub fn approve(
        rt: &tokio::runtime::Runtime,
        eth_client: &EthereumClient,
        erc20_addr: Address,
        approver: Address,
        to_approve: Address,
        amount: U256,
    ) {
        async_run!(rt, {
            let eth_rp = eth_client.get_request_provider().await.unwrap();

            let mock_erc20 = MockERC20::new(erc20_addr, &eth_rp);

            let signed_tx = mock_erc20
                .approve(to_approve, amount)
                .into_transaction_request()
                .from(approver);
            let rx = alloy::providers::Provider::send_transaction(&eth_rp, signed_tx)
                .await
                .unwrap()
                .get_receipt()
                .await
                .unwrap();

            info!("erc20 approval rx: {:?}", rx.transaction_hash);
        });
    }

    pub fn transfer(
        rt: &tokio::runtime::Runtime,
        eth_client: &EthereumClient,
        erc20_addr: Address,
        from: Address,
        to: Address,
        amount: U256,
    ) {
        async_run!(rt, {
            let eth_rp = eth_client.get_request_provider().await.unwrap();

            let mock_erc20 = MockERC20::new(erc20_addr, &eth_rp);

            let signed_tx = mock_erc20
                .transfer(to, amount)
                .into_transaction_request()
                .from(from);

            let rx = alloy::providers::Provider::send_transaction(&eth_rp, signed_tx)
                .await
                .unwrap()
                .get_receipt()
                .await
                .unwrap();

            info!("erc20 transfer rx: {:?}", rx.transaction_hash);
        });
    }

    pub fn query_balance(
        rt: &tokio::runtime::Runtime,
        eth_client: &EthereumClient,
        erc20_addr: Address,
        addr: Address,
    ) -> U256 {
        async_run!(rt, {
            let eth_rp = eth_client.get_request_provider().await.unwrap();

            let mock_erc20 = MockERC20::new(erc20_addr, &eth_rp);

            let balance_q = mock_erc20.balanceOf(addr);

            eth_client.query(balance_q).await.unwrap()._0
        })
    }
}

pub mod lite_processor {
    use std::{error::Error, str::FromStr};

    use alloy::primitives::Address;
    use log::info;
    use valence_chain_client_utils::{
        ethereum::EthereumClient,
        evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
    };

    use crate::utils::{solidity_contracts::LiteProcessor, NEUTRON_HYPERLANE_DOMAIN};

    pub fn setup_lite_processor(
        rt: &tokio::runtime::Runtime,
        eth_client: &EthereumClient,
        admin: Address,
        mailbox: &str,
        authorization_contract_address: &str,
    ) -> Result<Address, Box<dyn Error>> {
        async_run!(rt, {
            let eth_rp = eth_client.get_request_provider().await.unwrap();

            let tx = LiteProcessor::deploy_builder(
                &eth_rp,
                crate::utils::hyperlane::bech32_to_evm_bytes32(authorization_contract_address)?,
                Address::from_str(mailbox)?,
                NEUTRON_HYPERLANE_DOMAIN,
                vec![admin],
            )
            .into_transaction_request();

            let lite_processor_rx = eth_client.execute_tx(tx).await.unwrap();

            let lite_processor_address = lite_processor_rx.contract_address.unwrap();
            info!("Lite Processor deployed at: {}", lite_processor_address);

            Ok(lite_processor_address)
        })
    }
}

pub fn mine_blocks(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    blocks: usize,
    interval: usize,
) {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();

        alloy::providers::ext::AnvilApi::anvil_mine(
            &eth_rp,
            Some(U256::from(blocks)),
            Some(U256::from(interval)),
        )
        .await
        .unwrap();
    });
}
