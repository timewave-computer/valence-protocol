use cosmos_sdk_proto::traits::MessageExt;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use neutron_sdk::proto_types::osmosis::tokenfactory::v1beta1::MsgCreateDenom;
use valence_service_utils::{
    error::ServiceError,
    msg::{ExecuteMsg, InstantiateMsg},
};

use crate::{
    msg::{ActionMsgs, Config, QueryMsg, ServiceConfig, ServiceConfigUpdate},
    state::TOKENIZED_DENOM,
};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const SUBDENOM: &str = "tokenizer";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg<ServiceConfig>,
) -> Result<Response, ServiceError> {
    // create TF token
    let create_denom_msg = create_denom_msg(env.contract.address.to_string());

    // Full denom of the token that will be created
    let denom = build_tokenfactory_denom(env.contract.address.as_str(), SUBDENOM);
    TOKENIZED_DENOM.save(deps.storage, &denom)?;

    // init the base service
    valence_service_base::instantiate(deps, CONTRACT_NAME, CONTRACT_VERSION, msg)?;

    Ok(Response::default().add_message(create_denom_msg))
}

/// Returns the full denom of a tokenfactory token: factory/<contract_address>/<label>
pub fn build_tokenfactory_denom(contract_address: &str, label: &str) -> String {
    format!("factory/{}/{}", contract_address, label)
}

fn create_denom_msg(sender: String) -> CosmosMsg {
    let msg_create_denom = MsgCreateDenom {
        sender,
        subdenom: SUBDENOM.to_string(),
    };
    // TODO: Change to AnyMsg instead of Stargate when we can test with CW 2.0 (They are the same, just a rename)
    #[allow(deprecated)]
    CosmosMsg::Stargate {
        type_url: "/osmosis.tokenfactory.v1beta1.MsgCreateDenom".to_string(),
        value: Binary::from(msg_create_denom.to_bytes().unwrap()),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<ActionMsgs, ServiceConfigUpdate>,
) -> Result<Response, ServiceError> {
    valence_service_base::execute(
        deps,
        env,
        info,
        msg,
        actions::process_action,
        execute::update_config,
    )
}

mod actions {
    use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
    use valence_service_utils::error::ServiceError;

    use crate::msg::{ActionMsgs, Config};

    pub fn process_action(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ActionMsgs,
        cfg: Config,
    ) -> Result<Response, ServiceError> {
        match msg {
            ActionMsgs::Tokenize { sender } => try_tokenize(deps, env, info, sender, cfg),
        }
    }

    fn try_tokenize(
        _deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        sender: String,
        _cfg: Config,
    ) -> Result<Response, ServiceError> {
        Ok(Response::new().add_attribute("method", "noop"))
    }
}

mod execute {
    use cosmwasm_std::{DepsMut, Env, MessageInfo};
    use valence_service_utils::error::ServiceError;

    use crate::msg::ServiceConfigUpdate;

    pub fn update_config(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        new_config: ServiceConfigUpdate,
    ) -> Result<(), ServiceError> {
        new_config.update_config(deps)
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => {
            to_json_binary(&valence_service_base::get_ownership(deps.storage)?)
        }
        QueryMsg::GetProcessor {} => {
            to_json_binary(&valence_service_base::get_processor(deps.storage)?)
        }
        QueryMsg::GetServiceConfig {} => {
            let config: Config = valence_service_base::load_config(deps.storage)?;
            to_json_binary(&config)
        }
        QueryMsg::GetRawServiceConfig {} => {
            let raw_config: ServiceConfig = valence_service_base::load_raw_config(deps.storage)?;
            to_json_binary(&raw_config)
        }
    }
}
