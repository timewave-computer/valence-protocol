use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary};
use valence_authorization_utils::{authorization::ActionBatch, message::MessageType};

#[cw_serde]
pub struct Config {
    // Address of the authorization contract
    pub authorization_contract: Addr,
    pub processor_domain: ProcessorDomain,
    pub state: State,
}

#[cw_serde]
pub enum ProcessorDomain {
    Main,
    External(Polytone),
}

#[cw_serde]
pub struct Polytone {
    // Address of proxy contract if processor is sitting on a different chain
    pub polytone_proxy_address: Addr,
    // Address of note contract (for callbacks) if processor is sitting on a different chain
    pub polytone_note_address: Addr,
}

#[cw_serde]
pub enum State {
    Paused,
    Active,
}

#[cw_serde]
pub struct MessageBatch {
    // Used for the callback
    pub id: u64,
    pub msgs: Vec<ProcessorMessage>,
    pub action_batch: ActionBatch,
}

#[cw_serde]
pub enum ProcessorMessage {
    CosmwasmExecuteMsg { msg: Binary },
    CosmwasmMigrateMsg { code_id: u64, msg: Binary },
}

impl ProcessorMessage {
    pub fn get_message_type(&self) -> MessageType {
        match self {
            ProcessorMessage::CosmwasmExecuteMsg { .. } => MessageType::CosmwasmExecuteMsg,
            ProcessorMessage::CosmwasmMigrateMsg { .. } => MessageType::CosmwasmMigrateMsg,
        }
    }
}
