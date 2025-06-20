use alloy_sol_types::{SolCall, SolValue};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, StdError, StdResult, Uint256};
use valence_encoder_utils::libraries::{
    ibc_eureka_transfer::solidity_types::{lombardTransferCall, transferCall},
    updateConfigCall,
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
    /// The recipient of the transfer (bech32 address on the IBC side).
    pub recipient: String,
    /// Amount to transfer. Setting this to 0 will transfer the entire balance.
    pub amount: Uint256,
    /// Min amount out to receive on the destination domain. Setting this to 0 will take the same value as amount.
    /// This is only used for Lombard transfers.
    pub min_amount_out: Uint256,
    /// Address of the Eureka Handler.
    pub eureka_handler_address: String,
    /// The address of the ERC20 token to transfer.
    pub transfer_token: String,
    /// The source client identifier (e.g. cosmoshub-0).
    pub source_client: String,
    /// Time out for the transfer in seconds.
    pub timeout: u64,
}

#[cw_serde]
pub struct Fees {
    /// The relay fee for the transfer.
    pub relay_fee: Uint256,
    /// The recipient of the relay fee.
    pub relay_fee_recipient: String,
    /// The expiry time for the quote in seconds.
    pub quote_expiry: u64,
}

#[cw_serde]
/// Enum representing the different function messages that can be sent.
pub enum FunctionMsgs {
    /// Message to transfer tokens.
    Transfer { fees: Fees, memo: String },
    /// Message to lombard transfer tokens.
    LombardTransfer { fees: Fees, memo: String },
}

type IBCEurekaTransferConfig = ExecuteMsg<FunctionMsgs, LibraryConfig>;

pub fn encode(msg: &Binary) -> StdResult<Vec<u8>> {
    // Extract the message from the binary and verify that it parses into a valid json for the library
    let msg: IBCEurekaTransferConfig = serde_json::from_slice(msg.as_slice()).map_err(|_| {
        StdError::generic_err("Message sent is not a valid message for this library!".to_string())
    })?;

    match msg {
        ExecuteMsg::ProcessFunction(function) => {
            match function {
                FunctionMsgs::Transfer { fees, memo } => {
                    let fees = valence_encoder_utils::libraries::ibc_eureka_transfer::solidity_types::Fees {
                                relayFee: alloy_primitives::U256::from_be_bytes(fees.relay_fee.to_be_bytes()),
                                relayFeeRecipient: parse_address(&fees.relay_fee_recipient)?,
                                quoteExpiry: fees.quote_expiry,
                            };
                    let transfer_call = transferCall { fees, memo };
                    Ok(transfer_call.abi_encode())
                }
                FunctionMsgs::LombardTransfer { fees, memo } => {
                    let fees = valence_encoder_utils::libraries::ibc_eureka_transfer::solidity_types::Fees {
                        relayFee: alloy_primitives::U256::from_be_bytes(fees.relay_fee.to_be_bytes()),
                        relayFeeRecipient: parse_address(&fees.relay_fee_recipient)?,
                        quoteExpiry: fees.quote_expiry,
                    };
                    let lombard_transfer_call = lombardTransferCall { fees, memo };
                    Ok(lombard_transfer_call.abi_encode())
                }
            }
        }
        ExecuteMsg::UpdateConfig { new_config } => {
            // Parse addresses
            let input_account = parse_address(&new_config.input_addr.to_string()?)?;
            let eureka_handler = parse_address(&new_config.eureka_handler_address)?;
            let transfer_token = parse_address(&new_config.transfer_token)?;

            // Build config struct
            let config =
                valence_encoder_utils::libraries::ibc_eureka_transfer::solidity_types::IBCEurekaTransferConfig {
                    amount: alloy_primitives::U256::from_be_bytes(new_config.amount.to_be_bytes()),
                    minAmountOut: alloy_primitives::U256::from_be_bytes(new_config.min_amount_out.to_be_bytes()),
                    transferToken: transfer_token,
                    inputAccount: input_account,
                    recipient: new_config.recipient,
                    sourceClient: new_config.source_client,
                    timeout: new_config.timeout,
                    eurekaHandler: eureka_handler,
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
