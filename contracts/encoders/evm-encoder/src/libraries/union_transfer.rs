use alloy_primitives::Bytes;
use alloy_sol_types::{SolCall, SolValue};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, StdError, StdResult, Uint256};
use valence_encoder_utils::libraries::{
    union_transfer::solidity_types::transferCall, updateConfigCall,
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
    /// The recipient of the transfer on the destination chain (for bech32 addresses the bytes conversion of the entire bech32 address string).
    /// Example:
    pub recipient: Binary,
    /// Amount to transfer. Setting this to 0 will transfer the entire balance.
    pub amount: Uint256,
    /// Address of the zkGM contract.
    pub zk_gm: String,
    /// The address of the ERC20 token.
    pub transfer_token: String,
    /// The name of the transfer token.
    pub transfer_token_name: String,
    /// The symbol of the transfer token.
    pub transfer_token_symbol: String,
    /// The decimals of the transfer token.
    pub transfer_token_decimals: u8,
    /// The token requested in return on destination chain. Bytes conversion of the token denom / address for Native Cosmos Tokens / CW-20 tokens.
    pub quote_token: Binary,
    /// The amount of the quote token.
    pub quote_token_amount: Uint256,
    /// The path to unwrap the transfer token.
    pub transfer_token_unwrapping_path: Uint256,
    /// The channel ID for the transfer.
    pub channel_id: u32,
    /// The timeout for the transfer.
    pub timeout: u64,
    /// The protocol version for the transfer.
    pub protocol_version: u8,
}

#[cw_serde]
/// Enum representing the different function messages that can be sent.
pub enum FunctionMsgs {
    /// Message to transfer tokens.
    /// If the quote amount is not passed, the value in the config will be used.
    Transfer { quote_amount: Option<Uint256> },
}

type UnionTransferConfig = ExecuteMsg<FunctionMsgs, LibraryConfig>;

pub fn encode(msg: &Binary) -> StdResult<Vec<u8>> {
    // Extract the message from the binary and verify that it parses into a valid json for the library
    let msg: UnionTransferConfig = serde_json::from_slice(msg.as_slice()).map_err(|_| {
        StdError::generic_err("Message sent is not a valid message for this library!".to_string())
    })?;

    match msg {
        ExecuteMsg::ProcessFunction(function) => match function {
            FunctionMsgs::Transfer { quote_amount } => {
                let transfer_call = transferCall {
                    _quoteAmount: alloy_primitives::U256::from_be_bytes(
                        quote_amount.unwrap_or_default().to_be_bytes(),
                    ),
                };
                Ok(transfer_call.abi_encode())
            }
        },
        ExecuteMsg::UpdateConfig { new_config } => {
            // Parse addresses
            let input_account = parse_address(&new_config.input_addr.to_string()?)?;
            let zk_gm = parse_address(&new_config.zk_gm)?;
            // Convert to address to verify it is a valid address
            let transfer_token = parse_address(&new_config.transfer_token)?;

            // Build config struct
            let config =
                valence_encoder_utils::libraries::union_transfer::solidity_types::UnionTransferConfig {
                    protocolVersion: new_config.protocol_version,
                    transferTokenDecimals: new_config.transfer_token_decimals,
                    channelId: new_config.channel_id,
                    timeout: new_config.timeout,
                    inputAccount: input_account,
                    zkGM: zk_gm,
                    amount: alloy_primitives::U256::from_be_bytes(new_config.amount.to_be_bytes()),
                    quoteTokenAmount: alloy_primitives::U256::from_be_bytes(new_config.quote_token_amount.to_be_bytes()),
                    transferTokenUnwrappingPath: alloy_primitives::U256::from_be_bytes(new_config.transfer_token_unwrapping_path.to_be_bytes()),
                    recipient: new_config.recipient.to_vec().into(),
                    transferToken: Bytes::from(transfer_token.to_vec()),
                    quoteToken: new_config.quote_token.to_vec().into(),
                    transferTokenName: new_config.transfer_token_name,
                    transferTokenSymbol: new_config.transfer_token_symbol,
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
