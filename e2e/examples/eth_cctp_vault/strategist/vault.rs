use std::{error::Error, str::FromStr};

use alloy::primitives::Address;
use async_trait::async_trait;
use cosmwasm_std::{Decimal, Uint128};
use localic_utils::NEUTRON_CHAIN_DENOM;
use log::info;
use valence_chain_client_utils::{
    cosmos::base_client::BaseClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_e2e::utils::{
    solidity_contracts::{MockERC20, ValenceVault},
    UUSDC_DENOM,
};

use crate::strategist::astroport::AstroportOps;

use super::strategy::Strategy;

#[async_trait]
pub trait EthereumVault {
    async fn calculate_total_fee(&self) -> Result<u32, Box<dyn Error>>;
}

#[async_trait]
impl EthereumVault for Strategy {
    async fn calculate_total_fee(&self) -> Result<u32, Box<dyn Error>> {
        // 2. Find withdraw fee F_total
        //   1. query Vault fee from the Eth vault
        //   2. query the dex position for their fee
        //   3. F_total = F_vault + F_position
        let eth_rp = self.eth_client.get_request_provider().await?;
        let valence_vault = ValenceVault::new(
            Address::from_str(&self.cfg.ethereum.libraries.valence_vault)?,
            &eth_rp,
        );

        let vault_cfg = self.eth_client.query(valence_vault.config()).await?;

        let fees = vault_cfg.fees;

        info!("vault fees: {:?}", fees);

        // TODO: need to decide which fee we want to take (mid_fee, out_fee, etc).
        // let pool_addr = self.pool_addr.to_string();
        // let cl_pool_cfg: ConfigResponse = self
        //     .neutron_client
        //     .query_contract_state(&pool_addr, PoolQueryMsg::Config {})
        //     .await
        //     .unwrap();

        // hardcoding flat fee for now
        let withdraw_fee = 100u32;

        if withdraw_fee > vault_cfg.maxWithdrawFeeBps {
            log::warn!(
                "Calculated withdraw fee {withdraw_fee} exceeds max allowed {}, using max",
                vault_cfg.maxWithdrawFeeBps
            );
            return Ok(vault_cfg.maxWithdrawFeeBps);
        }

        Ok(withdraw_fee)
    }
}
