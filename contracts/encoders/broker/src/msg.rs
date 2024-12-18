use std::collections::HashMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use valence_encoder_utils::msg::EncodingMessage;

#[cw_serde]
pub struct InstantiateMsg {
    // Version -> Address
    pub encoders: HashMap<String, String>,
    pub owner: String,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    // Version -> Address
    RegisterEncoder { version: String, address: String },
    // Version
    RemoveEncoder { version: String },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    Encoder { version: String },
    #[returns(Vec<(String, Addr)>)]
    ListEncoders {},
    // Checks that the library and function that we want to encode into exist in the encoder for the given version
    #[returns(bool)]
    IsValidEncodingInfo {
        encoder_version: String,
        library: String,
        function: String,
    },
    // Encodes the message
    #[returns(Binary)]
    Encode {
        encoder_version: String,
        encoding_message: EncodingMessage,
    },
}
