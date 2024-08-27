use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary};
use valence_authorization_utils::authorization::ActionBatch;

#[cw_serde]
pub struct Config {
    // Address of the authorization contract
    pub authorization_contract: Addr,
    // If processor is sitting on a different chain we need to know the polytone contracts
    pub polytone_contracts: Option<Polytone>,
    pub state: State,
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
    pub msgs: Vec<Binary>,
    pub action_batch: ActionBatch,
}
