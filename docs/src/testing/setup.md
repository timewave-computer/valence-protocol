# Initial Testing Set Up

For testing your programs, no matter if you want to use the manager or not, there is a common set up that needs to be done. This set up is necessary to initialize the testing context with all the required information of the local-interchain environment.

## 1. Setting the TestContext using the TestContextBuilder

The `TestContext` is the interchain environment in which your program will run. Let's say you want to configure the Neutron chain and Osmosis chain, you may set it up as follows:

```rust
    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_chain(ConfigChainBuilder::default_osmosis().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, OSMOSIS_CHAIN_NAME)
        .build()?;
```

This will instantiate a `TestContext` with two chains, Neutron and Osmosis, that are connected via IBC by providing the `transfer_channels` parameter. The `api_url` is the URL of the local-interchain API, and the `artifacts_dir` is the path where the compiled programs are stored. The `log_file_path` is the path where the logs will be stored. The most important part here are the chains, which are created using the `ConfigChainBuilder` with the default configurations for Neutron and Osmosis and the transfer channels between them. We provide builders for most chains but you can also create your own configurations.

## 2. Custom chain-specific setup

Some chains require additional setup to interact with others. For example, if you are going to use a liquid staking chain like Persistence, you need to register and activate the host zone to allow liquid staking of its native token. We provide helper functions that do this for you, here's an example:

```rust
    info!("Registering host zone...");
    register_host_zone(
        test_ctx
            .get_request_builder()
            .get_request_builder(PERSISTENCE_CHAIN_NAME),
        NEUTRON_CHAIN_ID,
        &connection_id,
        &channel_id,
        &native_denom,
        DEFAULT_KEY,
    )?;


    info!("Activating host zone...");
    activate_host_zone(NEUTRON_CHAIN_ID)?;
```

Other examples of this would be deploying Astroport contracts, creating Osmosis pools... We provider helper functions for pretty much all of them and we have examples for all of them in the `examples` folder.
