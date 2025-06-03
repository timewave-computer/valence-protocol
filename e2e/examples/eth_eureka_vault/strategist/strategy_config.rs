use serde::{Deserialize, Serialize};
use valence_strategist_utils::worker::ValenceWorkerTomlSerde;

// here we define the inputs for the strategy.
// this configuration type should have sufficient information
// to create the strategy, initialize the (g)rpc clients, and
// begin with the execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub neutron: neutron::NeutronStrategyConfig,
    pub ethereum: ethereum::EthereumStrategyConfig,
}

// default impl serde trait to enable toml config file parsing
impl ValenceWorkerTomlSerde for StrategyConfig {}

// configuration relevant for the neutron part of the strategy
pub mod neutron {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NeutronStrategyConfig {
        pub grpc_url: String,
        pub grpc_port: String,
        pub chain_id: String,
        pub mnemonic: String,
        pub target_pool: String,
        pub denoms: NeutronDenoms,
        pub accounts: NeutronAccounts,
        pub libraries: NeutronLibraries,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NeutronDenoms {
        pub lp_token: String,
        pub wbtc: String,
        pub ntrn: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NeutronAccounts {
        pub deposit: String,
        pub position: String,
        pub withdraw: String,
        pub liquidation: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NeutronLibraries {
        pub neutron_ibc_transfer: String,
        pub astroport_lper: String,
        pub astroport_lwer: String,
        pub liquidation_forwarder: String,
        pub authorizations: String,
        pub processor: String,
    }
}

// configuration relevant for the ethereum part of the strategy
pub mod ethereum {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EthereumStrategyConfig {
        pub rpc_url: String,
        pub mnemonic: String,
        pub denoms: EthereumDenoms,
        pub accounts: EthereumAccounts,
        pub libraries: EthereumLibraries,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EthereumDenoms {
        pub wbtc: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EthereumAccounts {
        pub deposit: String,
        pub withdraw: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EthereumLibraries {
        pub valence_vault: String,
        pub eureka_transfer: String,
        pub lite_processor: String,
    }
}
