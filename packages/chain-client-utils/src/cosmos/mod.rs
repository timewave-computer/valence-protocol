pub mod base_client;
pub mod errors;
pub mod grpc_client;
pub mod proto_timestamp;
pub mod signing_client;
pub mod wasm_client;

pub(crate) type CosmosServiceClient<T> =
    cosmos_sdk_proto::cosmos::tx::v1beta1::service_client::ServiceClient<T>;
pub(crate) type WasmQueryClient<T> =
    cosmrs::proto::cosmwasm::wasm::v1::query_client::QueryClient<T>;
pub(crate) type BankQueryClient<T> =
    cosmrs::proto::cosmos::bank::v1beta1::query_client::QueryClient<T>;
pub(crate) type AuthQueryClient<T> =
    cosmos_sdk_proto::cosmos::auth::v1beta1::query_client::QueryClient<T>;
