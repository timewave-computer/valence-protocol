use std::{error::Error, path::Path};

use valence_e2e::utils::worker::ValenceWorkerTomlSerde;

use super::ethereum_config::{
    EthereumAccounts, EthereumDenoms, EthereumLibraries, EthereumStrategyConfig,
};

fn main() -> Result<(), Box<dyn Error>> {
    let denoms = EthereumDenoms {
        wbtc: "0xWBTC_ADDR...".to_string(),
    };

    let accounts = EthereumAccounts {
        deposit: "0xDeposit_account...".to_string(),
    };

    let libraries = EthereumLibraries {
        one_way_vault: "0xone_way_vault...".to_string(),
        eureka_forwarder: "0xeureka_fwd...".to_string(),
    };

    let eth_cfg = EthereumStrategyConfig {
        rpc_url: "https://...".to_string(),
        mnemonic: "racoon racoon racoon racoon racoon racoon...".to_string(),
        authorizations: "0xauthorizations...".to_string(),
        processor: "0xprocessor...".to_string(),
        denoms,
        accounts,
        libraries,
    };

    let temp_path = Path::new("./e2e/examples/eth_btc_vault/ethereum/example_strategy.toml");

    eth_cfg.to_file(temp_path)?;

    Ok(())
}
