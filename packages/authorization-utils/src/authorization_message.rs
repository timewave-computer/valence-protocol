use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;

#[cw_serde]
pub struct MessageDetails {
    pub message_type: MessageType,
    pub message: Message,
}

#[cw_serde]
pub enum MessageType {
    CosmwasmExecuteMsg,
    CosmwasmMigrateMsg,
}

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
}
