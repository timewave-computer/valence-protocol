use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{StdError, StdResult};
use neutron_sdk::bindings::{msg::NeutronMsg, types::InterchainQueryResult};
use serde_json::Value;

#[cw_serde]
pub struct InstantiateMsg {
    // connection id of associated chain
    pub connection_id: String,
}

#[cw_serde]
pub enum ExecuteMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(QueryRegistrationInfoResponse)]
    GetRegistrationConfig(QueryRegistrationInfoRequest),

    #[returns(QueryReconstructionResponse)]
    ReconstructQuery(QueryReconstructionRequest),
}

#[cw_serde]
pub struct QueryReconstructionRequest {
    pub icq_result: InterchainQueryResult,
    pub query_type: String,
}

#[cw_serde]
pub struct QueryReconstructionResponse {
    pub json_value: Value,
}

#[cw_serde]
pub struct QueryRegistrationInfoRequest {
    /// module here refers to some string identifier of the query we want to perform.
    /// one useful identifier is that of the proto type, e.g. `/osmosis.gamm.v1beta1.Pool`.
    /// basically describes what type we are dealing with
    pub module: String,
    /// params here describe the parameters to be passed into our query request.
    /// if module above describes the what, these params describe the how.
    pub params: serde_json::Map<String, Value>,
}

#[cw_serde]
pub struct QueryRegistrationInfoResponse {
    pub registration_msg: NeutronMsg,
    pub reply_id: u64,
}

#[cw_serde]
pub struct PendingQueryIdConfig {
    pub associated_domain_registry: String,
    pub query_type: String,
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum TypeRegistryError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unknown reply id: {0}")]
    UnknownReplyId(u64),

    #[error("Unsupported module: {0}")]
    UnsupportedModule(String),

    #[error("Unknown type URL: {0}")]
    UnknownTypeUrl(String),

    #[error("json field {0} missing from the query params: {1:?}")]
    JsonFieldMissing(String, serde_json::Map<String, Value>),
}

impl From<TypeRegistryError> for StdError {
    fn from(val: TypeRegistryError) -> Self {
        match val {
            TypeRegistryError::Std(std_error) => std_error,
            e => StdError::generic_err(e.to_string()),
        }
    }
}

/// macro to generate enums for types we wish to support in each domain
/// registry
#[macro_export]
macro_rules! define_registry_types {
    ($(($variant:ident, $type:ty)),* $(,)?) => {
        #[allow(clippy::large_enum_variant)]
        #[cw_serde]
        pub enum DomainRegistryType {
            $(
                $variant($type),
            )*
        }

        /// default implementation for a str parser to go from proto type url to
        /// the actual type
        impl FromStr for DomainRegistryType
        where $($type: QueryTypeDefinition, )*
        {
            type Err = $crate::TypeRegistryError;

            fn from_str(type_url: &str) -> Result<Self, Self::Err> {
                match type_url {
                    $(
                        <$type>::TYPE_URL => Ok(DomainRegistryType::$variant(<$type>::default())),
                    )*
                    _ => Err($crate::TypeRegistryError::UnknownTypeUrl(type_url.to_string())),
                }
            }
        }

        impl DomainRegistryType {
            pub fn get_registration_config(&self, params: serde_json::Map<String, Value>) -> StdResult<(KVKey, u64)> {
                match self {
                    $(
                        DomainRegistryType::$variant(t) => Ok((
                            t.get_kv_key(params)?,
                            <$type>::REPLY_ID,                                )),
                    )*
                }
            }

            pub fn reconstruct_response(&self, request: &QueryReconstructionRequest) -> StdResult<Value> {
                match self {
                    $(
                        DomainRegistryType::$variant(_t) => <$type>::decode_and_reconstruct(request),
                    )*
                }
            }
        }

    };
}

pub fn get_u64_query_param(params: &serde_json::Map<String, Value>, key: &str) -> StdResult<u64> {
    let value = match params.get(key) {
        Some(Value::Number(number)) => number.as_u64().ok_or(StdError::generic_err(format!(
            "failed to parse {:?} as u64 for {key} access",
            number
        ))),
        Some(Value::String(str_num)) => str_num.parse::<u64>().map_err(|_| {
            StdError::generic_err(format!(
                "failed to parse {:?} to u64 for key {key}",
                str_num
            ))
        }),
        _ => Err(StdError::generic_err(format!(
            "field pool_id missing from query params: {:?}",
            params
        ))),
    }?;

    Ok(value)
}

pub fn get_str_query_param(
    params: &serde_json::Map<String, Value>,
    key: &str,
) -> StdResult<String> {
    let value = match params.get(key) {
        Some(Value::String(str_val)) => Ok(str_val),
        _ => Err(StdError::generic_err(format!(
            "field {key} missing from query params: {:?}",
            params
        ))),
    }?;

    Ok(value.to_string())
}
