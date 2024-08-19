use std::{fmt, str::FromStr};

use cosmos_grpc_client::{
    cosmos_sdk_proto::cosmos::{bank::v1beta1::QueryBalanceRequest, base::v1beta1::Coin},
    CoinType, Decimal, GrpcClient, StdError, Wallet,
};

use super::{Connector, PinnedFuture};

pub struct CosmosConnector {
    // rpc: String,
    // wallet: Wallet,
    // client: GrpcClient,
    wallet: Wallet,
    client: GrpcClient,
}

impl fmt::Debug for CosmosConnector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CosmosConnector")
            .field("wallet", &self.wallet)
            .finish_non_exhaustive()
    }
}

impl Connector for CosmosConnector {
    fn connect(&self) -> Result<(), StdError> {
        // Implementation here
        // println!("Connecting to Cosmos domain with rpc: {}", self.rpc);
        Ok(())
    }

    fn get_balance(&mut self, addr: String) -> PinnedFuture<Option<Coin>> {
        Box::pin(async move {
            let request = QueryBalanceRequest {
                address: addr,
                denom: "untrn".to_string(),
            };
            let response = self.client.clients.bank.balance(request).await.unwrap().into_inner();
            response.balance.clone()
        })
    }

    fn new(endpoint: String, wallet_mnemonic: String) -> PinnedFuture<'static, Self> {
        Box::pin(async move {
            let grpc = GrpcClient::new(&endpoint).await.unwrap();
            let grpc_clone = grpc.clone();
            let wallet = Wallet::from_seed_phrase(
                grpc_clone,
                String::from_utf8(wallet_mnemonic.into()).unwrap(),
                "cosmos",
                CoinType::Cosmos,
                0,
                Decimal::from_str("0.0025").unwrap(),
                Decimal::from_str("1.5").unwrap(),
                "cosmos",
            )
            .await
            .unwrap();

            CosmosConnector {
                wallet,
                client: grpc,
            }
        })
    }
    // Other method implementations...
}
