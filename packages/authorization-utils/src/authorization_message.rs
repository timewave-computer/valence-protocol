use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;

use crate::msg::EncoderInfo;

#[cw_serde]
pub struct MessageDetails {
    pub message_type: MessageType,
    pub message: Message,
}

#[cw_serde]
pub enum MessageType {
    CosmwasmExecuteMsg,
    CosmwasmMigrateMsg,
    SolidityCall(EncoderInfo, LibraryName),
    SolidityRawCall,
}

// LibraryName is the library we want to encode the message for on the encoder we pass it as a String to avoid requiring to migrate the contract to
// create authorizations for libraries we add later on the encoders
#[cw_serde]
pub struct LibraryName(String);

#[cw_serde]
pub struct Message {
    // Name of the message that is passed to the contract, e.g. in CosmWasm: the snake_case name of the ExecuteMsg, how it's passed in the JSON
    pub name: String,
    pub params_restrictions: Option<Vec<ParamRestriction>>,
}

#[cw_serde]
pub enum ParamRestriction {
    // First parameter is an array of indexes in the json to know what we have to look for
    // Example: ["msg", "amount"] means that we have to look for the amount index inside the msg index
    // example_json = { "msg": { "amount": 100 } }
    MustBeIncluded(Vec<String>),
    CannotBeIncluded(Vec<String>),
    MustBeValue(Vec<String>, Binary),
    // Used when we are passing the raw bytes to be executed in another domain, e.g. ABI encoded bytes for Solidity
    // This will restrict that the bytes passed are restricted to this value.
    MustBeBytes(Binary),
}
