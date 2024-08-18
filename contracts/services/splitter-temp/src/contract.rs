#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{CONFIG, PROCESSOR};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:base_service";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    cw_ownable::initialize_owner(deps.storage, deps.api, Some(&msg.owner))?;
    PROCESSOR.save(deps.storage, &deps.api.addr_validate(&msg.processor)?)?;

    let config = msg.config.validate(deps.as_ref())?;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", msg.owner))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateProcessor { processor } => {
            cw_ownable::is_owner(deps.storage, &info.sender)?;
            PROCESSOR.save(deps.storage, &deps.api.addr_validate(&processor)?)?;
            Ok(Response::default())
        }
        ExecuteMsg::UpdateOwnership(action) => {
            cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
            Ok(Response::default())
        }
        ExecuteMsg::UpdateConfig { new_config } => execute::update_config(deps, info, new_config),
        ExecuteMsg::Processor(action_msg) => actions::handle_action(deps, env, info, action_msg),
    }
}

mod execute {
    use cosmwasm_std::{DepsMut, MessageInfo, Response};

    use crate::{msg::OptionalServiceConfig, ContractError};

    pub fn update_config(
        deps: DepsMut,
        info: MessageInfo,
        new_config: OptionalServiceConfig,
    ) -> Result<Response, ContractError> {
        cw_ownable::is_owner(deps.storage, &info.sender)?;

        new_config.update_config(deps)?;

        Ok(Response::new()
            .add_attribute("method", "update_config")
            .add_attribute("updated_by", info.sender))
    }
}

mod actions {
    use cosmwasm_std::{
        to_json_binary, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, WasmMsg,
    };

    use crate::{helpers::is_processor, msg::ActionsMsgs, state::CONFIG, ContractError};

    pub fn handle_action(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        msg: ActionsMsgs,
    ) -> Result<Response, ContractError> {
        is_processor(&deps, &info)?;

        match msg {
            ActionsMsgs::Split {} => {
                let config = CONFIG.load(deps.storage)?;
                let mut messages: Vec<CosmosMsg> = vec![];

                config.splits.iter().try_for_each(|(denom, split)| {
                    // Query bank balance
                    let balance = deps.querier.query_balance(&config.input_addr, denom)?;

                    // TODO: Check that balance is not zero
                    if !balance.amount.is_zero() {
                        // TODO: change split to be percentage and not amounts
                        messages.extend(
                            split
                                .iter()
                                .map(|(addr, amount)| {
                                    let bank_msg = BankMsg::Send {
                                        to_address: addr.to_string()?,
                                        amount: vec![Coin {
                                            denom: denom.clone(),
                                            amount: *amount,
                                        }],
                                    };

                                    Ok(WasmMsg::Execute {
                                        contract_addr: config.input_addr.to_string(),
                                        msg: to_json_binary(
                                            &base_account::msg::ExecuteMsg::ExecuteMsg {
                                                msgs: vec![bank_msg.into()],
                                            },
                                        )?,
                                        funds: vec![],
                                    }
                                    .into())
                                })
                                .collect::<Result<Vec<_>, ContractError>>()?,
                        );
                    }

                    Ok::<(), ContractError>(())
                })?;

                Ok(Response::new().add_attribute("method", "split"))
            }
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetAdmin {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::GetServiceConfig {} => to_json_binary(&CONFIG.load(deps.storage)?),
    }
}

#[cfg(test)]
mod tests {}
