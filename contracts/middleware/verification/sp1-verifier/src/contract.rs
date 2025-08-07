#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError, StdResult,
};
use sp1_verifier::{Groth16Verifier, GROTH16_VK_BYTES};
use valence_verification_utils::verifier::{InstantiateMsg, QueryMsg};

use crate::state::DOMAIN_VK;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, StdError> {
    // Store the domain verification key
    DOMAIN_VK.save(deps.storage, &msg.domain_vk)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate_verifier")
        .add_attribute("domain_vk", msg.domain_vk.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(_deps: DepsMut, _env: Env, _info: MessageInfo, _msg: Empty) -> StdResult<Response> {
    unimplemented!("This contract does not handle any execute messages, only queries")
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Verify {
            vk,
            inputs,
            proof,
            payload,
        } => to_json_binary(&verify(deps, &vk, proof, inputs, payload)?),
        QueryMsg::DomainVk {} => {
            // Return the stored domain verification key
            let domain_vk = DOMAIN_VK.load(deps.storage)?;
            to_json_binary(&domain_vk)
        }
    }
}

fn verify(
    deps: Deps,
    vk: &Binary,
    proof: Binary,
    inputs: Binary,
    payload: Binary,
) -> StdResult<bool> {
    // Verify the proof using the provided Verifying Key
    verify_proof(vk, proof, inputs.clone())?;

    // Verify the domain proof using the provided Domain Verification Key
    let domain_vk = DOMAIN_VK.load(deps.storage)?;
    // The public inputs of the domain proof are the first 32 bytes of the public inputs of our program (coprocessor root)
    let domain_inputs = Binary::from(&inputs.as_slice()[0..32]);
    verify_proof(&domain_vk, payload, domain_inputs)?;

    // If verification was successful, return true
    Ok(true)
}

fn verify_proof(vk: &Binary, proof: Binary, inputs: Binary) -> StdResult<()> {
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
            "Failed to verify SP1 program proof with vk hash {sp1_vkey_hash}: {e}",
        ))
    })?;

    Ok(())
}
