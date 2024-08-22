use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;
use valence_authorization_utils::authorization::ActionBatch;

#[cw_serde]
pub struct MessageBatch {
    // Used for the callback
    pub id: u64,
    pub msgs: Vec<Binary>,
    pub action_batch: ActionBatch,
}
