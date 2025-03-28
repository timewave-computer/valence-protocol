use alloy_sol_types::{SolCall, SolValue};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, StdError, StdResult, Uint256};
use valence_encoder_utils::libraries::{
    stargate_transfer::solidity_types::transferCall, updateConfigCall, Bytes32Address, ToFixedBytes,
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
    /// The recipient for the library. Bytes32 representation of the address in solidity.
    pub recipient: Bytes32Address,
    /// The destination domain to transfer to
    pub destination_domain: u32,
    /// Address of Stargate Pool contract.
    pub stargate_address: String,
    /// The address of the token to transfer.
    pub transfer_token: String,
    /// Amount to transfer. Setting this to 0 will transfer the entire balance.
    pub amount: Uint256,
    /// Min amount to receive. If not provided it will automatically calculated
    pub min_amount_to_receive: Option<Uint256>,
    /// The refund address for the library. If not provided it will be set to the input address.
    pub refund_address: Option<String>,
    /// Extra options for the library.
    pub extra_options: Option<Binary>,
    /// Compose message for the library.
    pub compose_msg: Option<Binary>,
    /// Oft command for the library. This is used to set up Taxi/Bus mode. Not passed will default to Taxi mode.
    pub oft_cmd: Option<Binary>,
}

#[cw_serde]
/// Enum representing the different function messages that can be sent.
pub enum FunctionMsgs {
    /// Message to transfer tokens.
    Transfer {},
}

type StargateTransferMsg = ExecuteMsg<FunctionMsgs, LibraryConfig>;

pub fn encode(msg: &Binary) -> StdResult<Vec<u8>> {
    // Extract the message from the binary and verify that it parses into a valid json for the library
    let msg: StargateTransferMsg = serde_json::from_slice(msg.as_slice()).map_err(|_| {
        StdError::generic_err("Message sent is not a valid message for this library!".to_string())
    })?;

    match msg {
        ExecuteMsg::ProcessFunction(function) => match function {
            FunctionMsgs::Transfer {} => Ok(transferCall::SELECTOR.to_vec()),
        },
        ExecuteMsg::UpdateConfig { new_config } => {
            // Parse addresses
            let input_account = parse_address(&new_config.input_addr.to_string()?)?;
            let stargate_address = parse_address(&new_config.stargate_address)?;
            let transfer_token = parse_address(&new_config.transfer_token)?;
            let refund_address = new_config
                .refund_address
                .clone()
                .map(|x| parse_address(&x))
                .unwrap_or(Ok(input_account))?;

            // Build config struct
            let config =
                valence_encoder_utils::libraries::stargate_transfer::solidity_types::StargateTransferConfig {
                    recipient: new_config.recipient.to_fixed_bytes()?.into(),
                    inputAccount: input_account,
                    destinationDomain: new_config.destination_domain,
                    stargateAddress: stargate_address,
                    transferToken: transfer_token,
                    amount: alloy_primitives::U256::from_be_bytes(new_config.amount.to_be_bytes()),
                    minAmountToReceive: new_config.min_amount_to_receive.map(|x| {
                        alloy_primitives::U256::from_be_bytes(x.to_be_bytes())
                    }).unwrap_or_default(),
                    refundAddress: refund_address,
                    extraOptions: new_config.extra_options.unwrap_or_default().to_vec().into(),
                    composeMsg: new_config.compose_msg.unwrap_or_default().to_vec().into(),
                    oftCmd: new_config.oft_cmd.unwrap_or_default().to_vec().into(),
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
