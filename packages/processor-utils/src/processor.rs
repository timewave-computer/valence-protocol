use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, CosmosMsg, StdError, StdResult, SubMsg, WasmMsg};
use cw_utils::Expiration;
use serde_json::{json, Value};
use valence_authorization_utils::{
    authorization::{Priority, Subroutine},
    domain::PolytoneProxyState,
    function::Function,
    msg::ProcessorMessage,
};

#[cw_serde]
pub struct Config {
    // Address of the authorization contract (if the processor is an external domain processor the authorization contract sits on another domain)
    pub authorization_contract: String,
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
    // Timeout for the IBC messages
    pub timeout_seconds: u64,
    // State of proxy on main domain
    pub proxy_on_main_domain_state: PolytoneProxyState,
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
    pub subroutine: Subroutine,
    pub priority: Priority,
    pub retry: Option<CurrentRetry>,
}

impl From<MessageBatch> for Vec<CosmosMsg> {
    fn from(val: MessageBatch) -> Self {
        match val.subroutine {
            Subroutine::Atomic(config) => process_functions(val.msgs, &config.functions),
            Subroutine::NonAtomic(config) => process_functions(val.msgs, &config.functions),
        }
    }
}

fn process_functions<F: Function>(msgs: Vec<ProcessorMessage>, functions: &[F]) -> Vec<CosmosMsg> {
    msgs.into_iter()
        .zip(functions)
        .map(|(msg, function)| create_cosmos_msg(msg, function.get_contract_address().to_owned()))
        .collect()
}

fn create_cosmos_msg(msg: ProcessorMessage, contract_address: String) -> CosmosMsg {
    match msg {
        ProcessorMessage::CosmwasmExecuteMsg { msg } => CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_address,
            msg,
            funds: vec![],
        }),
        ProcessorMessage::CosmwasmMigrateMsg { code_id, msg } => {
            CosmosMsg::Wasm(WasmMsg::Migrate {
                contract_addr: contract_address,
                new_code_id: code_id,
                msg,
            })
        }
    }
}

impl MessageBatch {
    /// This is used for non-atomic batches. We need to catch the reply always because we need to know if the message was successful to continue
    /// with the next message in the batch or apply the retry logic
    pub fn create_message_by_index(&self, index: usize) -> Vec<SubMsg> {
        let contract_address = self
            .subroutine
            .get_contract_address_by_function_index(index);

        let submessage =
            SubMsg::reply_always(self.msgs[index].to_wasm_message(&contract_address), self.id);
        vec![submessage]
    }

    /// Very similar to create_message_by_index, but we append an execution id to the message
    /// so that the library can know to which ID it has to reply to
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

        // Use this json to create a new message
        let mut new_msg = self.msgs[index].clone();
        new_msg.set_msg(Binary::from(
            serde_json::to_vec(&json).map_err(|e| StdError::generic_err(e.to_string()))?,
        ));

        let contract_address = self
            .subroutine
            .get_contract_address_by_function_index(index);

        let submessage =
            SubMsg::reply_always(new_msg.to_wasm_message(&contract_address), execution_id);
        Ok(vec![submessage])
    }
}

#[cw_serde]
pub struct CurrentRetry {
    pub retry_amounts: u64,
    pub retry_cooldown: Expiration,
}
