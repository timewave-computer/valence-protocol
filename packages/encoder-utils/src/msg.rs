use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;
use valence_authorization_utils::authorization::{Priority, Subroutine};

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(bool)]
    IsValidEncodingInfo { library: String, function: String },
    #[returns(Binary)]
    Encode { encoding_message: EncodingMessage },
}

#[cw_serde]
pub enum EncodingMessage {
    SendMsgs {
        priority: Priority,
        subroutine: Subroutine,
        msgs: Vec<Binary>,
    },
    InsertMsgs {
        queue_position: u64,
        priority: Priority,
        subroutine: Subroutine,
        msgs: Vec<Binary>,
    },
    EvictMsgs {
        queue_position: u64,
        priority: Priority,
    },
    Pause {},
    Resume {},
}
