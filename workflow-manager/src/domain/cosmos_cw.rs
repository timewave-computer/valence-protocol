use std::{collections::HashMap, fmt, str::FromStr};

use async_trait::async_trait;
use cosmos_grpc_client::{
    cosmos_sdk_proto::{
        cosmos::{bank::v1beta1::QueryBalanceRequest, base::v1beta1::Coin},
        cosmwasm::wasm::v1::{MsgInstantiateContract2, QueryCodeRequest},
    },
    cosmrs::bip32::secp256k1::sha2::{digest::Update, Digest, Sha256},
    Decimal, GrpcClient, Wallet,
};
use cosmwasm_std::{instantiate2_address, CanonicalAddr};

use crate::{account::AccountType, config::{Cfg, ChainInfo}};

use super::{Connector, PinnedFuture};

const MNEMONIC: &str = "crazy into this wheel interest enroll basket feed fashion leave feed depth wish throw rack language comic hand family shield toss leisure repair kite";

pub struct CosmosCwConnector {
    wallet: Wallet,
    code_ids: HashMap<String, u64>,
    chain_name: String,
}

impl fmt::Debug for CosmosCwConnector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CosmosCwConnector")
            .field("wallet", &self.wallet)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl Connector for CosmosCwConnector {
    fn new(chain_info: ChainInfo, code_ids: HashMap<String, u64>) -> PinnedFuture<'static, Self> {
        Box::pin(async move {
            let grpc = GrpcClient::new(&chain_info.grpc).await.unwrap();

            let wallet = Wallet::from_seed_phrase(
                grpc,
                MNEMONIC,
                chain_info.prefix,
                chain_info.coin_type,
                0,
                Decimal::from_str(&chain_info.gas_price).unwrap(),
                Decimal::from_str("1.5").unwrap(),
                &chain_info.gas_denom,
            )
            .await
            .unwrap();

            CosmosCwConnector {
                wallet,
                chain_name: chain_info.name,
                code_ids,
            }
        })
    }

    fn get_account_addr(
        &self,
        cfg: &Cfg,
        account_id: u64,
        account_type: AccountType,
    ) -> PinnedFuture<String> {
        Box::pin(async move {
            // Get the creator address as canonical
            let creator: CanonicalAddr = self.wallet.account_address.as_bytes().into();

            // Get the checksum of the code id
            let req = QueryCodeRequest {
                code_id: self
                    .code_ids
                    .get(&account_type.to_string())
                    .unwrap()
                    .clone(),
            };
            let checksum = self
                .wallet
                .client
                .clients
                .wasm
                .code(req)
                .await
                .unwrap()
                .get_ref()
                .code_info
                .clone()
                .unwrap()
                .data_hash
                .clone();

            println!("{:?}", checksum);
            let code_id = cfg.contracts.code_ids.get(&self.chain_name.to_string());

            // TODO: generate a unique salt per workflow and per contract
            let salt = Sha256::new().chain(account_id.to_string()).finalize();

            instantiate2_address(&checksum, &creator, &salt)
                .unwrap()
                .to_string()
        })
    }

    fn init_account(&mut self, _account_type: &AccountType) -> PinnedFuture<String> {
        Box::pin(async move {
            // TODO: get code id from config
            // TODO: Get init message
            // let init_msg = valence_base_account::msg::InstantiateMsg {
            //     admin: self.wallet.account_address.to_string(),
            // };

            // Should be enough because we know the address is correct.

            MsgInstantiateContract2 {
                sender: todo!(),
                admin: todo!(),
                code_id: todo!(),
                label: todo!(),
                msg: todo!(),
                funds: todo!(),
                salt: todo!(),
                fix_msg: todo!(),
            };
            // let msg = MsgInstantiateContract {
            //     sender: self.wallet.account_address.to_string(),
            //     code_id: 5987,
            //     msg: to_vec(&init_msg).unwrap(),
            //     funds: vec![],
            //     label: "base_account".to_string(),
            //     admin: self.wallet.account_address.clone(),
            // }
            // .build_any();
            // let response = self
            //     .wallet
            //     .broadcast_tx(vec![msg], None, None, BroadcastMode::Sync)
            //     .await
            //     .unwrap()
            //     .tx_response;
            // println!("{:?}", response);
            self.wallet.chain_id.clone()
        })
    }

    fn get_balance(&mut self, addr: String) -> PinnedFuture<Option<Coin>> {
        Box::pin(async move {
            let request = QueryBalanceRequest {
                address: addr,
                denom: "untrn".to_string(),
            };
            let response = self
                .wallet
                .client
                .clients
                .bank
                .balance(request)
                .await
                .unwrap()
                .into_inner();
            response.balance.clone()
        })
    }

    // Other method implementations...
}

/// Private methods only need for cosmos
impl CosmosCwConnector {}
