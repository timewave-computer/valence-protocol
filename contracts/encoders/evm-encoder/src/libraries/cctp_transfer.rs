use alloy_sol_types::{SolCall, SolValue};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, StdError, StdResult, Uint256};
use valence_encoder_utils::libraries::{
    cctp_transfer::solidity_types::transferCall, updateConfigCall,
};
use valence_library_utils::{msg::ExecuteMsg, LibraryAccountType};

use crate::parse_address;

use super::{get_update_ownership_call, get_update_processor_call};

// We need to define a config and functions for this library as we don't have a CosmWasm equivalent
#[cw_serde]
/// Struct representing the library configuration.
pub struct LibraryConfig {
    /// The input address for the library.
    pub input_addr: LibraryAccountType,
    /// The mint recipient for the library. Bytes32 representation of the address in solidity.
    pub mint_recipient: Binary,
    /// Amount to transfer. Setting this to 0 will transfer the entire balance.
    pub amount: Uint256,
    /// The destination domain to transfer to
    pub destination_domain: u32,
    /// The address of the token to transfer.
    pub transfer_token: String,
    /// The address of the cctp token messenger.
    pub cctp_token_messenger: String,
}

#[cw_serde]
/// Enum representing the different function messages that can be sent.
pub enum FunctionMsgs {
    /// Message to transfer tokens.
    Transfer {},
}

type CctpTransferMsg = ExecuteMsg<FunctionMsgs, LibraryConfig>;

pub fn encode(msg: &Binary) -> StdResult<Vec<u8>> {
    // Extract the message from the binary and verify that it parses into a valid json for the library
    let msg: CctpTransferMsg = serde_json::from_slice(msg.as_slice()).map_err(|_| {
        StdError::generic_err("Message sent is not a valid message for this library!".to_string())
    })?;

    match msg {
        ExecuteMsg::ProcessFunction(function) => match function {
            FunctionMsgs::Transfer {} => Ok(transferCall::SELECTOR.to_vec()),
        },
        ExecuteMsg::UpdateConfig { new_config } => {
            // Parse addresses
            let input_account = parse_address(&new_config.input_addr.to_string()?)?;
            let cctp_transfer_messenger = parse_address(&new_config.cctp_token_messenger)?;
            let transfer_token = parse_address(&new_config.transfer_token)?;

            let mint_recipient_fixed: [u8; 32] = new_config
                .mint_recipient
                .as_slice()
                .try_into()
                .map_err(|e| {
                    StdError::generic_err(format!(
                        "Error converting mint recipient to fixed size: {}",
                        e
                    ))
                })?;

            // Build config struct
            let config =
                valence_encoder_utils::libraries::cctp_transfer::solidity_types::CCTPTransferConfig {
                    amount: alloy_primitives::U256::from_be_bytes(new_config.amount.to_be_bytes()),
                    mintRecipient: alloy_primitives::FixedBytes::<32>::from(mint_recipient_fixed),
                    inputAccount: input_account,
                    destinationDomain: new_config.destination_domain,
                    cctpTokenMessenger: cctp_transfer_messenger,
                    transferToken: transfer_token,
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
