use cosmos_sdk_proto::{cosmos::base::v1beta1::Coin, traits::MessageExt};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use neutron_sdk::proto_types::osmosis::tokenfactory::v1beta1::{MsgCreateDenom, MsgMint};
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

fn mint_msg(sender: String, recipient: String, amount: u128, denom: String) -> CosmosMsg {
    let msg_mint = MsgMint {
        sender,
        amount: Some(Coin {
            denom,
            amount: amount.to_string(),
        }),
        mint_to_address: recipient,
    };
    // TODO: Change to AnyMsg instead of Stargate when we can test with CW 2.0 (They are the same, just a rename)
    #[allow(deprecated)]
    CosmosMsg::Stargate {
        type_url: "/osmosis.tokenfactory.v1beta1.MsgMint".to_string(),
        value: Binary::from(msg_mint.to_bytes().unwrap()),
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
    use std::collections::BTreeMap;

    use cosmwasm_std::{
        coin, ensure, Addr, BankMsg, DepsMut, Env, MessageInfo, Response, StdError, Uint128,
    };
    use valence_service_utils::{error::ServiceError, execute_on_behalf_of};

    use crate::{
        msg::{ActionMsgs, Config},
        state::TOKENIZED_DENOM,
    };

    use super::mint_msg;

    pub fn process_action(
        deps: DepsMut,
        env: Env,
        _info: MessageInfo,
        msg: ActionMsgs,
        cfg: Config,
    ) -> Result<Response, ServiceError> {
        match msg {
            ActionMsgs::Tokenize { sender } => try_tokenize(deps, env, sender, cfg),
        }
    }

    fn try_tokenize(
        deps: DepsMut,
        env: Env,
        sender: String,
        cfg: Config,
    ) -> Result<Response, ServiceError> {
        // build a map of sender balances relevant to tokenizing the share.
        // denom -> balance
        let mut sender_balances_map: BTreeMap<String, Uint128> = BTreeMap::new();
        for (denom, _) in cfg.input_denoms.iter() {
            let sender_balance = deps.querier.query_balance(sender.to_string(), denom)?;
            sender_balances_map.insert(sender_balance.denom, sender_balance.amount);
        }

        let mut output_shares = Uint128::zero();
        for (denom, sender_balance) in sender_balances_map.iter() {
            // we check what amount of this denom is needed to buy a share
            let price = cfg.input_denoms.get(denom).unwrap();
            // then we calculate how many shares the sender can buy with this denom amount
            let share_amount = sender_balance.checked_div(*price).unwrap();

            // if output shares is uninitialized, set it to the share amount
            // otherwise validate that currently observed denom meets the requirement
            if output_shares.is_zero() {
                output_shares = share_amount;
            } else {
                ensure!(
                    share_amount == output_shares,
                    StdError::generic_err("Invalid share amount")
                );
            }
        }

        let mut price_tokens = vec![];
        for (denom, price) in cfg.input_denoms.iter() {
            let amount_to_send = price.checked_mul(output_shares).unwrap();
            price_tokens.push(coin(amount_to_send.u128(), denom));
        }

        let transfer_msg = BankMsg::Send {
            to_address: cfg.output_addr.to_string(),
            amount: price_tokens,
        };

        let tf_denom = TOKENIZED_DENOM.load(deps.storage)?;

        let mint_msg = mint_msg(
            env.contract.address.to_string(),
            sender.to_string(),
            output_shares.u128(),
            tf_denom,
        );

        let delegated_msgs = execute_on_behalf_of(
            vec![transfer_msg.into(), mint_msg],
            &Addr::unchecked(sender),
        )?;

        Ok(Response::new()
            .add_attribute("method", "tokenize")
            .add_message(delegated_msgs))
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
