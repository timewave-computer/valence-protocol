use std::{fs, path::Path};

use serde::{Deserialize, Serialize};
use valence_e2e::utils::worker::{ValenceWorker, ValenceWorkerTomlSerde};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub noble: noble::NobleStrategyConfig,
    pub neutron: neutron::NeutronStrategyConfig,
    pub ethereum: ethereum::EthereumStrategyConfig,
}

impl ValenceWorker for StrategyConfig {
    fn get_name(&self) -> String {
        "Valence X-Vault: ETH-NOBLE-NEUTRON".to_string()
    }
}

impl ValenceWorkerTomlSerde for StrategyConfig {}

pub mod noble {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NobleStrategyConfig {
        pub grpc_url: String,
        pub grpc_port: String,
        pub chain_id: String,
        pub mnemonic: String,
    }
}

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
        pub usdc: String,
        pub ntrn: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NeutronAccounts {
        pub deposit: String,
        pub position: String,
        pub withdraw: String,
        pub liquidation: String,
        pub noble_inbound_ica: IcaAccount,
        pub noble_outbound_ica: IcaAccount,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct IcaAccount {
        pub library_account: String,
        pub remote_addr: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NeutronLibraries {
        pub neutron_ibc_transfer: String,
        pub noble_inbound_transfer: String,
        pub noble_cctp_transfer: String,
        pub astroport_lper: String,
        pub astroport_lwer: String,
        pub liquidation_forwarder: String,
        pub authorizations: String,
        pub processor: String,
    }
}

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
        pub usdc_erc20: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EthereumAccounts {
        pub deposit: String,
        pub withdraw: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EthereumLibraries {
        pub valence_vault: String,
        pub cctp_forwarder: String,
        pub lite_processor: String,
    }
}
