use std::{error::Error, str::FromStr};

use alloy::primitives::Address;
use async_trait::async_trait;
use cosmwasm_std::Uint128;
use log::info;
use valence_authorization_utils::{authorization::Priority, msg::PermissionedMsg};
use valence_clearing_queue::msg::ObligationsResponse;
use valence_domain_clients::{
    cosmos::{base_client::BaseClient, wasm_client::WasmClient},
    evm::request_provider_client::RequestProviderClient,
};
use valence_e2e::utils::{
    solidity_contracts::{
        sol_authorizations::Authorizations, sol_lite_processor::LiteProcessor, IBCEurekaTransfer,
        OneWayVault, ERC20,
    },
    worker::ValenceWorker,
};

use crate::strategy_config::Strategy;

// implement the ValenceWorker trait for the Strategy struct.
// This trait defines the main loop of the strategy and inherits
// the default implementation for spawning the worker.
#[async_trait]
impl ValenceWorker for Strategy {
    fn get_name(&self) -> String {
        "Valence X-Vault: ETH-NEUTRON BTC".to_string()
    }

    async fn cycle(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let worker_name = self.get_name();
        info!("{worker_name}: Starting cycle...");

        let eth_rp = self.eth_client.get_request_provider().await?;

        // ======================= ETH Side setup =============================
        // here we build up the Ethereum domain state for the strategy cycle
        let eth_authorizations_contract = Authorizations::new(
            Address::from_str(&self.cfg.ethereum.authorizations)?,
            &eth_rp,
        );
        let eth_processor_contract =
            LiteProcessor::new(Address::from_str(&self.cfg.ethereum.processor)?, &eth_rp);

        let eth_wbtc_contract =
            ERC20::new(Address::from_str(&self.cfg.ethereum.denoms.wbtc)?, &eth_rp);
        let eth_one_way_vault_contract = OneWayVault::new(
            Address::from_str(&self.cfg.ethereum.libraries.one_way_vault)?,
            &eth_rp,
        );
        let eth_eureka_transfer_contract = IBCEurekaTransfer::new(
            Address::from_str(&self.cfg.ethereum.libraries.eureka_forwarder)?,
            &eth_rp,
        );

        // first we carry out the deposit flow
        self.deposit().await?;

        // after deposit flow is complete, we process the new obligations
        self.register_withdraw_obligations().await?;

        // with new obligations registered into the clearing queue, we
        // carry out the settlements
        self.settlement().await?;

        Ok(())
    }
}

impl Strategy {
    /// carries out the steps needed to bring the new deposits from Ethereum to
    /// Neutron (via Cosmos Hub) before depositing them into Mars protocol.
    async fn deposit(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        // 1. query the ethereum deposit account balance

        // 2. validate that the deposit account balance exceeds the eureka routing
        // threshold amount (from cfg)

        // 3. perform IBC-Eureka transfer to Cosmos Hub ICA

        // 4. block execution until the funds arrive to the Cosmos Hub ICA owned
        // by the Valence Interchain Account on Neutron

        // 5. Initiate ICA-IBC-Transfer from Cosmos Hub ICA to Neutron program
        // deposit account

        // 6. block execution until funds arrive to the Neutron program deposit
        // account

        // 7. use Valence Forwarder to route funds from the Neutron program
        // deposit account to the Mars deposit account

        // 8. use Mars Lending library to deposit funds from Mars deposit account
        // into Mars protocol

        Ok(())
    }

    /// reads the newly submitted withdrawal obligations that are not yet
    /// present in the Clearing Queue, generates their zero-knowledge proofs,
    /// and posts them into the Clearing queue in order.
    async fn register_withdraw_obligations(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        // 1. query the Clearing Queue library for the latest posted withdraw request ID

        // 2. query the OneWayVault for emitted events and filter them such that
        // only requests with id greater than the one queried in step 1. are fetched

        // 3. process the new OneWayVault Withdraw events in order from the oldest
        // to the newest, posting them to the coprocessor to obtain a ZKP

        // 4. preserving the order, post the ZKPs obtained in step 3. to the Neutron
        // Authorizations contract, enqueuing them to the processor

        // 5. tick the processor to register the obligations to the clearing queue

        Ok(())
    }

    /// performs the final settlement of registered withdrawal obligations in
    /// the Clearing Queue library. this involves topping up the settlement
    /// account with funds necessary to carry out all withdrawal obligations
    /// in the queue.
    async fn settlement(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        // 1. query the current settlement account balance
        let settlement_acc_bal = self
            .neutron_client
            .query_balance(
                &self.cfg.neutron.accounts.settlement,
                &self.cfg.neutron.denoms.wbtc,
            )
            .await?;

        // 2. query the Clearing Queue and calculate the total active obligations
        let clearing_queue: ObligationsResponse = self
            .neutron_client
            .query_contract_state(
                &self.cfg.neutron.libraries.clearing,
                valence_clearing_queue::msg::QueryMsg::PendingObligations {
                    from: None,
                    to: None,
                },
            )
            .await?;

        let total_queue_obligations: u128 = clearing_queue
            .obligations
            .iter()
            .map(|o| o.payout_coins[0].amount.u128())
            .sum();

        // 3. if settlement account balance is insufficient to cover the active
        // obligations, we perform the Mars protocol withdrawals
        if settlement_acc_bal < total_queue_obligations {
            // 3. simulate Mars protocol withdrawal to obtain the funds necessary
            // to fulfill all active withdrawal requests
            let obligations_delta = total_queue_obligations - settlement_acc_bal;

            let mars_simulation_response = 0; // TODO

            // 4. call the Mars lending library to perform the withdrawal.
            // This will deposit the underlying assets directly to the settlement account.
            // TODO
        }

        // 5. queue the Clearing Queue settlement requests to the processor
        self.neutron_client
            .execute_wasm(
                &self.cfg.neutron.authorizations,
                valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
                    valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
                        label: "TBD".to_string(),
                        messages: vec![],
                        ttl: None,
                    },
                ),
                vec![],
                None,
            )
            .await?;

        // 6. tick the processor until all withdraw obligations are settled
        self.neutron_client
            .execute_wasm(
                &self.cfg.neutron.processor,
                valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
                    valence_processor_utils::msg::PermissionlessMsg::Tick {},
                ),
                vec![],
                None,
            )
            .await?;

        Ok(())
    }
}
