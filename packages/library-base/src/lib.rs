use cosmwasm_std::{CustomQuery, DepsMut, Env, MessageInfo, Response};
use helpers::assert_processor;
use serde::de::DeserializeOwned;
use serde::Serialize;

use state::PROCESSOR;
use valence_library_utils::{
    error::LibraryError,
    msg::{ExecuteMsg, InstantiateMsg, LibraryConfigValidation},
    raw_config::save_raw_library_config,
    LibraryConfigUpdateTrait,
};

pub mod helpers;
pub mod state;

pub use crate::state::{get_ownership, get_processor, load_config, load_raw_config, save_config};

pub fn instantiate<T, U>(
    deps: DepsMut,
    contract_name: &str,
    contract_version: &str,
    msg: InstantiateMsg<T>,
) -> Result<Response, LibraryError>
where
    T: LibraryConfigValidation<U> + Serialize + DeserializeOwned,
    U: Serialize + DeserializeOwned,
{
    cw2::set_contract_version(deps.storage, contract_name, contract_version)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(&msg.owner))?;

    PROCESSOR.save(deps.storage, &deps.api.addr_validate(&msg.processor)?)?;

    // Saves the raw library config
    save_raw_library_config(deps.storage, &msg.config)?;

    let config = msg.config.validate(deps.as_ref())?;
    save_config(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("processor", msg.processor)
        .add_attribute("owner", format!("{:?}", msg.owner)))
}

type ProcessFunction<M, Q, T, U> =
    fn(DepsMut<Q>, Env, MessageInfo, T, U) -> Result<Response<M>, LibraryError>;
type UpdateConfig<Q, V> = fn(DepsMut<Q>, Env, MessageInfo, V) -> Result<(), LibraryError>;

pub fn execute<M, Q, T, U, V>(
    deps: DepsMut<Q>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<T, V>,
    process_function: ProcessFunction<M, Q, T, U>,
    update_config: UpdateConfig<Q, V>,
) -> Result<Response<M>, LibraryError>
where
    Q: CustomQuery,
    U: Serialize + DeserializeOwned,
    V: LibraryConfigUpdateTrait + Serialize + DeserializeOwned,
{
    match msg {
        ExecuteMsg::ProcessFunction(function) => {
            assert_processor(deps.as_ref().storage, &info.sender)?;
            let config = load_config(deps.storage)?;
            process_function(deps, env, info, function, config)
        }
        ExecuteMsg::UpdateConfig { new_config } => {
            cw_ownable::assert_owner(deps.as_ref().storage, &info.sender)?;
            // We update the raw storage
            new_config.update_raw(deps.storage)?;
            update_config(deps, env, info, new_config)?;
            Ok(Response::new().add_attribute("method", "update_config"))
        }
        ExecuteMsg::UpdateProcessor { processor } => {
            cw_ownable::assert_owner(deps.as_ref().storage, &info.sender)?;
            PROCESSOR.save(deps.storage, &deps.api.addr_validate(&processor)?)?;
            Ok(Response::default()
                .add_attribute("method", "update_processor")
                .add_attribute("processor", processor))
        }
        ExecuteMsg::UpdateOwnership(action) => {
            let result = cw_ownable::update_ownership(
                deps.into_empty(),
                &env.block,
                &info.sender,
                action.clone(),
            )?;
            Ok(Response::default()
                .add_attribute("method", "update_ownership")
                .add_attribute("action", format!("{action:?}"))
                .add_attribute("result", format!("{result:?}")))
        }
    }
}
