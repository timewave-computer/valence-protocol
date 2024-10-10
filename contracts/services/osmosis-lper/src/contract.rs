#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use valence_service_utils::{
    error::ServiceError,
    msg::{ExecuteMsg, InstantiateMsg},
};

use crate::{
    msg::{ActionsMsgs, QueryMsg},
    valence_service_integration::{Config, OptionalServiceConfig, ServiceConfig},
};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg<ServiceConfig>,
) -> Result<Response, ServiceError> {
    valence_service_base::instantiate(deps, CONTRACT_NAME, CONTRACT_VERSION, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<ActionsMsgs, OptionalServiceConfig>,
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

mod execute {
    use cosmwasm_std::{DepsMut, Env, MessageInfo};
    use valence_service_utils::error::ServiceError;

    use crate::valence_service_integration::{Config, OptionalServiceConfig};

    pub fn update_config(
        deps: &DepsMut,
        _env: Env,
        _info: MessageInfo,
        config: &mut Config,
        new_config: OptionalServiceConfig,
    ) -> Result<(), ServiceError> {
        new_config.update_config(deps, config)
    }
}

mod actions {
    use cosmwasm_std::{
        coin, Coin, CosmosMsg, Decimal, DepsMut, Env, Fraction, MessageInfo, Response, StdError,
        StdResult, Uint128,
    };
    use osmosis_std::{cosmwasm_to_proto_coins, types::osmosis::gamm::v1beta1::GammQuerier};

    use valence_osmosis_utils::utils::{
        get_pool_ratio, get_provide_liquidity_msg, get_provide_ss_liquidity_msg, query_pool,
        query_pool_asset_balance,
    };
    use valence_service_utils::{error::ServiceError, execute_on_behalf_of};

    use crate::{msg::ActionsMsgs, valence_service_integration::Config};

    pub fn process_action(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: ActionsMsgs,
        cfg: Config,
    ) -> Result<Response, ServiceError> {
        println!("processing osmosis liquid pooler action: {:?}", msg);
        match msg {
            ActionsMsgs::ProvideDoubleSidedLiquidity {} => {
                provide_double_sided_liquidity(deps, cfg)
            }
            ActionsMsgs::ProvideSingleSidedLiquidity { asset, limit } => {
                provide_single_sided_liquidity(deps, cfg, asset, limit)
            }
        }
    }

    fn provide_single_sided_liquidity(
        deps: DepsMut,
        cfg: Config,
        asset: String,
        limit: Uint128,
    ) -> Result<Response, ServiceError> {
        // first we assert the input account balance
        let input_acc_asset_bal = query_pool_asset_balance(&deps, cfg.input_addr.as_str(), &asset)?;

        deps.api.debug(
            format!(
                "input account pool asset balance: {:?}",
                input_acc_asset_bal
            )
            .as_str(),
        );

        let provision_amount = if input_acc_asset_bal.amount > limit {
            limit
        } else {
            input_acc_asset_bal.amount
        };

        let share_out_amt = calculate_share_out_amt_swap(
            &deps,
            cfg.lp_config.pool_id,
            vec![coin(provision_amount.u128(), asset.to_string())],
        )?;

        deps.api
            .debug(&format!("share out amount: {share_out_amt}"));

        let liquidity_provision_msg = get_provide_ss_liquidity_msg(
            cfg.input_addr.as_str(),
            cfg.lp_config.pool_id,
            coin(provision_amount.u128(), asset),
            share_out_amt,
        )?;

        deps.api.debug(&format!(
            "liquidity provision msg: {:?}",
            liquidity_provision_msg
        ));

        let delegated_input_acc_msgs =
            execute_on_behalf_of(vec![liquidity_provision_msg], &cfg.input_addr.clone())?;
        deps.api
            .debug(format!("delegated lp msg: {:?}", delegated_input_acc_msgs).as_str());

        Ok(Response::default().add_message(delegated_input_acc_msgs))
    }

    fn provide_double_sided_liquidity(
        deps: DepsMut,
        cfg: Config,
    ) -> Result<Response, ServiceError> {
        // first we assert the input account balances
        let bal_asset_1 = query_pool_asset_balance(
            &deps,
            cfg.input_addr.as_str(),
            cfg.lp_config.pool_asset_1.as_str(),
        )?;
        let bal_asset_2 = query_pool_asset_balance(
            &deps,
            cfg.input_addr.as_str(),
            cfg.lp_config.pool_asset_2.as_str(),
        )?;

        deps.api
            .debug(format!("input account pool asset 1 balance: {:?}", bal_asset_1).as_str());
        deps.api
            .debug(format!("input account pool asset 2 balance: {:?}", bal_asset_2).as_str());

        let pool_response = query_pool(&deps, cfg.lp_config.pool_id)?;

        let pool_ratio = get_pool_ratio(
            pool_response,
            cfg.lp_config.pool_asset_1.clone(),
            cfg.lp_config.pool_asset_2.clone(),
        )?;

        let (asset_1_provision_amt, asset_2_provision_amt) =
            calculate_provision_amounts(bal_asset_1.amount, bal_asset_2.amount, pool_ratio)?;

        let provision_coins = vec![
            Coin {
                denom: cfg.lp_config.pool_asset_1.clone(),
                amount: asset_1_provision_amt,
            },
            Coin {
                denom: cfg.lp_config.pool_asset_2.clone(),
                amount: asset_2_provision_amt,
            },
        ];

        let share_out_amt =
            calculate_share_out_amt_no_swap(&deps, cfg.lp_config.pool_id, provision_coins.clone())?;

        let liquidity_provision_msg: CosmosMsg = get_provide_liquidity_msg(
            cfg.input_addr.as_str(),
            cfg.lp_config.pool_id,
            provision_coins,
            share_out_amt,
        )?;

        let delegated_input_acc_msgs =
            execute_on_behalf_of(vec![liquidity_provision_msg], &cfg.input_addr.clone())?;
        deps.api
            .debug(format!("delegated lp msg: {:?}", delegated_input_acc_msgs).as_str());

        Ok(Response::default().add_message(delegated_input_acc_msgs))
    }

    fn calculate_share_out_amt_no_swap(
        deps: &DepsMut,
        pool_id: u64,
        coins_in: Vec<Coin>,
    ) -> StdResult<String> {
        let gamm_querier = GammQuerier::new(&deps.querier);
        let resp = gamm_querier
            .calc_join_pool_no_swap_shares(pool_id, cosmwasm_to_proto_coins(coins_in))?;
        Ok(resp.shares_out)
    }

    fn calculate_share_out_amt_swap(
        deps: &DepsMut,
        pool_id: u64,
        coin_in: Vec<Coin>,
    ) -> StdResult<String> {
        let gamm_querier = GammQuerier::new(&deps.querier);
        let resp = gamm_querier.calc_join_pool_shares(pool_id, cosmwasm_to_proto_coins(coin_in))?;

        Ok(resp.share_out_amount)
    }

    fn calculate_provision_amounts(
        asset_1_bal: Uint128,
        asset_2_bal: Uint128,
        pool_ratio: Decimal,
    ) -> StdResult<(Uint128, Uint128)> {
        // first we assume that we are going to provide all of asset_1 and up to all of asset_2
        // then we get the expected amount of asset_2 tokens to provide
        let expected_asset_2_provision_amt = asset_1_bal
            .checked_multiply_ratio(pool_ratio.numerator(), pool_ratio.denominator())
            .map_err(|e| StdError::generic_err(e.to_string()))?;

        // then we check if the expected amount of asset_2 tokens is greater than the available balance
        if expected_asset_2_provision_amt > asset_2_bal {
            // if it is, we calculate the amount of asset_1 tokens to provide
            let asset_1_provision_amt = asset_2_bal
                .checked_multiply_ratio(pool_ratio.denominator(), pool_ratio.numerator())
                .map_err(|e| StdError::generic_err(e.to_string()))?;
            Ok((asset_1_provision_amt, asset_2_bal))
        } else {
            // if it is not, we provide all of asset_1 and the expected amount of asset_2
            Ok((asset_1_bal, expected_asset_2_provision_amt))
        }
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
    }
}
