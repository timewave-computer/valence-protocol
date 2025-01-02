use std::str::FromStr;

use alloy_primitives::Address;
use alloy_sol_types::SolCall;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, StdError, StdResult};
use valence_encoder_utils::libraries::forwarder::solidity_types::forwardCall;
use valence_forwarder_library::msg::{ForwardingConstraints, FunctionMsgs};
use valence_library_utils::{msg::ExecuteMsg, LibraryAccountType};

// We need to define a new config that will be used to encode the message because the one from the CW library is not the same as the one from the Solidity library
#[cw_serde]
/// Struct representing the library configuration.
pub struct LibraryConfig {
    /// The input address for the library.
    pub input_addr: LibraryAccountType,
    /// The output address for the library.
    pub output_addr: LibraryAccountType,
    /// The forwarding configurations for the library.
    pub forwarding_configs: Vec<ForwardingConfig>,
    /// The forwarding constraints for the library.
    pub forwarding_constraints: ForwardingConstraints,
}

#[cw_serde]
pub struct ForwardingConfig {
    /// The address of the token to forward.
    pub token_address: String,
    /// The maximum amount to forward.
    pub max_amount: u128,
}

type ForwarderMsg = ExecuteMsg<FunctionMsgs, LibraryConfig>;

pub fn encode(msg: &Binary) -> StdResult<Vec<u8>> {
    // Extract the message from the binary and verify that it parses into a valid json for the library
    let msg: ForwarderMsg = serde_json::from_slice(msg.as_slice()).map_err(|_| {
        StdError::generic_err("Message sent is not a valid message for this library!".to_string())
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
        ExecuteMsg::UpdateConfig { new_config } => {
            let input_account = Address::from_str(&new_config.input_addr.to_string()?).map_err(|e| {
                StdError::generic_err(format!("Error parsing input address: {}", e))
            })?;

            let output_account = Address::from_str(&new_config.output_addr.to_string()?).map_err(|e| {
                StdError::generic_err(format!("Error parsing output address: {}", e))
            })?;

            let forwarding_configs=


            let forwarder_config =
                valence_encoder_utils::libraries::forwarder::solidity_types::ForwarderConfig {
                    inputAccount: input_account,
                    outputAccount: output_account,
                    forwarding_configs: new_config
                        .forwarding_configs
                        .iter()
                        .map(|fwd_cfg| {
                            Ok(valence_encoder_utils::libraries::forwarder::solidity_types::ForwardingConfig {
                                tokenAddress: Address::from_str(&fwd_cfg.token_address)
                                    .map_err(|e| StdError::generic_err(format!("Error parsing token address: {}", e)))?,
                                maxAmount: fwd_cfg.max_amount,
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?
                    intervalType: ,
                    minInterval: todo!(),
                };
        }
        ExecuteMsg::UpdateProcessor { .. } => todo!(),
        ExecuteMsg::UpdateOwnership(..) => todo!(),
    }
}
