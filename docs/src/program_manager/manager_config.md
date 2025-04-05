# Manager config

The program manager requires information like chain connection details and bridges details, this must be provided to the manager via a config.

```rust
pub struct Config {
    // Map of chain connections details
    pub chains: HashMap<String, ChainInfo>,
    // Contract information per chain for instantiation
    pub contracts: Contracts,
    // Map of bridges information
    pub bridges: HashMap<String, HashMap<String, Bridge>>,
    pub general: GeneralConfig,
}
```

## Setup

The manager config is a global mutateable config and can be read and set from anywhere in your project.

### Get config

You can get the config like this: 

```rust
let manager_config = valence_program_manager::config::GLOBAL_CONFIG.lock().await
```

### Write config

Writing to the config is possible with: 

```rust
let mut manager_config = valence_program_manager::config::GLOBAL_CONFIG.lock().await

// Mutate field
manager_config.general.registry_addr = "addr1234".to_string();

// Write full config
*manager_config = new_manager_config;
```

### Non-async functions

The manager config is using `tokio::sync::Mutex`, because of that you need to use blocking operation in non-async functions, like this:

```rust
let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
rt.block_on(valence_program_manager::config::GLOBAL_CONFIG.lock())
```

* Note - You must be careful with blocking, the manager might be blocked from accessing the global config if not freed properly.

## Chains

This is a map of the `chain_id => ChainInfo`.  
It's required for the manager to connect to the chains, execute and query 

```rust
pub struct ChainInfo {
    pub name: String,
    pub rpc: String,
    pub grpc: String,
    pub prefix: String,
    pub gas_price: String,
    pub gas_denom: String,
    pub coin_type: u64,
}
```