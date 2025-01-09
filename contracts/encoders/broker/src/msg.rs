use std::collections::HashMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use valence_encoder_utils::msg::{ProcessorMessageToDecode, ProcessorMessageToEncode};

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
    // Checks that the library that we want to encode into exists for a specific encoder version
    #[returns(bool)]
    IsValidLibrary {
        encoder_version: String,
        library: String,
    },
    // Encodes the message
    #[returns(Binary)]
    Encode {
        encoder_version: String,
        message: ProcessorMessageToEncode,
    },
    // Decodes the message
    #[returns(Binary)]
    Decode {
        encoder_version: String,
        message: ProcessorMessageToDecode,
    },
}
