#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use sp1_verifier::{Groth16Verifier, GROTH16_VK_BYTES};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new().add_attribute("action", "instantiate_verification_gateway"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> StdResult<Response> {
    unimplemented!("This contract does not handle any execute messages, only queries")
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::VerifyProof { vk, proof, inputs } => {
            to_json_binary(&verify_proof(&vk, proof, inputs)?)
        }
    }
}

fn verify_proof(vk: &Binary, proof: Binary, inputs: Binary) -> StdResult<bool> {
    // Get the VK as a String
    let sp1_vkey_hash = String::from_utf8(vk.to_vec()).map_err(|e| {
        cosmwasm_std::StdError::generic_err(format!("Failed to parse vk hash: {e}"))
    })?;

    Groth16Verifier::verify(
        proof.as_slice(),
        inputs.as_slice(),
        &sp1_vkey_hash,
        &GROTH16_VK_BYTES,
    )
    .map_err(|e| {
        cosmwasm_std::StdError::generic_err(format!(
            "Failed to verify SP1 proof with vk hash {sp1_vkey_hash}: {e}",
        ))
    })?;

    Ok(true)
}
