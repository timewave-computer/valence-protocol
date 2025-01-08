use alloy_sol_types::{sol, SolCall};
use cosmwasm_std::{Binary, StdError, StdResult};
use valence_forwarder_library::msg::{Config, FunctionMsgs};
use valence_library_utils::msg::ExecuteMsg;

// Define our forwarder library API
// TODO: Add the updateConfig, updateProcessor and updateOwnership once we get to the library implementation
sol! {
    function forward() external view;
}

pub fn encode(msg: &Binary) -> StdResult<Vec<u8>> {
    // Extract the message from the binary and verify that it parses into a valid json for the library
    let msg: ExecuteMsg<FunctionMsgs, Config> =
        serde_json::from_slice(msg.as_slice()).map_err(|_| {
            StdError::generic_err(
                "Message sent is not a valid message for this library!".to_string(),
            )
        })?;

    match msg {
        ExecuteMsg::ProcessFunction(function) => {
            match function {
                FunctionMsgs::Forward {} => {
                    // This uses the forward() function definition from the sol! macro
                    // Returns the function selector (first 4 bytes of the keccak256 hash) which is the only thing we need to call the function because it has no params
                    // No need to ABI encode the selector as it is already the ABI encoded function signature
                    Ok(forwardCall::SELECTOR.to_vec())
                }
            }
        }
        // TODO: Decide what is going to be allowed to be updated from here in the library
        ExecuteMsg::UpdateConfig { .. } => todo!(),
        ExecuteMsg::UpdateProcessor { .. } => todo!(),
        ExecuteMsg::UpdateOwnership(..) => todo!(),
    }
}
