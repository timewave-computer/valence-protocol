use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Binary, StdError};
use valence_authorization_utils::authorization::ActionBatch;

#[cw_serde]
pub struct Config {
    // Address of the authorization contract
    pub authorization_contract: Addr,
    // If processor is sitting on a different chain we need to know the polytone contracts
    pub polytone_contracts: Option<PolytoneContracts>,
    pub state: State,
}

impl Config {
    pub fn is_valid(&self, api: &dyn Api) -> Result<(), StdError> {
        api.addr_validate(self.authorization_contract.as_str())?;
        if let Some(polytone_contracts) = &self.polytone_contracts {
            api.addr_validate(polytone_contracts.polytone_proxy_contract.as_str())?;
            api.addr_validate(polytone_contracts.polytone_note_contract.as_str())?;
        }

        Ok(())
    }
}

#[cw_serde]
pub struct PolytoneContracts {
    // Address of proxy contract if processor is sitting on a different chain
    pub polytone_proxy_contract: Addr,
    // Address of note contract (for callbacks) if processor is sitting on a different chain
    pub polytone_note_contract: Addr,
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
