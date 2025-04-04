use std::{error::Error, time::Duration};

use alloy::{
    primitives::{Address, U256},
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use cosmwasm_std::{to_json_binary, CosmosMsg, WasmMsg};
use localic_utils::{NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_ID};
use log::info;
use tokio::{runtime::Runtime, time::sleep};
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
        solidity_contracts::{CCTPTransfer, ValenceVault},
        ADMIN_MNEMONIC, DEFAULT_ANVIL_RPC_ENDPOINT, NOBLE_CHAIN_DENOM, NOBLE_CHAIN_ID, UUSDC_DENOM,
    },
};

use crate::program::{NeutronProgramAccounts, NeutronProgramLibraries};

pub struct Strategist {
    eth_client: EthereumClient,
    noble_client: NobleClient,
    neutron_client: NeutronClient,
    neutron_program_accounts: NeutronProgramAccounts,
    neutron_program_libraries: NeutronProgramLibraries,
    uusdc_on_neutron_denom: String,
    lp_token_denom: String,
    pool_addr: String,
    cctp_transfer_lib: Address,
    vault_addr: Address,
}

impl Strategist {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rt: &Runtime,
        neutron_program_accounts: NeutronProgramAccounts,
        neutron_program_libraries: NeutronProgramLibraries,
        uusdc_on_neutron_denom: String,
        lp_token_denom: String,
        pool_addr: String,
        cctp_transfer_lib: Address,
        vault_addr: Address,
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
            uusdc_on_neutron_denom,
            lp_token_denom,
            pool_addr,
            cctp_transfer_lib,
            vault_addr,
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

        for i in 1..10 {
            sleep(Duration::from_secs(3)).await;
            let post_bal = self
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

            if init_bal + transfer_amount == post_bal {
                info!(
                    "Funds successfully routed in from Noble inbound ICA to Neutron deposit acc!"
                );
                break;
            } else if i == 10 {
                panic!("Failed to route funds from Noble inbound ICA to Neutron deposit acc!");
            } else {
                info!("Funds in transit #{i}...")
            }
        }
    }

    /// IBC-transfers funds from Neutron withdraw account to noble outbound ica
    pub async fn route_neutron_to_noble(&self) {
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
        sleep(Duration::from_secs(3)).await;
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

        assert_ne!(
            withdraw_account_usdc_bal, 0,
            "withdraw account must have usdc in order to route funds to noble"
        );
        assert_ne!(
            withdraw_account_ntrn_bal, 0,
            "withdraw account must have ntrn in order to route funds to noble"
        );
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
        info!(
            "withdraw account USDC token balance: {:?}",
            withdraw_acc_usdc_bal
        );
        info!(
            "withdraw account NTRN token balance: {:?}",
            withdraw_acc_ntrn_bal
        );
        assert_eq!(0, withdraw_acc_usdc_bal);
    }

    /// CCTP-transfers funds from Ethereum deposit account to Noble inbound ica
    pub async fn route_eth_to_noble(&self) {
        info!("[STRATEGIST] CCTP forwarding USDC from Ethereum to Noble...");
        let eth_rp = self.eth_client.get_request_provider().await.unwrap();

        let cctp_transfer_contract = CCTPTransfer::new(self.cctp_transfer_lib, &eth_rp);

        let cctp_config = self
            .eth_client
            .query(cctp_transfer_contract.config())
            .await
            .unwrap();
        let cctp_processor = self
            .eth_client
            .query(cctp_transfer_contract.processor())
            .await
            .unwrap();
        let cctp_owner = self
            .eth_client
            .query(cctp_transfer_contract.owner())
            .await
            .unwrap();
        let signer_addr = self.eth_client.signer.address();

        info!("[route_eth_to_noble] cctp config: {:?}", cctp_config);
        info!(
            "[route_eth_to_noble] cctp processor: {:?}",
            cctp_processor._0
        );
        info!("[route_eth_to_noble] cctp owner: {:?}", cctp_owner._0);
        info!("[route_eth_to_noble] signer address: {:?}", signer_addr);

        let signed_tx = cctp_transfer_contract
            .transfer()
            .into_transaction_request()
            .from(signer_addr);

        let cctp_transfer_rx = self.eth_client.execute_tx(signed_tx).await.unwrap();

        info!(
            "cctp transfer tx hash: {:?}",
            cctp_transfer_rx.transaction_hash
        );
    }

    /// CCTP-transfers funds from Noble outbound ica to Ethereum withdraw account
    pub async fn route_noble_to_eth(&self) {
        info!("[STRATEGIST] CCTP forwarding USDC from Noble to Ethereum...");
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
        sleep(Duration::from_secs(3)).await;

        let noble_outbound_acc_usdc_bal = self
            .noble_client
            .query_balance(
                &self.neutron_program_accounts.noble_outbound_ica.remote_addr,
                UUSDC_DENOM,
            )
            .await
            .unwrap();

        assert_ne!(
            noble_outbound_acc_usdc_bal, 0,
            "Noble outbound ICA account must have usdc in order to initiate CCTP forwarding"
        );

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

        sleep(Duration::from_secs(10)).await;

        let noble_outbound_acc_usdc_bal = self
            .noble_client
            .query_balance(
                &self.neutron_program_accounts.noble_outbound_ica.remote_addr,
                UUSDC_DENOM,
            )
            .await
            .unwrap();

        info!(
            "Noble outbound ICA account balance post cctp transfer: {:?}",
            noble_outbound_acc_usdc_bal
        );
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

        assert_ne!(
            deposit_account_usdc_bal, 0,
            "deposit account must have uusdc in order to lp"
        );

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
        info!("position account LP token balance: {:?}", output_acc_bal);
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

        assert_ne!(
            position_account_shares_bal, 0,
            "position account must have shares in order to exit lp"
        );

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
        info!(
            "withdraw account USDC token balance: {:?}",
            withdraw_acc_usdc_bal
        );
        info!(
            "withdraw account NTRN token balance: {:?}",
            withdraw_acc_ntrn_bal
        );
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

        assert_ne!(
            withdraw_account_ntrn_bal, 0,
            "withdraw account must have NTRN in order to swap into USDC"
        );

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
        info!(
            "withdraw account USDC token balance: {:?}",
            withdraw_acc_usdc_bal
        );
        info!(
            "withdraw account NTRN token balance: {:?}",
            withdraw_acc_ntrn_bal
        );
        assert_ne!(0, withdraw_acc_usdc_bal);
        assert_eq!(0, withdraw_acc_ntrn_bal);
    }
}
