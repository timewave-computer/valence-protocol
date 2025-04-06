use crate::parse_address;
use alloy_sol_types::SolCall;
use cosmwasm_std::{StdError, StdResult};
use valence_encoder_utils::libraries::{
    renounceOwnershipCall, transferOwnershipCall, updateProcessorCall,
};

pub mod cctp_transfer;
pub mod forwarder;
pub mod stargate_transfer;

// Function calls that are common to all libraries

/// Gets the call to update the processor of the library
pub fn get_update_processor_call(processor_addr: &str) -> StdResult<Vec<u8>> {
    let processor_addr = parse_address(processor_addr)?;

    let call = updateProcessorCall {
        _processor: processor_addr,
    };
    Ok(call.abi_encode())
}

/// Gets the call to update the ownership of the library. The Ownable solidity contract does only implement the transferOwnership and renounceOwnership functions.
pub fn get_update_ownership_call(action: cw_ownable::Action) -> StdResult<Vec<u8>> {
    match action {
        cw_ownable::Action::TransferOwnership { new_owner, .. } => {
            let new_owner_addr = parse_address(new_owner.as_str())?;
            let call = transferOwnershipCall {
                newOwner: new_owner_addr,
            };
            Ok(call.abi_encode())
        }
        cw_ownable::Action::RenounceOwnership => Ok(renounceOwnershipCall::SELECTOR.to_vec()),
        cw_ownable::Action::AcceptOwnership => Err(StdError::generic_err(
            "AcceptOwnership is not supported".to_string(),
        )),
    }
}
