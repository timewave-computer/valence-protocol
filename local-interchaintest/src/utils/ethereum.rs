use std::{collections::HashMap, error::Error};

use bollard::{
    container::{Config, CreateContainerOptions, ListContainersOptions, StartContainerOptions},
    Docker,
};
use log::info;

pub async fn set_up_anvil_container() -> Result<(), Box<dyn Error>> {
    // Connect to the Docker daemon
    let docker = Docker::connect_with_local_defaults()?;

    let mut filters = HashMap::new();
    filters.insert("name", vec!["anvil"]);
    let options = ListContainersOptions {
        all: true,
        filters,
        ..Default::default()
    };

    // Check if container already exists
    let containers = docker.list_containers(Some(options)).await?;
    if !containers.is_empty() {
        info!("Anvil container already exists");
        return Ok(());
    }

    let config = Config {
        image: Some("ghcr.io/foundry-rs/foundry:latest"),
        cmd: Some(vec!["anvil"]),
        env: Some(vec!["ANVIL_IP_ADDR=0.0.0.0"]),
        exposed_ports: {
            let mut ports = HashMap::new();
            ports.insert("8545/tcp", HashMap::new());
            Some(ports)
        },
        host_config: Some(bollard::service::HostConfig {
            port_bindings: {
                let mut port_bindings = HashMap::new();
                port_bindings.insert(
                    "8545/tcp".to_string(),
                    Some(vec![bollard::service::PortBinding {
                        host_ip: Some("0.0.0.0".to_string()),
                        host_port: Some("8545".to_string()),
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
        name: "anvil",
        platform: None,
    });
    let container = docker.create_container(options, config).await?;

    // Start container
    docker
        .start_container(&container.id, None::<StartContainerOptions<String>>)
        .await?;

    info!("Anvil container started successfully");
    Ok(())
}
