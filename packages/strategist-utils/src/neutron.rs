use crate::{
    common::{error::StrategistError, transaction::TransactionResponse},
    cosmos::{base_client::BaseClient, grpc_client::GrpcSigningClient, wasm_client::WasmClient},
};
use async_trait::async_trait;
use localic_utils::NEUTRON_CHAIN_DENOM;

const CHAIN_PREFIX: &str = "neutron";

pub struct NeutronClient {
    grpc_url: String,
    mnemonic: String,
    chain_id: String,
    chain_denom: String,
}

impl NeutronClient {
    pub fn new(
        rpc_url: &str,
        rpc_port: &str,
        mnemonic: &str,
        chain_id: &str,
        chain_denom: &str,
    ) -> Self {
        Self {
            grpc_url: format!("{rpc_url}:{rpc_port}"),
            mnemonic: mnemonic.to_string(),
            chain_id: chain_id.to_string(),
            chain_denom: chain_denom.to_string(),
        }
    }
}

#[async_trait]
impl BaseClient for NeutronClient {
    /// neutron has custom ibc logic so we override the default BaseClient ibc_transfer
    async fn ibc_transfer(
        &self,
        to: String,
        denom: String,
        amount: String,
        channel_id: String,
        timeout_seconds: u64,
    ) -> Result<TransactionResponse, StrategistError> {
        let latest_block_header = self.latest_block_header().await?;

        let signing_client = self.get_signing_client().await?;

        let current_time = latest_block_header.time.unwrap();

        let current_seconds = current_time.seconds as u64;
        let timeout_seconds = current_seconds + timeout_seconds;
        let timeout_nanos = (timeout_seconds * 1_000_000_000) + current_time.nanos as u64;

        let ibc_transfer_msg = neutron_std::types::ibc::applications::transfer::v1::MsgTransfer {
            source_port: "transfer".to_string(),
            source_channel: channel_id.to_string(),
            token: Some(neutron_std::types::cosmos::base::v1beta1::Coin {
                denom: denom.to_string(),
                amount,
            }),
            sender: signing_client.address.to_string(),
            receiver: to.to_string(),
            timeout_height: None,
            timeout_timestamp: timeout_nanos,
            memo: "hi".to_string(),
        }
        .to_any();

        let valid_any = cosmrs::Any {
            type_url: ibc_transfer_msg.type_url,
            value: ibc_transfer_msg.value,
        };

        let raw_tx = signing_client
            .create_tx(valid_any, NEUTRON_CHAIN_DENOM, 200_000, 500_000u64, None)
            .await
            .unwrap();

        let channel = self.get_grpc_channel().await?;

        let mut grpc_client =
            cosmos_sdk_proto::cosmos::tx::v1beta1::service_client::ServiceClient::new(channel);

        let broadcast_tx_response = grpc_client.broadcast_tx(raw_tx).await?.into_inner();

        match broadcast_tx_response.tx_response {
            Some(tx_response) => Ok(TransactionResponse::try_from(tx_response)?),
            None => Err(StrategistError::TransactionError("failed".to_string())),
        }
    }
}

#[async_trait]
impl WasmClient for NeutronClient {}

#[async_trait]
impl GrpcSigningClient for NeutronClient {
    fn grpc_url(&self) -> String {
        self.grpc_url.to_string()
    }

    fn mnemonic(&self) -> String {
        self.mnemonic.to_string()
    }

    fn chain_prefix(&self) -> String {
        CHAIN_PREFIX.to_string()
    }

    fn chain_id(&self) -> String {
        self.chain_id.to_string()
    }

    fn chain_denom(&self) -> String {
        self.chain_denom.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use localic_utils::{NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM, OSMOSIS_CHAIN_ADMIN_ADDR};

    use crate::osmosis::OsmosisClient;

    use super::*;

    const LOCAL_GRPC_URL: &str = "http://127.0.0.1";
    const LOCAL_GRPC_PORT: &str = "40571";
    const LOCAL_MNEMONIC: &str = "decorate bright ozone fork gallery riot bus exhaust worth way bone indoor calm squirrel merry zero scheme cotton until shop any excess stage laundry";
    const LOCAL_SIGNER_ADDR: &str = "neutron1hj5fveer5cjtn4wd6wstzugjfdxzl0xpznmsky";
    const LOCAL_ALT_ADDR: &str = "neutron1kljf09rj77uxeu5lye7muejx6ajsu55cuw2mws";
    const LOCAL_CHAIN_ID: &str = "localneutron-1";
    const LOCAL_PROCESSOR_ADDR: &str =
        "neutron12p7twsmksqw8lhj98hlxld7hxfl3tmwn6853ggtsalzm2ryx7ylsrmdfr6";

    const NEUTRON_ON_OSMO: &str =
        "ibc/4E41ED8F3DCAEA15F4D6ADC6EDD7C04A676160735C9710B904B7BF53525B56D6";

    // update during dev to a real one for mainnet testing
    const _CHAIN_ID: &str = "neutron-1";
    const _GRPC_URL: &str = "-";
    const _GRPC_PORT: &str = "-";
    const _NEUTRON_DAO_ADDR: &str =
        "neutron1suhgf5svhu4usrurvxzlgn54ksxmn8gljarjtxqnapv8kjnp4nrstdxvff";
    const _MNEMONIC: &str = "-";

    #[tokio::test]
    async fn test_latest_block_height() {
        let client = NeutronClient::new(
            LOCAL_GRPC_URL,
            LOCAL_GRPC_PORT,
            LOCAL_MNEMONIC,
            LOCAL_CHAIN_ID,
            NEUTRON_CHAIN_DENOM,
        );

        let block_height = client
            .latest_block_header()
            .await
            .expect("Failed to get latest block height")
            .height;

        println!("block height: {block_height}");
        assert!(block_height > 0);
    }

    #[tokio::test]
    async fn test_query_balance() {
        let client = NeutronClient::new(
            LOCAL_GRPC_URL,
            LOCAL_GRPC_PORT,
            LOCAL_MNEMONIC,
            LOCAL_CHAIN_ID,
            NEUTRON_CHAIN_DENOM,
        );
        let balance = client
            .query_balance(LOCAL_SIGNER_ADDR, "untrn")
            .await
            .unwrap();

        assert!(balance > 0);
    }

    #[tokio::test]
    async fn test_query_contract_state() {
        let client = NeutronClient::new(
            LOCAL_GRPC_URL,
            LOCAL_GRPC_PORT,
            LOCAL_MNEMONIC,
            LOCAL_CHAIN_ID,
            NEUTRON_CHAIN_DENOM,
        );

        let query = valence_processor_utils::msg::QueryMsg::Config {};

        let state: valence_processor_utils::processor::Config = client
            .query_contract_state(LOCAL_PROCESSOR_ADDR, query)
            .await
            .unwrap();

        assert_eq!(
            state.state,
            valence_processor_utils::processor::State::Active
        );
    }

    #[tokio::test]
    async fn test_transfer() {
        let client = NeutronClient::new(
            LOCAL_GRPC_URL,
            LOCAL_GRPC_PORT,
            LOCAL_MNEMONIC,
            LOCAL_CHAIN_ID,
            NEUTRON_CHAIN_DENOM,
        );

        let pre_transfer_balance = client
            .query_balance(LOCAL_ALT_ADDR, NEUTRON_CHAIN_DENOM)
            .await
            .unwrap();

        let rx = client
            .transfer(LOCAL_ALT_ADDR, 100_000, NEUTRON_CHAIN_DENOM)
            .await
            .unwrap();

        client.poll_for_tx(&rx.hash).await.unwrap();

        let post_transfer_balance = client
            .query_balance(LOCAL_ALT_ADDR, NEUTRON_CHAIN_DENOM)
            .await
            .unwrap();

        assert_eq!(pre_transfer_balance + 100_000, post_transfer_balance);
    }

    #[tokio::test]
    async fn test_execute_wasm() {
        let client = NeutronClient::new(
            LOCAL_GRPC_URL,
            LOCAL_GRPC_PORT,
            LOCAL_MNEMONIC,
            LOCAL_CHAIN_ID,
            NEUTRON_CHAIN_DENOM,
        );

        let processor_tick_msg = valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
            valence_processor_utils::msg::PermissionlessMsg::Tick {},
        );

        let rx = client
            .execute_wasm(LOCAL_PROCESSOR_ADDR, processor_tick_msg, vec![])
            .await
            .unwrap();

        let response = client.poll_for_tx(&rx.hash).await.unwrap();

        assert!(response.height > 0);
    }

    #[tokio::test]
    async fn test_ibc_transfer() {
        let client = NeutronClient::new(
            LOCAL_GRPC_URL,
            LOCAL_GRPC_PORT,
            LOCAL_MNEMONIC,
            LOCAL_CHAIN_ID,
            NEUTRON_CHAIN_DENOM,
        );

        let osmosis_client = OsmosisClient::new(
            LOCAL_GRPC_URL,
            "43093",
            LOCAL_MNEMONIC,
            "localosmosis-1",
            "uosmo",
        );

        let osmo_balance_0 = osmosis_client
            .query_balance(OSMOSIS_CHAIN_ADMIN_ADDR, NEUTRON_ON_OSMO)
            .await
            .unwrap();
        println!("osmo_balance_0: {osmo_balance_0}");

        let tx_response = client
            .ibc_transfer(
                OSMOSIS_CHAIN_ADMIN_ADDR.to_string(),
                "untrn".to_string(),
                "100000".to_string(),
                "channel-0".to_string(),
                5,
            )
            .await
            .unwrap();

        client.poll_for_tx(&tx_response.hash).await.unwrap();

        tokio::time::sleep(Duration::from_secs(5)).await;

        let osmo_balance_1 = osmosis_client
            .query_balance(OSMOSIS_CHAIN_ADMIN_ADDR, NEUTRON_ON_OSMO)
            .await
            .unwrap();
        println!("osmo_balance_1: {osmo_balance_1}");

        // assert that first transfer worked
        assert_eq!(osmo_balance_0 + 100_000, osmo_balance_1);

        let osmo_rx = osmosis_client
            .ibc_transfer(
                NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
                NEUTRON_ON_OSMO.to_string(),
                "100000".to_string(),
                "channel-0".to_string(),
                5,
            )
            .await
            .unwrap();

        osmosis_client.poll_for_tx(&osmo_rx.hash).await.unwrap();

        tokio::time::sleep(Duration::from_secs(5)).await;

        let osmo_balance_2 = osmosis_client
            .query_balance(OSMOSIS_CHAIN_ADMIN_ADDR, NEUTRON_ON_OSMO)
            .await
            .unwrap();
        println!("osmo_balance_2: {osmo_balance_2}");

        // assert that the second transfer worked
        assert_eq!(osmo_balance_0, osmo_balance_2);
    }
}
