use alloy_sol_types::SolValue;
use cosmwasm_std::{Binary, HexBinary, StdError, StdResult};
use valence_encoder_utils::processor::solidity_types::Callback;

pub fn decode(msg: &HexBinary) -> StdResult<Binary> {
    // Decode the callback message
    let callback = Callback::abi_decode(msg.as_slice(), true)
        .map_err(|e| StdError::generic_err(e.to_string()))?;

    // Create the expected callback message for the authorization contract
    let processor_callback: valence_authorization_utils::msg::InternalAuthorizationMsg =
        callback.into();

    // Wrap it in a Binary
    let binary = Binary::new(
        serde_json::to_vec(&processor_callback)
            .map_err(|e| StdError::generic_err(format!("Failed to serialize callback: {e}")))?,
    );

    Ok(binary)
}
