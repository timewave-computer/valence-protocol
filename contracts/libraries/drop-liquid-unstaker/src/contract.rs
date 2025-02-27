#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use valence_library_utils::{
    error::LibraryError,
    msg::{ExecuteMsg, InstantiateMsg},
};

use crate::msg::{Config, FunctionMsgs, LibraryConfig, LibraryConfigUpdate, QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg<LibraryConfig>,
) -> Result<Response, LibraryError> {
    valence_library_base::instantiate(deps, CONTRACT_NAME, CONTRACT_VERSION, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<FunctionMsgs, LibraryConfigUpdate>,
) -> Result<Response, LibraryError> {
    valence_library_base::execute(
        deps,
        env,
        info,
        msg,
        functions::process_function,
        execute::update_config,
    )
}

mod functions {
    use cosmwasm_std::{
        ensure_eq, to_json_binary, CosmosMsg, DepsMut, Empty, Env, MessageInfo, Response,
        StdResult, WasmMsg,
    };
    use cw721::msg::{Cw721ExecuteMsg, OwnerOfResponse};
    use valence_library_utils::{error::LibraryError, execute_on_behalf_of};
    use valence_liquid_staking_utils::drop::{LiquidStakerExecuteMsg, ReceiveNftMsg};

    use crate::msg::{Config, FunctionMsgs};

    pub fn process_function(
        mut deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: FunctionMsgs,
        cfg: Config,
    ) -> Result<Response, LibraryError> {
        match msg {
            FunctionMsgs::LiquidUnstake {} => {
                // We will query the balance of the input address and unstake the entire balance
                // to the liquid unstaker address.
                let balance = deps
                    .querier
                    .query_balance(cfg.input_addr.clone(), cfg.denom)?;

                // Check that input account has something to unstake
                if balance.amount.is_zero() {
                    return Err(LibraryError::ExecutionError(
                        "No funds to unstake".to_string(),
                    ));
                }

                let unstake_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cfg.liquid_unstaker_addr.to_string(),
                    msg: to_json_binary(&LiquidStakerExecuteMsg::Unbond {})?,
                    funds: vec![balance],
                });

                // Wrap the unstake msg to be executed on behalf of the input account
                let input_account_msgs = execute_on_behalf_of(vec![unstake_msg], &cfg.input_addr)?;

                Ok(Response::new()
                    .add_message(input_account_msgs)
                    .add_attribute("method", "liquid_unstake"))
            }
            FunctionMsgs::Claim { token_id } => {
                // Verify that voucher belongs to input account
                verify_voucher_ownership(
                    deps.branch(),
                    cfg.voucher_addr.to_string(),
                    &cfg,
                    token_id.clone(),
                )?;

                // Create the claim message
                let claim_msg = create_send_nft_with_hook_msg(
                    cfg.voucher_addr.to_string(),
                    cfg.withdrawal_manager_addr.to_string(),
                    token_id,
                    cfg.output_addr.to_string(),
                )?;

                // Wrap the claim msg to be executed on behalf of the input account
                let input_account_msgs = execute_on_behalf_of(vec![claim_msg], &cfg.input_addr)?;

                Ok(Response::new()
                    .add_message(input_account_msgs)
                    .add_attribute("method", "claim"))
            }
        }
    }

    fn verify_voucher_ownership(
        deps: DepsMut,
        voucher_contract: String,
        cfg: &Config,
        token_id: String,
    ) -> Result<(), LibraryError> {
        // Check that the token_id belongs to the input account
        let owner_response: OwnerOfResponse = deps
            .querier
            .query_wasm_smart(
                voucher_contract.clone(),
                &cw721_base::msg::QueryMsg::OwnerOf {
                    token_id: token_id.clone(),
                    include_expired: None,
                },
            )
            .map_err(|_| LibraryError::ExecutionError("Voucher does not exist".to_string()))?;

        ensure_eq!(
            owner_response.owner,
            cfg.input_addr.to_string(),
            LibraryError::ExecutionError("Voucher does not belong to input account".to_string())
        );

        Ok(())
    }

    fn create_send_nft_with_hook_msg(
        nft_contract_addr: String,
        receiving_contract_addr: String,
        token_id: String,
        receiver: String,
    ) -> StdResult<CosmosMsg> {
        // Create the hook message to be sent along with the NFT.
        let hook_msg = to_json_binary(&ReceiveNftMsg::Withdraw {
            receiver: Some(receiver),
        })?;

        // Build the SendNft message for the CW721 contract.
        let send_nft_msg: Cw721ExecuteMsg<Empty, Empty, Empty> = Cw721ExecuteMsg::SendNft {
            contract: receiving_contract_addr,
            token_id,
            msg: hook_msg,
        };

        let exec_msg = WasmMsg::Execute {
            contract_addr: nft_contract_addr,
            msg: to_json_binary(&send_nft_msg)?,
            funds: vec![],
        };

        Ok(exec_msg.into())
    }
}

mod execute {
    use cosmwasm_std::{DepsMut, Env, MessageInfo};
    use valence_library_utils::error::LibraryError;

    use crate::msg::LibraryConfigUpdate;

    pub fn update_config(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        new_config: LibraryConfigUpdate,
    ) -> Result<(), LibraryError> {
        new_config.update_config(deps)
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => {
            to_json_binary(&valence_library_base::get_ownership(deps.storage)?)
        }
        QueryMsg::GetProcessor {} => {
            to_json_binary(&valence_library_base::get_processor(deps.storage)?)
        }
        QueryMsg::GetLibraryConfig {} => {
            let config: Config = valence_library_base::load_config(deps.storage)?;
            to_json_binary(&config)
        }
        QueryMsg::GetRawLibraryConfig {} => {
            let raw_config: LibraryConfig =
                valence_library_utils::raw_config::query_raw_library_config(deps.storage)?;
            to_json_binary(&raw_config)
        }
    }
}
