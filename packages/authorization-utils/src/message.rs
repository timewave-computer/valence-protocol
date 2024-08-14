use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;

#[cw_serde]
pub struct MessageInfo {
    pub message_type: MessageType,
    pub message: Message,
}

#[cw_serde]
// Abstracting this because maybe we might have different message types in the future (e.g. Migration)
pub enum MessageType {
    ExecuteMsg,
}

#[cw_serde]
pub struct Message {
    // Name of the message that is passed to the contract, e.g. in CosmWasm: the snake_case name of the ExecuteMsg, how it's passed in the JSON
    pub name: String,
    pub params_restrictions: Option<Vec<ParamsRestrictions>>,
}

#[cw_serde]
// ParamRestrictinos will be passed separating it by a "." character. Example: If we want to specify that the json must have a param "address" under "owner" included, we will use
// MustBeIncluded("owner.address")
pub enum ParamsRestrictions {
    MustBeIncluded(String),
    CannotBeIncluded(String),
    // Will check that the param defined in String is included, and if it is, we will compare it with Binary (after converting it to Binary)
    MustBeValue(String, Binary),
}
