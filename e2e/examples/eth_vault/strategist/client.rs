use std::{error::Error, str::FromStr};

use alloy::{
    primitives::{Address, U256},
    providers::Provider,
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use cosmwasm_std::Uint128;
use localic_utils::{NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_ID};
use log::{info, warn};
use tokio::runtime::Runtime;
use valence_chain_client_utils::{
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
    neutron::NeutronClient,
    noble::NobleClient,
};

use valence_e2e::{
    async_run,
    utils::{
        solidity_contracts::{MockERC20, ValenceVault},
        NOBLE_CHAIN_DENOM,
    },
};

use crate::{
    strategist::{astroport::AstroportOps, routing::EthereumVaultRouting, vault::EthereumVault},
    utils::{get_current_second, wait_until_next_minute},
};

use super::setup::StrategyConfig;

pub(crate) struct Strategist {
    // (g)RPC clients
    pub eth_client: EthereumClient,
    pub noble_client: NobleClient,
    pub neutron_client: NeutronClient,

    pub strategy: StrategyConfig,
}

impl Strategist {
    pub fn new(rt: &Runtime, strategy: StrategyConfig) -> Result<Self, Box<dyn Error>> {
        // build the noble client
        let noble_client = async_run!(rt, {
            NobleClient::new(
                &strategy.noble.grpc_url,
                &strategy.noble.grpc_port,
                &strategy.noble.mnemonic,
                &strategy.noble.chain_id,
                NOBLE_CHAIN_DENOM,
            )
            .await
            .expect("failed to create noble client")
        });

        // build the neutron client
        let neutron_client = async_run!(rt, {
            NeutronClient::new(
                &strategy.neutron.grpc_url,
                &strategy.neutron.grpc_port,
                &strategy.neutron.mnemonic,
                NEUTRON_CHAIN_ID,
            )
            .await
            .expect("failed to create neutron client")
        });

        // build the eth client
        let eth_client = EthereumClient {
            rpc_url: strategy.ethereum.rpc_url.to_string(),
            signer: MnemonicBuilder::<English>::default()
                .phrase(strategy.ethereum.mnemonic.clone())
                .index(7)? // derive the mnemonic at a different index to avoid nonce issues
                .build()?,
        };

        Ok(Self {
            eth_client,
            noble_client,
            neutron_client,
            strategy,
        })
    }
}

impl Strategist {
    pub async fn start(self) {
        let mut i = 0;

        loop {
            // strategist runs every minute, usually taking around 12sec to complete
            // and sleeping the remaining seconds
            wait_until_next_minute().await;
            info!(
                "strategist loop #{i} started at second {}",
                get_current_second()
            );
            let eth_rp = self.eth_client.get_request_provider().await.unwrap();
            let valence_vault = ValenceVault::new(
                Address::from_str(&self.strategy.ethereum.libraries.valence_vault).unwrap(),
                &eth_rp,
            );
            let eth_usdc_erc20 = MockERC20::new(
                Address::from_str(&self.strategy.ethereum.denoms.usdc_erc20).unwrap(),
                &eth_rp,
            );

            // 1. calculate the amount of usdc needed to fulfill
            // the active withdraw obligations
            let pending_obligations = self
                .eth_client
                .query(valence_vault.totalAssetsToWithdrawNextUpdate())
                .await
                .unwrap()
                ._0;

            // 2. query ethereum program accounts for their usdc balances
            let eth_deposit_acc_usdc_bal = self
                .eth_client
                .query(eth_usdc_erc20.balanceOf(
                    Address::from_str(&self.strategy.ethereum.accounts.deposit).unwrap(),
                ))
                .await
                .unwrap()
                ._0;

            // 3. see if pending obligations can be netted and update the pending
            // obligations accordingly
            let netting_amount = pending_obligations.min(eth_deposit_acc_usdc_bal);
            info!("netting amount: {netting_amount}");

            let pending_obligations = pending_obligations.checked_sub(netting_amount).unwrap();
            info!("updated pending obligations: {pending_obligations}");

            // 4. lp shares to be liquidated will yield untrn+uusdc. to figure out
            // the amount of ntrn needed to get 1/2 of the obligations, we half the
            // usdc amount
            let missing_usdc_amount: u128 = pending_obligations
                .try_into()
                .map_err(|_| "Pending obligations U256 Value too large for u128")
                .unwrap();
            info!("total to withdraw: {missing_usdc_amount}USDC");

            let halved_usdc_obligation_amt = Uint128::new(missing_usdc_amount / 2);
            info!("halved usdc obligation amount: {halved_usdc_obligation_amt}");

            // 5. simulate how many untrn we need to obtain half of the
            // missing usdc obligation amount
            let expected_untrn_amount = self
                .reverse_simulate_swap(
                    &self.strategy.neutron.target_pool.to_string(),
                    NEUTRON_CHAIN_DENOM,
                    &self.strategy.neutron.denoms.usdc,
                    halved_usdc_obligation_amt,
                )
                .await
                .unwrap();
            info!("reverse swap simulation response: {expected_untrn_amount}untrn => {halved_usdc_obligation_amt}usdc");

            // 6. simulate liquidity provision with the 1/2 usdc amount and the equivalent untrn amount.
            // this will give us the amount of shares that are equivalent to those tokens.
            // TODO: think if this simulation makes sense here as the order is reversed.
            let shares_to_liquidate = self
                .simulate_provide_liquidity(
                    &self.strategy.neutron.target_pool,
                    &self.strategy.neutron.denoms.usdc,
                    halved_usdc_obligation_amt,
                    NEUTRON_CHAIN_DENOM,
                    expected_untrn_amount,
                )
                .await
                .unwrap();

            // 7. forward the shares to be liquidated from the position account to the withdraw account
            self.forward_shares_for_liquidation(shares_to_liquidate)
                .await;

            // 8. liquidate the forwarded shares to get USDC+NTRN
            self.exit_position().await;

            // 9. swap NTRN into USDC to obtain the full obligation amount
            self.swap_ntrn_into_usdc().await;

            // 10. update the vault to conclude the previous epoch. we already derived
            // the netting amount in step #3, so we need to find the redemption rate and
            // total fee.
            let redemption_rate = self.calculate_redemption_rate().await.unwrap();
            let total_fee = self.calculate_total_fee().await.unwrap();
            let r = U256::from(redemption_rate.atomics().u128());

            let clamped_withdraw_fee = total_fee.clamp(1, 10_000);

            info!(
                "Updating Ethereum Vault with:
                rate: {r}
                witdraw_fee_bps: {clamped_withdraw_fee}
                netting_amount: {netting_amount}"
            );

            let update_result = self
                .eth_client
                .execute_tx(
                    valence_vault
                        .update(r, clamped_withdraw_fee, netting_amount)
                        .into_transaction_request(),
                )
                .await;

            if let Err(e) = &update_result {
                info!("Update failed: {:?}", e);
                panic!("failed to update vault");
            }

            if eth_rp
                .get_transaction_receipt(update_result.unwrap().transaction_hash)
                .await
                .map_err(|e| warn!("Error: {:?}", e))
                .unwrap()
                .is_none()
            {
                warn!("Failed to get update_vault tx receipt")
            };

            // 11. pull the funds due for deposit from origin to position domain
            //   1. cctp transfer eth deposit acc -> noble inbound ica
            //   2. ica ibc transfer noble inbound ica -> neutron deposit acc
            self.route_eth_to_noble().await;
            self.route_noble_to_neutron().await;

            // 12. enter the position with funds available in neutron deposit acc
            self.enter_position().await;

            // 13. pull the funds due for withdrawal from position to origin domain
            //   1. ibc transfer neutron withdraw acc -> noble outbound ica
            //   2. cctp transfer noble outbound ica -> eth withdraw acc
            self.route_neutron_to_noble().await;
            self.route_noble_to_eth().await;

            info!(
                "strategist loop #{i} completed at second {}",
                get_current_second()
            );

            i += 1;
        }
    }
}
