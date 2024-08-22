use std::{fmt, str::FromStr};

use cosmos_grpc_client::{
    cosmos_sdk_proto::cosmos::{bank::v1beta1::QueryBalanceRequest, base::v1beta1::Coin},
    Decimal, GrpcClient, Wallet,
};

use crate::{account::AccountType, config::ChainInfo};

use super::{Connector, PinnedFuture};

const MNEMONIC: &str = "crazy into this wheel interest enroll basket feed fashion leave feed depth wish throw rack language comic hand family shield toss leisure repair kite";

pub struct CosmosCwConnector {
    wallet: Wallet,
}

impl fmt::Debug for CosmosCwConnector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CosmosCwConnector")
            .field("wallet", &self.wallet)
            .finish_non_exhaustive()
    }
}

impl Connector for CosmosCwConnector {
    fn new(chain_info: ChainInfo) -> PinnedFuture<'static, Self> {
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

            CosmosCwConnector { wallet }
        })
    }

    fn init_account(&mut self, _account_type: &AccountType) -> PinnedFuture<String> {
        Box::pin(async move {
            // TODO: get code id from config
            // let init_msg = valence_base_account::msg::InstantiateMsg {
            //     admin: self.wallet.account_address.to_string(),
            // };

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
