#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw2::set_contract_version;
use valence_encoder_utils::msg::{
    ProcessorMessageToDecode, ProcessorMessageToEncode, QueryMsg as EncoderQueryMsg,
};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::ENCODERS,
};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(&msg.owner))?;

    msg.encoders
        .into_iter()
        .try_for_each(|(version, encoder)| {
            ENCODERS.save(deps.storage, version, &deps.api.addr_validate(&encoder)?)
        })?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", msg.owner))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::RegisterEncoder { version, address } => {
            cw_ownable::assert_owner(deps.storage, &info.sender)?;

            ENCODERS.save(
                deps.storage,
                version.clone(),
                &deps.api.addr_validate(&address)?,
            )?;

            Ok(Response::new()
                .add_attribute("method", "register_encoder")
                .add_attribute("address", address)
                .add_attribute("version", version))
        }
        ExecuteMsg::RemoveEncoder { version } => {
            cw_ownable::assert_owner(deps.storage, &info.sender)?;

            ENCODERS.remove(deps.storage, version.clone());

            Ok(Response::new()
                .add_attribute("method", "remove_encoder")
                .add_attribute("version", version))
        }
        ExecuteMsg::UpdateOwnership(action) => {
            let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
            Ok(Response::new().add_attributes(ownership.into_attributes()))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::Encoder { version } => to_json_binary(&get_encoder(deps, version)?),
        QueryMsg::ListEncoders {} => to_json_binary(&list_encoders(deps)?),
        QueryMsg::IsValidLibrary {
            encoder_version,
            library,
        } => to_json_binary(&is_valid_library(deps, encoder_version, library)?),
        QueryMsg::Encode {
            encoder_version,
            message,
        } => to_json_binary(&encode(deps, encoder_version, message)?),
        QueryMsg::Decode {
            encoder_version,
            message,
        } => to_json_binary(&decode(deps, encoder_version, message)?),
    }
}

fn get_encoder(deps: Deps, version: String) -> StdResult<String> {
    ENCODERS
        .load(deps.storage, version)
        .map(|addr| addr.to_string())
        .or_else(|_| Ok("".to_string()))
}

fn list_encoders(deps: Deps) -> StdResult<Vec<(String, Addr)>> {
    ENCODERS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (version, address) = item?;
            Ok((version, address))
        })
        .collect::<StdResult<Vec<(String, Addr)>>>()
}

fn is_valid_library(deps: Deps, encoder_version: String, library: String) -> StdResult<bool> {
    let encoder = ENCODERS.load(deps.storage, encoder_version)?;
    deps.querier
        .query_wasm_smart(encoder, &EncoderQueryMsg::IsValidLibrary { library })
}

fn encode(
    deps: Deps,
    encoder_version: String,
    message: ProcessorMessageToEncode,
) -> StdResult<Binary> {
    let encoder = ENCODERS.load(deps.storage, encoder_version)?;
    deps.querier
        .query_wasm_smart(encoder, &EncoderQueryMsg::Encode { message })
}

fn decode(
    deps: Deps,
    encoder_version: String,
    message: ProcessorMessageToDecode,
) -> StdResult<Binary> {
    let encoder = ENCODERS.load(deps.storage, encoder_version)?;
    deps.querier
        .query_wasm_smart(encoder, &EncoderQueryMsg::Decode { message })
}
