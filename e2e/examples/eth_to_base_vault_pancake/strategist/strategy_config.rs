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
        pub weth: String,
        pub usdc: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EthereumAccounts {
        pub vault_deposit: String,
        pub vault_withdraw: String,
        pub aave_input: String,
        pub cctp_input: String,
        pub standard_bridge_input: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EthereumLibraries {
        pub vault: String,
        pub cctp_transfer: String,
        pub standard_bridge_transfer: String,
        pub aave_position_manager: String,
        pub forwarder_vault_deposit_to_aave_input: String,
        pub forwarder_vault_deposit_to_standard_bridge_input: String,
    }
}

pub mod base {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BaseStrategyConfig {
        pub rpc_url: String,
        pub mnemonic: String,
        pub denoms: BaseDenoms,
        pub accounts: BaseAccounts,
        pub libraries: BaseLibraries,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BaseDenoms {
        pub weth: String,
        pub usdc: String,
        pub cake: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BaseAccounts {
        pub pancake_input: String,
        pub pancake_output: String,
        pub cctp_input: String,
        pub standard_bridge_input: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BaseLibraries {
        pub pancake_position_manager: String,
        pub cctp_transfer: String,
        pub standard_bridge_transfer: String,
        pub pancake_output_to_input_forwarder: String,
        pub pancake_output_to_cctp_input_forwarder: String,
        pub pancake_output_to_standard_bridge_input_forwarder: String,
    }
}
