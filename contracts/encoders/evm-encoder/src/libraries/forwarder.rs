use alloy_sol_types::{SolCall, SolValue};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, StdError, StdResult};
use valence_encoder_utils::libraries::{
    forwarder::solidity_types::{forwardCall, IntervalType},
    updateConfigCall,
};
use valence_forwarder_library::msg::{ForwardingConstraints, FunctionMsgs};
use valence_library_utils::{msg::ExecuteMsg, LibraryAccountType};

use crate::parse_address;

use super::{get_update_ownership_call, get_update_processor_call};

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
            // Parse addresses
            let input_account = parse_address(&new_config.input_addr.to_string()?)?;
            let output_account = parse_address(&new_config.output_addr.to_string()?)?;

            // Convert forwarding configurations
            let forwarding_configs = new_config
                .forwarding_configs
                .iter()
                .map(|cfg| {
                    Ok(valence_encoder_utils::libraries::forwarder::solidity_types::ForwardingConfig {
                        tokenAddress: parse_address(&cfg.token_address)?,
                        maxAmount: cfg.max_amount,
                    })
                })
                .collect::<StdResult<Vec<_>>>()?;

            // Parse interval constraints
            let (interval_type, min_interval) =
                match new_config.forwarding_constraints.min_interval() {
                    Some(min_interval) => match min_interval {
                        cw_utils::Duration::Height(blocks) => (IntervalType::BLOCKS, *blocks),
                        cw_utils::Duration::Time(time) => (IntervalType::TIME, *time),
                    },
                    // If no interval is set, the value 0 is used which will be interpreted as no interval in the contract
                    None => (IntervalType::TIME, 0),
                };

            // Build config struct
            let config =
                valence_encoder_utils::libraries::forwarder::solidity_types::ForwarderConfig {
                    inputAccount: input_account,
                    outputAccount: output_account,
                    forwardingConfigs: forwarding_configs,
                    intervalType: interval_type,
                    minInterval: min_interval,
                };

            // Create the encoded call with the encoded config
            let call = updateConfigCall {
                _config: config.abi_encode().into(),
            };
            Ok(call.abi_encode())
        }
        ExecuteMsg::UpdateProcessor { processor } => get_update_processor_call(&processor),
        ExecuteMsg::UpdateOwnership(action) => get_update_ownership_call(action),
    }
}
