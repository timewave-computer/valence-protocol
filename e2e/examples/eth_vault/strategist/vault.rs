use std::{error::Error, str::FromStr};

use alloy::primitives::U256;
use async_trait::async_trait;
use cosmwasm_std::{Decimal, Uint128};
use localic_utils::NEUTRON_CHAIN_DENOM;
use log::info;
use valence_astroport_utils::astroport_native_lp_token::{ConfigResponse, PoolQueryMsg};
use valence_chain_client_utils::{
    cosmos::{base_client::BaseClient, wasm_client::WasmClient},
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_e2e::utils::{
    solidity_contracts::{MockERC20, ValenceVault},
    UUSDC_DENOM,
};

use crate::{strategist::astroport::AstroportOps, Strategist};

#[async_trait]
pub trait EthereumVault {
    async fn calculate_redemption_rate(&self) -> Result<Decimal, Box<dyn Error>>;
    async fn calculate_total_fee(&self) -> Result<u32, Box<dyn Error>>;
    async fn calculate_netting_amount(&self) -> Result<u32, Box<dyn Error>>;

    async fn vault_update(
        &self,
        rate: U256,
        withdraw_fee_bps: u32,
        netting_amount: U256,
    ) -> Result<(), Box<dyn Error>>;
}

#[async_trait]
impl EthereumVault for Strategist {
    async fn calculate_netting_amount(&self) -> Result<u32, Box<dyn Error>> {
        // 3. Find netting amount N
        //   1. query Vault for total pending withdrawals (USDC)
        //   2. query Eth deposit account for USDC balance
        //   3. N = min(deposit_bal, withdrawals_sum)
        Ok(0)
    }

    async fn calculate_total_fee(&self) -> Result<u32, Box<dyn Error>> {
        // 2. Find withdraw fee F_total
        //   1. query Vault fee from the Eth vault
        //   2. query the dex position for their fee
        //   3. F_total = F_vault + F_position
        let eth_rp = self.eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(self.vault_addr, &eth_rp);

        let fees = self
            .eth_client
            .query(valence_vault.config())
            .await
            .unwrap()
            .fees;

        info!("vault fees: {:?}", fees);

        let pool_addr = self.pool_addr.to_string();
        let cl_pool_cfg: ConfigResponse = self
            .neutron_client
            .query_contract_state(&pool_addr, PoolQueryMsg::Config {})
            .await
            .unwrap();

        let pool_fee = match cl_pool_cfg.try_get_cl_params() {
            Some(cl_params) => {
                (cl_params.out_fee * Decimal::from_ratio(10000u128, 1u128))
                    .atomics()
                    .u128() as u32 // intentionally truncating
            }
            None => 0u32,
        };

        Ok(fees.platformFeeBps + pool_fee)
    }

    /// concludes the vault epoch and updates the Valence Vault state
    async fn vault_update(
        &self,
        rate: U256,
        withdraw_fee_bps: u32,
        netting_amount: U256,
    ) -> Result<(), Box<dyn Error>> {
        info!(
            "Updating Ethereum Vault with:
            rate: {rate}
            witdraw_fee_bps: {withdraw_fee_bps}
            netting_amount: {netting_amount}"
        );
        let eth_rp = self.eth_client.get_request_provider().await.unwrap();

        let clamped_withdraw_fee = withdraw_fee_bps.clamp(1, 10_000);

        let valence_vault = ValenceVault::new(self.vault_addr, &eth_rp);

        let update_msg = valence_vault
            .update(rate, clamped_withdraw_fee, netting_amount)
            .into_transaction_request();

        let update_result = self.eth_client.execute_tx(update_msg).await;

        if let Err(e) = &update_result {
            info!("Update failed: {:?}", e);
            panic!("failed to update vault");
        }

        Ok(())
    }

    async fn calculate_redemption_rate(&self) -> Result<Decimal, Box<dyn Error>> {
        let eth_rp = self.eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(self.vault_addr, &eth_rp);
        let eth_usdc_erc20 = MockERC20::new(self.ethereum_usdc_erc20, &eth_rp);

        let neutron_position_acc = self
            .neutron_program_accounts
            .position_account
            .to_string()
            .unwrap();
        let noble_inbound_ica = self
            .neutron_program_accounts
            .noble_inbound_ica
            .remote_addr
            .to_string();
        let neutron_deposit_acc = self
            .neutron_program_accounts
            .deposit_account
            .to_string()
            .unwrap();
        let eth_deposit_acc = self.eth_program_accounts.deposit;

        // 1. query total shares issued from the vault
        let vault_issued_shares = self
            .eth_client
            .query(valence_vault.totalSupply())
            .await
            .unwrap()
            ._0;
        let vault_current_rate = self
            .eth_client
            .query(valence_vault.redemptionRate())
            .await
            .unwrap()
            ._0;
        info!("current vault redemption rate: {:?}", vault_current_rate);

        // 2. query shares in position account and simulate their liquidation for USDC
        let neutron_position_acc_shares = self
            .neutron_client
            .query_balance(&neutron_position_acc, &self.lp_token_denom)
            .await
            .unwrap();
        let (usdc_amount, ntrn_amount) = self
            .simulate_liquidation(
                &self.pool_addr,
                neutron_position_acc_shares,
                &self.uusdc_on_neutron_denom,
                NEUTRON_CHAIN_DENOM,
            )
            .await
            .unwrap();

        let swap_simulation_output = self
            .simulate_swap(
                &self.pool_addr,
                NEUTRON_CHAIN_DENOM,
                ntrn_amount,
                &self.uusdc_on_neutron_denom,
            )
            .await
            .unwrap();

        // 3. query pending deposits (eth deposit acc + noble inbound ica + neutron deposit acc)

        let eth_deposit_usdc = self
            .eth_client
            .query(eth_usdc_erc20.balanceOf(eth_deposit_acc))
            .await
            .unwrap()
            ._0;
        let noble_inbound_ica_usdc = self
            .noble_client
            .query_balance(&noble_inbound_ica, UUSDC_DENOM)
            .await
            .unwrap();
        let neutron_deposit_acc_usdc = self
            .neutron_client
            .query_balance(&neutron_deposit_acc, &self.uusdc_on_neutron_denom)
            .await
            .unwrap();

        //   4. R = total_shares / total_assets
        let normalized_eth_balance = Uint128::from_str(&eth_deposit_usdc.to_string()).unwrap();

        let total_assets = noble_inbound_ica_usdc
            + neutron_deposit_acc_usdc
            + normalized_eth_balance.u128()
            + usdc_amount.u128()
            + swap_simulation_output.u128();
        let normalized_shares = Uint128::from_str(&vault_issued_shares.to_string()).unwrap();

        info!("total assets: {}USDC", total_assets);
        info!("total shares: {}", normalized_shares.u128());
        match Decimal::checked_from_ratio(normalized_shares, total_assets) {
            Ok(ratio) => {
                info!("redemption rate: {}", ratio);
                Ok(ratio)
            }
            Err(_) => Ok(Decimal::zero()),
        }
    }
}
