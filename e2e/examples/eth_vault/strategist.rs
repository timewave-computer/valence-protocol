use std::error::Error;

use alloy::{
    primitives::{Address, U256},
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use cosmwasm_std::{to_json_binary, CosmosMsg, WasmMsg};
use localic_utils::{NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_ID};
use log::{info, warn};
use tokio::runtime::Runtime;
use valence_astroport_utils::astroport_native_lp_token::{Asset, AssetInfo};
use valence_chain_client_utils::{
    cosmos::{base_client::BaseClient, wasm_client::WasmClient},
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
    neutron::NeutronClient,
    noble::NobleClient,
};

use valence_e2e::{
    async_run,
    utils::{
        parse::{get_chain_field_from_local_ic_log, get_grpc_address_and_port_from_url},
        solidity_contracts::{CCTPTransfer, MockERC20, ValenceVault},
        ADMIN_MNEMONIC, DEFAULT_ANVIL_RPC_ENDPOINT, NOBLE_CHAIN_DENOM, NOBLE_CHAIN_ID, UUSDC_DENOM,
    },
};

use crate::{
    evm::EthereumProgramAccounts,
    program::{NeutronProgramAccounts, NeutronProgramLibraries},
};

pub struct Strategist {
    eth_client: EthereumClient,
    noble_client: NobleClient,
    neutron_client: NeutronClient,
    neutron_program_accounts: NeutronProgramAccounts,
    neutron_program_libraries: NeutronProgramLibraries,
    eth_program_accounts: EthereumProgramAccounts,
    uusdc_on_neutron_denom: String,
    lp_token_denom: String,
    pool_addr: String,
    cctp_transfer_lib: Address,
    vault_addr: Address,
    ethereum_usdc_erc20: Address,
}

impl Strategist {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rt: &Runtime,
        neutron_program_accounts: NeutronProgramAccounts,
        neutron_program_libraries: NeutronProgramLibraries,
        ethereum_program_accounts: EthereumProgramAccounts,
        uusdc_on_neutron_denom: String,
        lp_token_denom: String,
        pool_addr: String,
        cctp_transfer_lib: Address,
        vault_addr: Address,
        ethereum_usdc_erc20: Address,
    ) -> Result<Self, Box<dyn Error>> {
        let noble_grpc_addr = get_chain_field_from_local_ic_log(NOBLE_CHAIN_ID, "grpc_address")?;
        let (noble_grpc_url, noble_grpc_port) =
            get_grpc_address_and_port_from_url(&noble_grpc_addr)?;

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

        let neutron_grpc_addr =
            get_chain_field_from_local_ic_log(NEUTRON_CHAIN_ID, "grpc_address")?;
        let (neutron_grpc_url, neutron_grpc_port) =
            get_grpc_address_and_port_from_url(&neutron_grpc_addr)?;

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

        let signer = MnemonicBuilder::<English>::default()
            .phrase("test test test test test test test test test test test junk")
            .index(7)? // derive the mnemonic at a different index to avoid nonce issues
            .build()?;

        let eth_client = EthereumClient {
            rpc_url: DEFAULT_ANVIL_RPC_ENDPOINT.to_string(),
            signer,
        };

        Ok(Self {
            eth_client,
            noble_client,
            neutron_client,
            neutron_program_accounts,
            neutron_program_libraries,
            eth_program_accounts: ethereum_program_accounts,
            uusdc_on_neutron_denom,
            lp_token_denom,
            pool_addr,
            cctp_transfer_lib,
            vault_addr,
            ethereum_usdc_erc20,
        })
    }
}

impl Strategist {
    pub async fn _start(self) {
        info!("[STRATEGIST] Starting...");

        loop {
            info!("[STRATEGIST] loop");
            // TODO
        }
    }

    /// concludes the vault epoch and updates the Valence Vault state
    pub async fn vault_update(
        &self,
        rate: U256,
        withdraw_fee_bps: u32,
        netting_amount: U256,
    ) -> Result<(), Box<dyn Error>> {
        info!("[STRATEGIST] Updating Ethereum Vault...");
        let eth_rp = self.eth_client.get_request_provider().await.unwrap();

        let valence_vault = ValenceVault::new(self.vault_addr, &eth_rp);

        let update_msg = valence_vault
            .update(rate, withdraw_fee_bps, netting_amount)
            .into_transaction_request();

        let update_result = self.eth_client.execute_tx(update_msg).await;

        if let Err(e) = &update_result {
            info!("Update failed: {:?}", e);
            panic!("failed to update vault");
        }

        Ok(())
    }

    /// IBC-transfers funds from noble inbound ica into neutron deposit account
    pub async fn route_noble_to_neutron(&self, transfer_amount: u128) {
        let noble_inbound_ica_balance = self
            .noble_client
            .query_balance(
                &self.neutron_program_accounts.noble_inbound_ica.remote_addr,
                UUSDC_DENOM,
            )
            .await
            .unwrap();

        if noble_inbound_ica_balance < transfer_amount {
            warn!("Noble inbound ICA account must have enough USDC to route funds to Neutron deposit acc; returning");
            return;
        }

        let init_bal = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .deposit_account
                    .to_string()
                    .unwrap(),
                &self.uusdc_on_neutron_denom,
            )
            .await
            .unwrap();

        let transfer_msg = &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
            valence_ica_ibc_transfer::msg::FunctionMsgs::Transfer {},
        );
        let rx = self
            .neutron_client
            .execute_wasm(
                &self.neutron_program_libraries.noble_inbound_transfer,
                transfer_msg,
                vec![],
            )
            .await
            .unwrap();
        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        info!("starting polling assertion on the destination...");
        self.neutron_client
            .poll_until_expected_balance(
                &self
                    .neutron_program_accounts
                    .deposit_account
                    .to_string()
                    .unwrap(),
                &self.uusdc_on_neutron_denom,
                init_bal + transfer_amount,
                1,
                10,
            )
            .await
            .unwrap();
    }

    /// IBC-transfers funds from Neutron withdraw account to noble outbound ica
    pub async fn route_neutron_to_noble(&self) {
        let noble_outbound_ica_usdc_bal = self
            .noble_client
            .query_balance(
                &self.neutron_program_accounts.noble_outbound_ica.remote_addr,
                UUSDC_DENOM,
            )
            .await
            .unwrap();
        let withdraw_account_usdc_bal = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                &self.uusdc_on_neutron_denom,
            )
            .await
            .unwrap();

        let withdraw_account_ntrn_bal = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                NEUTRON_CHAIN_DENOM,
            )
            .await
            .unwrap();

        if withdraw_account_usdc_bal == 0 {
            warn!("[STRATEGIST] withdraw account must have USDC in order to route funds to noble; returning");
            return;
        }

        if withdraw_account_ntrn_bal == 0 {
            warn!("[STRATEGIST] withdraw account must have NTRN in order to route funds to noble; returning");
            return;
        }

        info!("[STRATEGIST] routing USDC to noble...");
        let transfer_rx = self
            .neutron_client
            .transfer(
                &self
                    .neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                110_000,
                NEUTRON_CHAIN_DENOM,
                None,
            )
            .await
            .unwrap();
        self.neutron_client
            .poll_for_tx(&transfer_rx.hash)
            .await
            .unwrap();

        let neutron_ibc_transfer_msg =
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_neutron_ibc_transfer_library::msg::FunctionMsgs::IbcTransfer {},
            );
        let rx = self
            .neutron_client
            .execute_wasm(
                &self.neutron_program_libraries.neutron_ibc_transfer,
                neutron_ibc_transfer_msg,
                vec![],
            )
            .await
            .unwrap();
        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        info!("starting polling assertion on noble outbound ica...");
        self.noble_client
            .poll_until_expected_balance(
                &self
                    .neutron_program_accounts
                    .noble_outbound_ica
                    .remote_addr
                    .to_string(),
                UUSDC_DENOM,
                noble_outbound_ica_usdc_bal + withdraw_account_usdc_bal,
                1,
                10,
            )
            .await
            .unwrap();
    }

    /// CCTP-transfers funds from Ethereum deposit account to Noble inbound ica
    pub async fn route_eth_to_noble(&self) {
        info!("[STRATEGIST] CCTP forwarding USDC from Ethereum to Noble...");
        let eth_rp = self.eth_client.get_request_provider().await.unwrap();

        let cctp_transfer_contract = CCTPTransfer::new(self.cctp_transfer_lib, &eth_rp);

        let pre_cctp_inbound_ica_usdc_bal = self
            .noble_client
            .query_balance(
                &self.neutron_program_accounts.noble_inbound_ica.remote_addr,
                UUSDC_DENOM,
            )
            .await
            .unwrap();

        let signer_addr = self.eth_client.signer.address();
        let signed_tx = cctp_transfer_contract
            .transfer()
            .into_transaction_request()
            .from(signer_addr);

        let cctp_transfer_rx = self.eth_client.execute_tx(signed_tx).await.unwrap();

        info!(
            "cctp transfer tx hash: {:?}",
            cctp_transfer_rx.transaction_hash
        );

        let remote_ica_addr = self
            .neutron_program_accounts
            .noble_inbound_ica
            .remote_addr
            .to_string();

        info!("starting polling assertion on the destination...");
        self.noble_client
            .poll_until_expected_balance(
                &remote_ica_addr,
                UUSDC_DENOM,
                pre_cctp_inbound_ica_usdc_bal + 1,
                1,
                10,
            )
            .await
            .unwrap();
    }

    /// CCTP-transfers funds from Noble outbound ica to Ethereum withdraw account
    pub async fn route_noble_to_eth(&self) {
        info!("[STRATEGIST] CCTP forwarding USDC from Noble to Ethereum...");

        let eth_rp = self.eth_client.get_request_provider().await.unwrap();
        let erc20 = MockERC20::new(self.ethereum_usdc_erc20, &eth_rp);
        let pre_cctp_ethereum_withdraw_acc_usdc_bal = self
            .eth_client
            .query(erc20.balanceOf(self.eth_program_accounts.withdraw))
            .await
            .unwrap()
            ._0;
        let pre_cctp_noble_outbound_ica_usdc_bal = self
            .noble_client
            .query_balance(
                &self.neutron_program_accounts.noble_outbound_ica.remote_addr,
                UUSDC_DENOM,
            )
            .await
            .unwrap();

        if pre_cctp_noble_outbound_ica_usdc_bal == 0 {
            warn!("[STRATEGIST] Noble outbound ICA account must have USDC in order to CCTP forward to Ethereum; returning");
            return;
        }

        let transfer_tx = self
            .neutron_client
            .transfer(
                &self
                    .neutron_program_accounts
                    .noble_outbound_ica
                    .library_account
                    .to_string()
                    .unwrap(),
                110_000,
                NEUTRON_CHAIN_DENOM,
                None,
            )
            .await
            .unwrap();
        self.neutron_client
            .poll_for_tx(&transfer_tx.hash)
            .await
            .unwrap();

        let neutron_ica_cctp_transfer_msg =
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_ica_cctp_transfer::msg::FunctionMsgs::Transfer {},
            );
        let rx = self
            .neutron_client
            .execute_wasm(
                &self.neutron_program_libraries.noble_cctp_transfer,
                neutron_ica_cctp_transfer_msg,
                vec![],
            )
            .await
            .unwrap();
        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        self.blocking_erc20_expected_balance_query(
            self.eth_program_accounts.withdraw,
            pre_cctp_ethereum_withdraw_acc_usdc_bal + U256::from(1),
            1,
            10,
        )
        .await;
    }

    async fn blocking_erc20_expected_balance_query(
        &self,
        addr: Address,
        min_amount: U256,
        interval_sec: u64,
        max_attempts: u32,
    ) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_sec));
        let eth_rp = self.eth_client.get_request_provider().await.unwrap();

        info!("EVM polling {addr} balance to exceed {min_amount}");

        let erc20 = MockERC20::new(self.ethereum_usdc_erc20, &eth_rp);

        for attempt in 1..max_attempts + 1 {
            interval.tick().await;

            match self.eth_client.query(erc20.balanceOf(addr)).await {
                Ok(balance) => {
                    let bal = balance._0;
                    if bal >= min_amount {
                        info!("balance exceeded!");
                        return;
                    } else {
                        info!(
                            "Balance polling attempt {attempt}/{max_attempts}: current={bal}, target={min_amount}"
                        );
                    }
                }
                Err(e) => warn!(
                    "Balance polling attempt {attempt}/{max_attempts} failed: {:?}",
                    e
                ),
            }
        }
    }

    /// enters the position on astroport
    pub async fn enter_position(&self) {
        info!("[STRATEGIST] entering LP position...");
        let deposit_account_usdc_bal = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .deposit_account
                    .to_string()
                    .unwrap(),
                &self.uusdc_on_neutron_denom,
            )
            .await
            .unwrap();

        if deposit_account_usdc_bal == 0 {
            warn!("[STRATEGIST] Deposit account must have USDC in order to LP; returning");
            return;
        }

        let provide_liquidity_msg =
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_astroport_lper::msg::FunctionMsgs::ProvideSingleSidedLiquidity {
                    asset: self.uusdc_on_neutron_denom.to_string(),
                    limit: None,
                    expected_pool_ratio_range: None,
                },
            );
        let rx = self
            .neutron_client
            .execute_wasm(
                &self.neutron_program_libraries.astroport_lper,
                provide_liquidity_msg,
                vec![],
            )
            .await
            .unwrap();
        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        let output_acc_bal = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .position_account
                    .to_string()
                    .unwrap(),
                &self.lp_token_denom,
            )
            .await
            .unwrap();
        info!("position account LP token balance: {output_acc_bal}");
        assert_ne!(0, output_acc_bal);
    }

    /// exits the position on astroport
    pub async fn exit_position(&self) {
        info!("[STRATEGIST] exiting LP position...");

        let position_account_shares_bal = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .position_account
                    .to_string()
                    .unwrap(),
                &self.lp_token_denom,
            )
            .await
            .unwrap();

        if position_account_shares_bal == 0 {
            warn!(
                "[STRATEGIST] Position account must have LP shares in order to exit LP; returning"
            );
            return;
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
                &self.neutron_program_libraries.astroport_lwer,
                withdraw_liquidity_msg,
                vec![],
            )
            .await
            .unwrap();
        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        let withdraw_acc_usdc_bal = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                &self.uusdc_on_neutron_denom,
            )
            .await
            .unwrap();
        let withdraw_acc_ntrn_bal = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                NEUTRON_CHAIN_DENOM,
            )
            .await
            .unwrap();
        info!("withdraw account USDC token balance: {withdraw_acc_usdc_bal}",);
        info!("withdraw account NTRN token balance: {withdraw_acc_ntrn_bal}",);
        assert_ne!(0, withdraw_acc_usdc_bal);
        assert_ne!(0, withdraw_acc_ntrn_bal);
    }

    /// swaps counterparty denom into usdc
    pub async fn swap_ntrn_into_usdc(&self) {
        info!("[STRATEGIST] swapping NTRN into USDC...");
        let withdraw_account_ntrn_bal = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                NEUTRON_CHAIN_DENOM,
            )
            .await
            .unwrap();

        if withdraw_account_ntrn_bal == 0 {
            warn!("[STRATEGIST] Withdraw account must have NTRN in order to swap into USDC; returning");
            return;
        }

        let swap_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.pool_addr.to_string(),
            msg: to_json_binary(
                &valence_astroport_utils::astroport_native_lp_token::ExecuteMsg::Swap {
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
                },
            )
            .unwrap(),
            funds: vec![cosmwasm_std::coin(
                withdraw_account_ntrn_bal,
                NEUTRON_CHAIN_DENOM.to_string(),
            )],
        });

        let base_account_execute_msgs = valence_account_utils::msg::ExecuteMsg::ExecuteMsg {
            msgs: vec![swap_msg],
        };

        let rx = self
            .neutron_client
            .execute_wasm(
                &self
                    .neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                base_account_execute_msgs,
                vec![],
            )
            .await
            .unwrap();

        self.neutron_client.poll_for_tx(&rx.hash).await.unwrap();

        let withdraw_acc_usdc_bal = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                &self.uusdc_on_neutron_denom,
            )
            .await
            .unwrap();
        let withdraw_acc_ntrn_bal = self
            .neutron_client
            .query_balance(
                &self
                    .neutron_program_accounts
                    .withdraw_account
                    .to_string()
                    .unwrap(),
                NEUTRON_CHAIN_DENOM,
            )
            .await
            .unwrap();
        info!("withdraw account USDC token balance: {withdraw_acc_usdc_bal}",);
        info!("withdraw account NTRN token balance: {withdraw_acc_ntrn_bal}",);
        assert_ne!(0, withdraw_acc_usdc_bal);
        assert_eq!(0, withdraw_acc_ntrn_bal);
    }
}
