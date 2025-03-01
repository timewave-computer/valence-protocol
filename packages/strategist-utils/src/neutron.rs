use crate::cosmos::{
    base_client::BaseClient, grpc_client::GrpcSigningClient, wasm_client::WasmClient,
};
use async_trait::async_trait;

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
impl BaseClient for NeutronClient {}

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

    use localic_utils::NEUTRON_CHAIN_DENOM;

    use super::*;

    const LOCAL_GRPC_URL: &str = "http://127.0.0.1";
    const LOCAL_GRPC_PORT: &str = "40231";
    const LOCAL_MNEMONIC: &str = "decorate bright ozone fork gallery riot bus exhaust worth way bone indoor calm squirrel merry zero scheme cotton until shop any excess stage laundry";
    const LOCAL_SIGNER_ADDR: &str = "neutron1hj5fveer5cjtn4wd6wstzugjfdxzl0xpznmsky";
    const LOCAL_ALT_ADDR: &str = "neutron1kljf09rj77uxeu5lye7muejx6ajsu55cuw2mws";
    const LOCAL_CHAIN_ID: &str = "localneutron-1";
    const LOCAL_PROCESSOR_ADDR: &str =
        "neutron12p7twsmksqw8lhj98hlxld7hxfl3tmwn6853ggtsalzm2ryx7ylsrmdfr6";

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
            .latest_block_height()
            .await
            .expect("Failed to get latest block height");

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
}
