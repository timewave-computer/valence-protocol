use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, Deps, StdError, StdResult, WasmMsg};
use valence_account_utils::ica::{IcaState, QueryMsg};
use valence_ibc_utils::types::ProtobufAny;

/// Helper function to execute proto messages using the Valence interchain account
pub fn execute_on_behalf_of(msgs: Vec<ProtobufAny>, account: &Addr) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: account.to_string(),
        msg: to_json_binary(&valence_account_utils::ica::ExecuteMsg::ExecuteIcaMsg { msgs })?,
        funds: vec![],
    }))
}

/// Helper to get the remote address of the ICA after verifying it's created
pub fn get_remote_ica_address(deps: Deps, contract_addr: &str) -> StdResult<String> {
    let ica_state: IcaState = deps
        .querier
        .query_wasm_smart(contract_addr, &QueryMsg::IcaState {})?;

    match ica_state {
        IcaState::Created(ica_information) => Ok(ica_information.address),
        _ => Err(StdError::generic_err("ICA not created")),
    }
}
