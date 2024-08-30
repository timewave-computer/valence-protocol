use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, StdError, StdResult, SubMsg};
use cw_utils::Expiration;
use serde_json::{json, Value};
use valence_authorization_utils::{
    authorization::{ActionBatch, Priority},
    msg::ProcessorMessage,
};

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
    pub priority: Priority,
}

impl MessageBatch {
    /// This is for atomic batches
    /// For the last action in the batch, we will always reply so that we can send the successful callback when completed
    /// For the rest of the actions, we will reply on error so that we can retry the batch if needed
    pub fn create_atomic_messages(&self) -> Vec<SubMsg> {
        self.msgs
            .iter()
            .zip(&self.action_batch.actions)
            .enumerate()
            .map(|(index, (msg, action))| {
                let wasm_message = msg.to_wasm_message(&action.contract_address);
                if index == self.msgs.len() - 1 {
                    SubMsg::reply_always(wasm_message, self.id)
                } else {
                    SubMsg::reply_on_error(wasm_message, self.id)
                }
            })
            .collect()
    }

    /// This is used for non-atomic batches. We need to catch the reply always because we need to know if the message was successful to continue
    /// with the next message in the batch or apply the retry logic
    pub fn create_message_by_index(&self, index: usize) -> Vec<SubMsg> {
        let submessage = SubMsg::reply_always(
            self.msgs[index].to_wasm_message(&self.action_batch.actions[index].contract_address),
            self.id,
        );
        vec![submessage]
    }

    /// Very similar to create_message_by_index, but we append an execution id to the message
    /// so that the service can know to which ID it has to reply to
    pub fn create_message_by_index_with_execution_id(
        &self,
        index: usize,
        execution_id: u64,
    ) -> StdResult<Vec<SubMsg>> {
        // Extract the json from the message
        // This won't fail because we've already validated the message before
        let mut json: Value = serde_json::from_slice(self.msgs[index].get_msg())
            .map_err(|_| StdError::generic_err("Invalid json"))?;

        // Append the execution id to the message
        if let Value::Object(ref mut map) = json {
            if let Some(Value::Object(ref mut inner_map)) = map.values_mut().next() {
                inner_map.insert("execution_id".to_string(), json!(execution_id));
            }
        }

        let submessage = SubMsg::reply_always(
            self.msgs[index].to_wasm_message(&self.action_batch.actions[index].contract_address),
            execution_id,
        );

        Ok(vec![submessage])
    }
}

#[cw_serde]
pub struct CurrentRetry {
    pub retry_amounts: u64,
    pub retry_cooldown: Expiration,
}
