use std::{
    collections::HashMap,
    fmt,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use cosmos_grpc_client::{
    cosmos_sdk_proto::{
        cosmos::{bank::v1beta1::QueryBalanceRequest, base::v1beta1::Coin},
        cosmwasm::wasm::v1::{MsgInstantiateContract2, QueryCodeRequest},
    }, cosmrs::{bip32::secp256k1::sha2::{digest::Update, Digest, Sha256}, tx::MessageExt}, Decimal, GrpcClient, ProstMsgNameToAny, ProstMsgToAny, Wallet
};
use cosmwasm_std::{instantiate2_address, CanonicalAddr};
use serde_json::to_vec;

use crate::{
    account::{AccountType, InstantiateAccountData},
    config::ChainInfo,
    helpers::addr_humanize,
};

use super::Connector;

const MNEMONIC: &str = "crazy into this wheel interest enroll basket feed fashion leave feed depth wish throw rack language comic hand family shield toss leisure repair kite";

pub struct CosmosCosmwasmConnector {
    wallet: Wallet,
    code_ids: HashMap<String, u64>,
    _chain_name: String,
}

impl fmt::Debug for CosmosCosmwasmConnector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CosmosCosmwasmConnector")
            .field("wallet", &self.wallet)
            .finish_non_exhaustive()
    }
}

impl CosmosCosmwasmConnector {
    pub async fn new(chain_info: ChainInfo, code_ids: HashMap<String, u64>) -> Self {
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

        CosmosCosmwasmConnector {
            wallet,
            code_ids,
            _chain_name: chain_info.name,
        }
    }
}

#[async_trait]
impl Connector for CosmosCosmwasmConnector {
    async fn predict_address(
        &mut self,
        id: &u64,
        contract_name: &str,
        extra_salt: &str,
    ) -> (String, Vec<u8>) {
        // Get the creator address as canonical
        let creator: CanonicalAddr = self.wallet.account_address.as_bytes().into();
        
        // Get the checksum of the code id
        let req = QueryCodeRequest {
            code_id: *self.code_ids.get(contract_name).unwrap(),
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

        // TODO: generate a unique salt per workflow and per contract by adding timestamp
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis();

        let salt = Sha256::new()
            .chain(contract_name.to_string())
            .chain(id.to_string())
            .chain(extra_salt)
            .chain(since_the_epoch.to_string())
            .finalize();

        let addr_canon = instantiate2_address(&checksum, &creator, &salt).unwrap();
        let addr = addr_humanize(&self.wallet.prefix, &addr_canon);

        (addr, salt.to_vec())
    }

    async fn init_account(&mut self, data: &InstantiateAccountData) -> () {
        let code_id = *self
            .code_ids
            .get(&data.info.ty.to_string())
            .unwrap();

        // TODO: change the
        let msg: Vec<u8> = match &data.info.ty {
            AccountType::Base { admin } => {
                to_vec(&valence_base_account::msg::InstantiateMsg {
                    admin: admin.clone().unwrap_or_else(|| self.wallet.account_address.to_string()),
                    approved_services: data.approved_services.clone(),
                }).unwrap() 
            }
            AccountType::Addr { addr } => vec![]
        };

        let m = MsgInstantiateContract2 {
            sender: self.wallet.account_address.clone(),
            admin: self.wallet.account_address.clone(),
            code_id,
            label: format!("account-{}", data.id),
            msg,
            funds: vec![],
            salt: data.salt.clone(),
            fix_msg: false,
        }.build_any();

        let response = self
            .wallet
            .simulate_tx(vec![m])
            // .broadcast_tx(vec![msg], None, None, BroadcastMode::Sync)
            .await
            .unwrap();
        println!("{:#?}", response);
        panic!("stop");
    }

    async fn get_balance(&mut self, addr: String) -> Option<Coin> {
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
    }
    // Other method implementations...
}
