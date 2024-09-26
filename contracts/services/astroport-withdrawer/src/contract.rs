#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use crate::{
    error::{ServiceError, UnauthorizedReason},
    msg::{Config, ExecuteMsg, InstantiateMsg, QueryMsg, ServiceConfigValidation},
    state::{CONFIG, PROCESSOR},
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
) -> Result<Response, ServiceError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(&msg.owner))?;

    PROCESSOR.save(deps.storage, &deps.api.addr_validate(&msg.processor)?)?;

    let config = msg.config.validate(deps.as_ref())?;
    CONFIG.save(deps.storage, &config)?;

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
) -> Result<Response, ServiceError> {
    match msg {
        ExecuteMsg::ProcessAction(action_msgs) => {
            let processor = PROCESSOR.load(deps.storage)?;
            if info.sender != processor {
                return Err(ServiceError::Unauthorized(
                    UnauthorizedReason::NotAllowed {},
                ));
            }
            let config = CONFIG.load(deps.storage)?;
            actions::process_action(deps, env, info, action_msgs, config)
        }
        ExecuteMsg::UpdateConfig { new_config } => {
            cw_ownable::assert_owner(deps.as_ref().storage, &info.sender)?;
            let config = new_config.validate(deps.as_ref())?;
            CONFIG.save(deps.storage, &config)?;
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
            let result =
                cw_ownable::update_ownership(deps, &env.block, &info.sender, action.clone())?;
            Ok(Response::default()
                .add_attribute("method", "update_ownership")
                .add_attribute("action", format!("{:?}", action))
                .add_attribute("result", format!("{:?}", result)))
        }
    }
}

mod actions {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{
        to_json_binary, Addr, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdResult, WasmMsg,
    };

    use crate::{
        astroport_cw20, astroport_native,
        error::ServiceError,
        msg::{ActionsMsgs, Config, PoolType},
    };

    pub fn process_action(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: ActionsMsgs,
        cfg: Config,
    ) -> Result<Response, ServiceError> {
        match msg {
            ActionsMsgs::WithdrawLiquidity {} => withdraw_liquidity(deps, cfg),
        }
    }

    fn withdraw_liquidity(deps: DepsMut, cfg: Config) -> Result<Response, ServiceError> {
        let msgs = create_withdraw_liquidity_msgs(&deps, &cfg)?;

        let input_account_msgs = execute_on_behalf_of(msgs, &cfg.input_addr)?;

        Ok(Response::new()
            .add_message(input_account_msgs)
            .add_attribute("method", "withdraw_liquidity"))
    }

    fn create_withdraw_liquidity_msgs(
        deps: &DepsMut,
        cfg: &Config,
    ) -> Result<Vec<CosmosMsg>, ServiceError> {
        match &cfg.withdrawer_config.pool_type {
            PoolType::NativeLpToken => astroport_native::create_withdraw_liquidity_msgs(deps, cfg),
            PoolType::Cw20LpToken => astroport_cw20::create_withdraw_liquidity_msgs(deps, cfg),
        }
    }

    // This is a helper function to execute a CosmosMsg on behalf of an account
    pub fn execute_on_behalf_of(msgs: Vec<CosmosMsg>, account: &Addr) -> StdResult<CosmosMsg> {
        // Used to execute a CosmosMsg on behalf of an account
        #[cw_serde]
        pub enum ExecuteMsg {
            ExecuteMsg { msgs: Vec<CosmosMsg> }, // Execute any CosmosMsg (approved services or admin)
        }

        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: account.to_string(),
            msg: to_json_binary(&ExecuteMsg::ExecuteMsg { msgs })?,
            funds: vec![],
        }))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::GetProcessor {} => {
            let processor = PROCESSOR.load(deps.storage)?;
            to_json_binary(&processor)
        }
        QueryMsg::GetServiceConfig {} => {
            let config: Config = CONFIG.load(deps.storage)?;
            to_json_binary(&config)
        }
    }
}
