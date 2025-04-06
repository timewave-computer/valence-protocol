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

## Example

We have a public repo that include configs for major persistant environments (like mainnet and testnet)

It can be used directly in the manager to deploy on those environments or as example to a working manager config layout.

[Public manager configs](https://github.com/timewave-computer/valence-program-manager-config)

## Config fields

### Chains

This is a map of `chain_id => ChainInfo`.  

It allows the manager to connect to chains that are required by your program and to execute actions on chain and query data.

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

* Note - Your program might require multiple chains, all chains must be included in the config or the manager will fail.

* Note - Neutron chain must be included even if the program is not using it as a domain.

### Contracts

Contracts field includes all the code ids of contract

```rust
pub struct Contracts {
    pub code_ids: HashMap<String, HashMap<String, u64>>,
}
```

`code_ids` field is a map of `chain_id => map(contract_name => code_id)`

This allows the manager to find the code id of a contract on a specific chain to instantiate it.

### Bridges

The bridge is a complex map of bridge information needed for cross-chain operations.

Easiest way to explain it is by `toml` format:

```toml
[bridges.neutron.juno.polytone.neutron]
voice_addr      = "neutron15c0d3k8nf5t82zzkl8l7he3smx033hsr9dvzjeeuj7e8n46rqy5se0pn3e"
note_addr       = "neutron174ne8p7zh539sht8sfjsa9r6uwe3pzlvqedr0yquml9crfzsfnlshvlse8"
other_note_port = "wasm.juno1yt5kcplze0sark8f55fklk70uay3863t5q3j3a8kgvs3rlmjya9qys0d2y"
connection_id   = "connection-95"
channel_id      = "channel-4721"
[bridges.neutron.juno.polytone.juno]
voice_addr      = "juno1c9hx3q7sd2d0xgknc52ft6qsqxemkuxh3nt8d4rmdtdua25x5h0sdd2zm5"
note_addr       = "juno1yt5kcplze0sark8f55fklk70uay3863t5q3j3a8kgvs3rlmjya9qys0d2y"
other_note_port = "wasm.neutron174ne8p7zh539sht8sfjsa9r6uwe3pzlvqedr0yquml9crfzsfnlshvlse8"
connection_id   = "connection-530"
channel_id      = "channel-620"
```

We are providing a bridge information here between `neutron` and `juno` chains, the bridge we are using is `polytone`, and the first information is for the `neutron` "side", while the second information is for the `juno` "side".

### General

```rust
pub struct GeneralConfig {
    pub registry_addr: String,
}
```

General field holds general information that is needed for the manager to work:

- `registry_addr` - The registry contract address on neutron.