use alloy_sol_types::{SolCall, SolValue};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, StdError, StdResult, Uint256};
use valence_encoder_utils::libraries::{
    standard_bridge_transfer::solidity_types::transferCall, updateConfigCall,
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
    /// The recipient of the transfer.
    pub recipient: String,
    /// Amount to transfer. Setting this to 0 will transfer the entire balance.
    pub amount: Uint256,
    /// Address of the L1 or L2 Standard Bridge.
    pub standard_bridge_address: String,
    /// The address of the ERC20 token to transfer. For ETH, use the zero address.
    pub transfer_token: String,
    /// The address of the remote token. ERC20 representation of the token on the other chain. For ETH, use the zero address.
    pub remote_token: String,
    /// Gas to use to complete the transfer on the receiving side. Used for sequencers/relayers.
    pub gas_limit: u32,
    /// Extra data to sent with the transaction. Will be emitted to identify the transaction.
    pub extra_data: Option<Binary>,
}

#[cw_serde]
/// Enum representing the different function messages that can be sent.
pub enum FunctionMsgs {
    /// Message to transfer tokens.
    Transfer {},
}

type StandardBridgeTransferMsg = ExecuteMsg<FunctionMsgs, LibraryConfig>;

pub fn encode(msg: &Binary) -> StdResult<Vec<u8>> {
    // Extract the message from the binary and verify that it parses into a valid json for the library
    let msg: StandardBridgeTransferMsg = serde_json::from_slice(msg.as_slice()).map_err(|_| {
        StdError::generic_err("Message sent is not a valid message for this library!".to_string())
    })?;

    match msg {
        ExecuteMsg::ProcessFunction(function) => match function {
            FunctionMsgs::Transfer {} => Ok(transferCall::SELECTOR.to_vec()),
        },
        ExecuteMsg::UpdateConfig { new_config } => {
            // Parse addresses
            let input_account = parse_address(&new_config.input_addr.to_string()?)?;
            let recipient = parse_address(&new_config.recipient)?;
            let standard_bridge = parse_address(&new_config.standard_bridge_address)?;
            let transfer_token = parse_address(&new_config.transfer_token)?;
            let remote_token = parse_address(&new_config.remote_token)?;

            // Build config struct
            let config =
                valence_encoder_utils::libraries::standard_bridge_transfer::solidity_types::StandardBridgeTransferConfig {
                    amount: alloy_primitives::U256::from_be_bytes(new_config.amount.to_be_bytes()),
                    inputAccount: input_account,
                    recipient,
                    standardBridge: standard_bridge,
                    token: transfer_token,
                    remoteToken: remote_token,
                    minGasLimit: new_config.gas_limit,
                    extraData: new_config
                        .extra_data
                        .map(|data| data.to_vec().into())
                        .unwrap_or_default(),
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
