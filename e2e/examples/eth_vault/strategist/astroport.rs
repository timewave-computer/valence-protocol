use std::error::Error;

use async_trait::async_trait;
use cosmwasm_std::{coin, to_json_binary, CosmosMsg, Uint128, WasmMsg};
use localic_utils::NEUTRON_CHAIN_DENOM;
use log::{info, warn};
use valence_astroport_utils::astroport_native_lp_token::{
    Asset, AssetInfo, ExecuteMsg as AstroportExecuteMsg, PoolQueryMsg, ReverseSimulationResponse,
    SimulationResponse,
};
use valence_chain_client_utils::cosmos::{base_client::BaseClient, wasm_client::WasmClient};

use super::client::Strategist;

#[async_trait]
pub trait AstroportOps {
    async fn swap_ntrn_into_usdc(&self);

    async fn exit_position(&self);
    async fn enter_position(&self);

    async fn simulate_swap(
        &self,
        pool_addr: &str,
        offer_denom: &str,
        offer_amount: Uint128,
        ask_denom: &str,
    ) -> Result<Uint128, Box<dyn Error>>;

    async fn simulate_liquidation(
        &self,
        pool_addr: &str,
        shares_amount: u128,
        denom_1: &str,
        denom_2: &str,
    ) -> Result<(Uint128, Uint128), Box<dyn Error>>;

    async fn simulate_provide_liquidity(
        &self,
        pool_addr: &str,
        d1: &str,
        a1: Uint128,
        d2: &str,
        a2: Uint128,
    ) -> Result<Uint128, Box<dyn Error>>;

    async fn reverse_simulate_swap(
        &self,
        pool_addr: &str,
        offer_denom: &str,
        ask_denom: &str,
        ask_amount: Uint128,
    ) -> Result<Uint128, Box<dyn Error>>;
}

#[async_trait]
impl AstroportOps for Strategist {
    async fn reverse_simulate_swap(
        &self,
        pool_addr: &str,
        offer_denom: &str,
        ask_denom: &str,
        ask_amount: Uint128,
    ) -> Result<Uint128, Box<dyn Error>> {
        if ask_amount.is_zero() {
            info!("ask amount is zero, skipping swap simulation");
            return Ok(Uint128::zero());
        }

        let reverse_simulation_response: ReverseSimulationResponse = self
            .neutron_client
            .query_contract_state(
                pool_addr,
                PoolQueryMsg::ReverseSimulation {
                    offer_asset_info: Some(AssetInfo::NativeToken {
                        denom: offer_denom.to_string(),
                    }),
                    ask_asset: Asset {
                        info: AssetInfo::NativeToken {
                            denom: ask_denom.to_string(),
                        },
                        amount: ask_amount,
                    },
                },
            )
            .await
            .unwrap();

        info!(
            "reverse swap simulation of {ask_amount}{ask_denom} -> {ask_denom} response: {:?}",
            reverse_simulation_response
        );

        Ok(reverse_simulation_response.offer_amount)
    }

    async fn simulate_provide_liquidity(
        &self,
        pool_addr: &str,
        d1: &str,
        a1: Uint128,
        d2: &str,
        a2: Uint128,
    ) -> Result<Uint128, Box<dyn Error>> {
        if a1.is_zero() || a2.is_zero() {
            info!("proposed liquidity amount 0, skipping");
            return Ok(Uint128::zero());
        }

        let simulate_provide_response: Uint128 = self
            .neutron_client
            .query_contract_state(
                pool_addr,
                PoolQueryMsg::SimulateProvide {
                    assets: vec![
                        Asset {
                            info: AssetInfo::NativeToken {
                                denom: d1.to_string(),
                            },
                            amount: a1,
                        },
                        Asset {
                            info: AssetInfo::NativeToken {
                                denom: d2.to_string(),
                            },
                            amount: a2,
                        },
                    ],
                    slippage_tolerance: None,
                },
            )
            .await
            .unwrap();

        info!(
            "providing {a1}{d1} + {a2}{d2} liquidity would yield -> {simulate_provide_response} LP tokens",
        );

        Ok(simulate_provide_response)
    }

    /// exits the position on astroport
    async fn exit_position(&self) {
        let liquidation_account_shares_bal = self
            .neutron_client
            .query_balance(
                &self.strategy.neutron.accounts.liquidation,
                &self.strategy.neutron.denoms.lp_token,
            )
            .await
            .unwrap();

        if liquidation_account_shares_bal == 0 {
            warn!("Liquidation account must have LP shares in order to exit LP; returning");
            return;
        } else {
            info!("exiting LP position for {liquidation_account_shares_bal}shares...");
        }

        let withdraw_liquidity_msg =
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_astroport_withdrawer::msg::FunctionMsgs::WithdrawLiquidity {
                    expected_pool_ratio_range: None,
                },
            );
        let rx = self
            .neutron_client
            .execute_wasm(
                &self.strategy.neutron.libraries.astroport_lwer,
                withdraw_liquidity_msg,
                vec![],
                None,
            )
            .await
            .unwrap();
        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();
    }

    async fn simulate_swap(
        &self,
        pool_addr: &str,
        offer_denom: &str,
        offer_amount: Uint128,
        ask_denom: &str,
    ) -> Result<Uint128, Box<dyn Error>> {
        if offer_amount.is_zero() {
            info!("offer amount is zero, skipping swap simulation");
            return Ok(Uint128::zero());
        }

        let share_liquidation_response: SimulationResponse = self
            .neutron_client
            .query_contract_state(
                pool_addr,
                PoolQueryMsg::Simulation {
                    offer_asset: Asset {
                        info: AssetInfo::NativeToken {
                            denom: offer_denom.to_string(),
                        },
                        amount: offer_amount,
                    },
                    ask_asset_info: Some(AssetInfo::NativeToken {
                        denom: ask_denom.to_string(),
                    }),
                },
            )
            .await
            .unwrap();

        info!(
            "swap simulation of {offer_amount}{offer_denom} -> {ask_denom} response: {:?}",
            share_liquidation_response
        );

        Ok(share_liquidation_response.return_amount)
    }

    async fn simulate_liquidation(
        &self,
        pool_addr: &str,
        shares_amount: u128,
        denom_1: &str,
        denom_2: &str,
    ) -> Result<(Uint128, Uint128), Box<dyn Error>> {
        if shares_amount == 0 {
            info!("shares amount is zero, skipping withdraw liquidation simulation");
            return Ok((Uint128::zero(), Uint128::zero()));
        }

        let share_liquidation_response: Vec<Asset> = self
            .neutron_client
            .query_contract_state(
                pool_addr,
                PoolQueryMsg::Share {
                    amount: Uint128::from(shares_amount),
                },
            )
            .await
            .unwrap();

        let output_coins: Vec<cosmwasm_std::Coin> = share_liquidation_response
            .iter()
            .map(|c| c.as_coin().unwrap())
            .collect();

        info!(
            "Share liquidation for {shares_amount} on the pool respnose: {:?}",
            output_coins
        );

        let coin_1 = output_coins.iter().find(|c| c.denom == denom_1).unwrap();
        let coin_2 = output_coins.iter().find(|c| c.denom == denom_2).unwrap();

        Ok((coin_1.amount, coin_2.amount))
    }

    /// enters the position on astroport
    async fn enter_position(&self) {
        let deposit_account_usdc_bal = self
            .neutron_client
            .query_balance(
                &self.strategy.neutron.accounts.deposit,
                &self.strategy.neutron.denoms.usdc,
            )
            .await
            .unwrap();

        if deposit_account_usdc_bal == 0 {
            warn!("Deposit account must have USDC in order to LP; returning");
            return;
        } else {
            info!("entering LP position...");
        }

        let provide_liquidity_msg =
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_astroport_lper::msg::FunctionMsgs::ProvideSingleSidedLiquidity {
                    asset: self.strategy.neutron.denoms.usdc.to_string(),
                    limit: None,
                    expected_pool_ratio_range: None,
                },
            );
        let rx = self
            .neutron_client
            .execute_wasm(
                &self.strategy.neutron.libraries.astroport_lper,
                provide_liquidity_msg,
                vec![],
                None,
            )
            .await
            .unwrap();
        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();
    }

    /// swaps counterparty denom into usdc
    async fn swap_ntrn_into_usdc(&self) {
        let withdraw_account_ntrn_bal = self
            .neutron_client
            .query_balance(
                &self.strategy.neutron.accounts.withdraw,
                NEUTRON_CHAIN_DENOM,
            )
            .await
            .unwrap();

        if withdraw_account_ntrn_bal == 0 {
            warn!("Withdraw account must have NTRN in order to swap into USDC; returning");
            return;
        } else {
            info!("swapping {withdraw_account_ntrn_bal}NTRN into USDC...");
        }

        let swap_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.strategy.neutron.target_pool.to_string(),
            msg: to_json_binary(&AstroportExecuteMsg::Swap {
                offer_asset: Asset {
                    info: AssetInfo::NativeToken {
                        denom: NEUTRON_CHAIN_DENOM.to_string(),
                    },
                    amount: withdraw_account_ntrn_bal.into(),
                },
                max_spread: None,
                belief_price: None,
                to: None,
                ask_asset_info: None,
            })
            .unwrap(),
            funds: vec![coin(withdraw_account_ntrn_bal, NEUTRON_CHAIN_DENOM)],
        });

        let base_account_execute_msgs = valence_account_utils::msg::ExecuteMsg::ExecuteMsg {
            msgs: vec![swap_msg],
        };

        let rx = self
            .neutron_client
            .execute_wasm(
                &self.strategy.neutron.accounts.withdraw,
                base_account_execute_msgs,
                vec![],
                None,
            )
            .await
            .unwrap();

        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();
    }
}
