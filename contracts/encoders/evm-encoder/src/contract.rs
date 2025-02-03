#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;
use valence_encoder_utils::msg::{ProcessorMessageToDecode, ProcessorMessageToEncode, QueryMsg};

use crate::{
    hyperlane,
    processor::{evict_msgs, insert_msgs, pause, resume, send_msgs},
    EVMLibrary,
};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(_deps: DepsMut, _env: Env, _info: MessageInfo, _msg: Empty) -> StdResult<Response> {
    unimplemented!("This contract does not handle any execute messages, only queries")
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::IsValidLibrary { library } => to_json_binary(&is_valid_library(library)),
        QueryMsg::Encode { message } => encode(message),
        QueryMsg::Decode { message } => decode(message),
    }
}

fn is_valid_library(library: String) -> bool {
    EVMLibrary::is_valid(&library)
}

fn encode(message: ProcessorMessageToEncode) -> StdResult<Binary> {
    match message {
        ProcessorMessageToEncode::SendMsgs {
            execution_id,
            priority,
            subroutine,
            messages,
        } => send_msgs::encode(execution_id, priority, subroutine, messages),
        ProcessorMessageToEncode::InsertMsgs {
            execution_id,
            queue_position,
            priority,
            subroutine,
            messages,
        } => insert_msgs::encode(execution_id, queue_position, priority, subroutine, messages),
        ProcessorMessageToEncode::EvictMsgs {
            queue_position,
            priority,
        } => evict_msgs::encode(queue_position, priority),
        ProcessorMessageToEncode::Pause {} => Ok(pause::encode()),
        ProcessorMessageToEncode::Resume {} => Ok(resume::encode()),
    }
}

fn decode(message: ProcessorMessageToDecode) -> StdResult<Binary> {
    match message {
        ProcessorMessageToDecode::HyperlaneCallback { callback } => {
            Ok(hyperlane::callback::decode(&callback)?)
        }
    }
}
