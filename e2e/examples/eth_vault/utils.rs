use std::error::Error;

use cosmwasm_std_old::Coin as BankCoin;
use localic_std::modules::bank;
use localic_utils::{
    utils::{ethereum::EthClient, test_context::TestContext},
    DEFAULT_KEY, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_NAME,
};

use log::info;

use valence_e2e::utils::{
    hyperlane::{set_up_cw_hyperlane_contracts, set_up_eth_hyperlane_contracts, set_up_hyperlane},
    ETHEREUM_HYPERLANE_DOMAIN, HYPERLANE_RELAYER_NEUTRON_ADDRESS,
};

use crate::program::ProgramHyperlaneContracts;

pub fn hyperlane_plumbing(
    test_ctx: &mut TestContext,
    eth: &EthClient,
) -> Result<ProgramHyperlaneContracts, Box<dyn Error>> {
    info!("uploading cosmwasm hyperlane contracts...");
    // Upload all Hyperlane contracts to Neutron
    let neutron_hyperlane_contracts = set_up_cw_hyperlane_contracts(test_ctx)?;

    info!("uploading evm hyperlane conrtacts...");
    // Deploy all Hyperlane contracts on Ethereum
    let eth_hyperlane_contracts = set_up_eth_hyperlane_contracts(eth, ETHEREUM_HYPERLANE_DOMAIN)?;

    info!("setting up hyperlane connection Neutron <> Ethereum");
    set_up_hyperlane(
        "hyperlane-net",
        vec!["localneutron-1-val-0-neutronic", "anvil"],
        "neutron",
        "ethereum",
        &neutron_hyperlane_contracts,
        &eth_hyperlane_contracts,
    )?;

    // Since we are going to relay callbacks to Neutron, let's fund the Hyperlane relayer account with some tokens
    info!("Fund relayer account...");
    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        HYPERLANE_RELAYER_NEUTRON_ADDRESS,
        &[BankCoin {
            denom: NEUTRON_CHAIN_DENOM.to_string(),
            amount: 5_000_000u128.into(),
        }],
        &BankCoin {
            denom: NEUTRON_CHAIN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    Ok(ProgramHyperlaneContracts {
        neutron_hyperlane_contracts,
        eth_hyperlane_contracts,
    })
}
