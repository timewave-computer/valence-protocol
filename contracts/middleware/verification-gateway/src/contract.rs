#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw_ownable::{assert_owner, initialize_owner, update_ownership};
use sp1_verifier::{Groth16Verifier, GROTH16_VK_BYTES};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::DOMAIN_VK,
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Store the domain verification key
    DOMAIN_VK.save(deps.storage, &msg.domain_vk)?;

    // Set up owner
    initialize_owner(deps.storage, deps.api, Some(&msg.owner))?;

    Ok(Response::new().add_attribute("action", "instantiate_verification_gateway"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateDomainVk { domain_vk } => {
            assert_owner(deps.storage, &info.sender)?;
            // Update the domain verification key
            DOMAIN_VK.save(deps.storage, &domain_vk)?;

            Ok(Response::new()
                .add_attribute("action", "update_domain_vk")
                .add_attribute("new_domain_vk", domain_vk.to_string()))
        }
        ExecuteMsg::UpdateOwnership(action) => {
            let ownership = update_ownership(deps, &env.block, &info.sender, action)?;
            Ok(Response::new().add_attributes(ownership.into_attributes()))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::VerifyProof { vk, proof, inputs } => {
            to_json_binary(&verify_proof(&vk, proof, inputs)?)
        }
        QueryMsg::VerifyDomainProof {
            domain_proof,
            domain_inputs,
        } => {
            // Retrieve the stored domain verification key
            let domain_vk = DOMAIN_VK.load(deps.storage)?;

            to_json_binary(&verify_proof(&domain_vk, domain_proof, domain_inputs)?)
        }
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
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
