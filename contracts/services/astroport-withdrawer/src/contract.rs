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
        error::ServiceError,
        msg::{ActionsMsgs, Config, PoolType},
    };

    use super::{astroport_cw20, astroport_native};

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

mod astroport_native {
    use super::*;
    use astroport::asset::{Asset, PairInfo};
    use cosmwasm_std::{Coin, CosmosMsg};

    pub fn query_liquidity_token(deps: &DepsMut, cfg: &Config) -> Result<String, ServiceError> {
        let pair_info: PairInfo = deps
            .querier
            .query_wasm_smart(cfg.pool_addr.clone(), &astroport::pair::QueryMsg::Pair {})?;

        Ok(pair_info.liquidity_token)
    }

    pub fn create_withdraw_liquidity_msgs(
        deps: &DepsMut,
        cfg: &Config,
    ) -> Result<Vec<CosmosMsg>, ServiceError> {
        // Get the token factory token that represents the liquidity token
        let token = query_liquidity_token(deps, cfg)?;

        // Query the balance of the account that is going to withdraw
        let balance = deps.querier.query_balance(&cfg.input_addr, &token)?;
        if balance.amount.is_zero() {
            return Err(ServiceError::ExecutionError(
                "Nothing to withdraw".to_string(),
            ));
        }

        // Calculate how much we are going to get when we withdraw
        let withdrawn_assets: Vec<Asset> = deps.querier.query_wasm_smart(
            cfg.pool_addr.clone(),
            &astroport::pair::QueryMsg::Share {
                amount: balance.amount,
            },
        )?;

        // Create the withdraw and send messages
        let withdraw_msg = CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
            contract_addr: cfg.pool_addr.to_string(),
            msg: to_json_binary(&astroport::pair::ExecuteMsg::WithdrawLiquidity {
                assets: vec![],
                min_assets_to_receive: Some(withdrawn_assets.clone()),
            })?,
            funds: vec![balance],
        });

        // Send the withdrawn assets to the output account
        let withdrawn_coins = withdrawn_assets
            .into_iter()
            .map(|asset| asset.as_coin())
            .collect::<Result<Vec<Coin>, _>>()?;

        let send_msg = CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
            to_address: cfg.output_addr.to_string(),
            amount: withdrawn_coins,
        });

        Ok(vec![withdraw_msg, send_msg])
    }
}

mod astroport_cw20 {
    use super::*;
    use astroport_cw20_lp_token::asset::{Asset, PairInfo};
    use cosmwasm_std::{Addr, Coin, CosmosMsg};
    use cw20::{BalanceResponse, Cw20ExecuteMsg};

    pub fn query_liquidity_token(deps: &DepsMut, cfg: &Config) -> Result<Addr, ServiceError> {
        let pair_info: PairInfo = deps.querier.query_wasm_smart(
            cfg.pool_addr.clone(),
            &astroport_cw20_lp_token::pair::QueryMsg::Pair {},
        )?;

        Ok(pair_info.liquidity_token)
    }

    pub fn create_withdraw_liquidity_msgs(
        deps: &DepsMut,
        cfg: &Config,
    ) -> Result<Vec<CosmosMsg>, ServiceError> {
        // Get the token factory token that represents the liquidity token
        let token_addr = query_liquidity_token(deps, cfg)?;

        // Query the balance of the account that is going to withdraw
        let balance_response: BalanceResponse = deps.querier.query_wasm_smart(
            token_addr.clone(),
            &cw20::Cw20QueryMsg::Balance {
                address: cfg.input_addr.to_string(),
            },
        )?;
        if balance_response.balance.is_zero() {
            return Err(ServiceError::ExecutionError(
                "Nothing to withdraw".to_string(),
            ));
        }

        // Calculate how much we are going to get when we withdraw
        let withdrawn_assets: Vec<Asset> = deps.querier.query_wasm_smart(
            cfg.pool_addr.clone(),
            &astroport_cw20_lp_token::pair::QueryMsg::Share {
                amount: balance_response.balance,
            },
        )?;

        // Create the withdraw and send messages
        let withdraw_msg = CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
            contract_addr: token_addr.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Send {
                contract: cfg.pool_addr.to_string(),
                amount: balance_response.balance,
                msg: to_json_binary(
                    &astroport_cw20_lp_token::pair::Cw20HookMsg::WithdrawLiquidity {
                        assets: vec![],
                    },
                )?,
            })?,
            funds: vec![],
        });

        // Send the withdrawn assets to the output account
        let withdrawn_coins = withdrawn_assets
            .into_iter()
            .map(|asset| asset.to_coin())
            .collect::<Result<Vec<Coin>, _>>()?;

        let send_msg = CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
            to_address: cfg.output_addr.to_string(),
            amount: withdrawn_coins,
        });

        Ok(vec![withdraw_msg, send_msg])
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
