use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;
use valence_authorization_utils::authorization::{Priority, Subroutine};

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(bool)]
    IsValidLibrary { library: String },
    #[returns(Binary)]
    Encode { message: ProcessorMessageToEncode },
}

#[cw_serde]
pub enum ProcessorMessageToEncode {
    SendMsgs {
        execution_id: u64,
        priority: Priority,
        subroutine: Subroutine,
        messages: Vec<Message>,
    },
    InsertMsgs {
        execution_id: u64,
        queue_position: u64,
        priority: Priority,
        subroutine: Subroutine,
        messages: Vec<Message>,
    },
    EvictMsgs {
        queue_position: u64,
        priority: Priority,
    },
    Pause {},
    Resume {},
}

#[cw_serde]
pub struct Message {
    pub library: String,
    pub data: Binary,
}
