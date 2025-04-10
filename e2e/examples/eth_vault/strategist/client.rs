use std::{error::Error, u128};

use alloy::{
    primitives::{Address, U256},
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use cosmwasm_std::{Uint128, Uint256};
use localic_utils::{NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_ID};
use log::info;
use neutron_sdk::interchain_queries::helpers::uint256_to_u128;
use tokio::runtime::Runtime;
use valence_chain_client_utils::{
    ethereum::EthereumClient, neutron::NeutronClient, noble::NobleClient,
};

use valence_e2e::{
    async_run,
    utils::{
        parse::{get_chain_field_from_local_ic_log, get_grpc_address_and_port_from_url},
        ADMIN_MNEMONIC, DEFAULT_ANVIL_RPC_ENDPOINT, NOBLE_CHAIN_DENOM, NOBLE_CHAIN_ID,
    },
};

use crate::{
    evm::{EthereumProgramAccounts, EthereumProgramLibraries},
    program::{NeutronProgramAccounts, NeutronProgramLibraries},
    strategist::{
        astroport::AstroportOps, routing::EthereumVaultRouting, u256_to_uint256,
        vault::EthereumVault,
    },
    utils::{get_current_second, wait_until_next_minute},
};

pub(crate) struct Strategist {
    // (g)RPC clients
    pub eth_client: EthereumClient,
    pub noble_client: NobleClient,
    pub neutron_client: NeutronClient,

    // Ethereum and Neutron account & library addresses
    pub neutron_program_accounts: NeutronProgramAccounts,
    pub neutron_program_libraries: NeutronProgramLibraries,
    pub eth_program_accounts: EthereumProgramAccounts,
    pub eth_program_libraries: EthereumProgramLibraries,

    // underlying pool and its lp token addrress
    pub lp_token_denom: String,
    pub pool_addr: String,

    // deposit token information on Ethereum & Cosmos
    pub uusdc_on_neutron_denom: String,
    pub ethereum_usdc_erc20: Address,
}

impl Strategist {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rt: &Runtime,
        neutron_program_accounts: NeutronProgramAccounts,
        neutron_program_libraries: NeutronProgramLibraries,
        ethereum_program_accounts: EthereumProgramAccounts,
        ethereum_program_libraries: EthereumProgramLibraries,
        uusdc_on_neutron_denom: String,
        lp_token_denom: String,
        pool_addr: String,
        ethereum_usdc_erc20: Address,
    ) -> Result<Self, Box<dyn Error>> {
        // get neutron & noble grpc (url, port) values from local-ic logs
        let (noble_grpc_url, noble_grpc_port) = get_grpc_address_and_port_from_url(
            &get_chain_field_from_local_ic_log(NOBLE_CHAIN_ID, "grpc_address")?,
        )?;
        let (neutron_grpc_url, neutron_grpc_port) = get_grpc_address_and_port_from_url(
            &get_chain_field_from_local_ic_log(NEUTRON_CHAIN_ID, "grpc_address")?,
        )?;

        // build the noble client
        let noble_client = async_run!(rt, {
            NobleClient::new(
                &noble_grpc_url,
                &noble_grpc_port.to_string(),
                ADMIN_MNEMONIC,
                NOBLE_CHAIN_ID,
                NOBLE_CHAIN_DENOM,
            )
            .await
            .expect("failed to create noble client")
        });

        // build the neutron client
        let neutron_client = async_run!(rt, {
            NeutronClient::new(
                &neutron_grpc_url,
                &neutron_grpc_port.to_string(),
                ADMIN_MNEMONIC,
                NEUTRON_CHAIN_ID,
            )
            .await
            .expect("failed to create neutron client")
        });

        // build the eth client
        let eth_client = EthereumClient {
            rpc_url: DEFAULT_ANVIL_RPC_ENDPOINT.to_string(),
            signer: MnemonicBuilder::<English>::default()
                .phrase("test test test test test test test test test test test junk")
                .index(7)? // derive the mnemonic at a different index to avoid nonce issues
                .build()?,
        };

        Ok(Self {
            eth_client,
            noble_client,
            neutron_client,
            neutron_program_accounts,
            neutron_program_libraries,
            eth_program_libraries: ethereum_program_libraries,
            eth_program_accounts: ethereum_program_accounts,
            uusdc_on_neutron_denom,
            lp_token_denom,
            pool_addr,
            ethereum_usdc_erc20,
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

            // 1. calculate the amount of usdc needed to fulfill
            // the active withdraw obligations
            let pending_obligations = self.calculate_usdc_obligation().await.unwrap();

            // 2. query ethereum program accounts for their usdc balances
            let eth_deposit_acc_usdc_bal = self.deposit_acc_bal().await.unwrap();

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
                    &self.pool_addr,
                    NEUTRON_CHAIN_DENOM,
                    &self.uusdc_on_neutron_denom,
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
                    &self.pool_addr,
                    &self.uusdc_on_neutron_denom,
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
            // Update the Vault with R, F_total, N
            self.vault_update(r, total_fee, netting_amount)
                .await
                .unwrap();

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
            self.state_log().await;

            i += 1;
        }
    }

    async fn state_log(&self) {
        self.neutron_program_accounts
            .log_balances(
                &self.neutron_client,
                &self.noble_client,
                vec![
                    self.uusdc_on_neutron_denom.to_string(),
                    NEUTRON_CHAIN_DENOM.to_string(),
                    self.lp_token_denom.to_string(),
                ],
            )
            .await;
        self.eth_program_accounts
            .log_balances(
                &self.eth_client,
                &self.eth_program_libraries.valence_vault,
                &self.ethereum_usdc_erc20,
            )
            .await;
    }
}
