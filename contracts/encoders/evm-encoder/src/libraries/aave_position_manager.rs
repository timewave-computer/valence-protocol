use alloy_sol_types::{SolCall, SolValue};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, StdError, StdResult, Uint256};

use valence_encoder_utils::libraries::{
    aave_position_manager::solidity_types::{
        borrowCall, repayCall, repayWithATokensCall, supplyCall, withdrawCall,
    },
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
    /// The output address for the library.
    pub output_addr: LibraryAccountType,
    /// The AAVE pool address.
    pub pool: String,
    /// The supply asset token address.
    pub supply_asset: String,
    /// The borrow asset token address.
    pub borrow_asset: String,
    /// The referral code if applicable.
    pub referral_code: Option<u16>,
}

#[cw_serde]
/// Enum representing the different function messages that can be sent.
pub enum FunctionMsgs {
    /// Message to supply tokens.
    Supply { amount: Uint256 },
    /// Message to borrow tokens.
    Borrow { amount: Uint256 },
    /// Message to withdraw tokens.
    Withdraw { amount: Uint256 },
    /// Message to repay tokens.
    Repay { amount: Uint256 },
    /// Message to repay with aTokens.
    RepayWithATokens { amount: Uint256 },
}

type StargateTransferMsg = ExecuteMsg<FunctionMsgs, LibraryConfig>;

pub fn encode(msg: &Binary) -> StdResult<Vec<u8>> {
    // Extract the message from the binary and verify that it parses into a valid json for the library
    let msg: StargateTransferMsg = serde_json::from_slice(msg.as_slice()).map_err(|_| {
        StdError::generic_err("Message sent is not a valid message for this library!".to_string())
    })?;

    match msg {
        ExecuteMsg::ProcessFunction(function) => match function {
            FunctionMsgs::Supply { amount } => {
                let supply_call = supplyCall {
                    amount: alloy_primitives::U256::from_be_bytes(amount.to_be_bytes()),
                };
                Ok(supply_call.abi_encode())
            }
            FunctionMsgs::Borrow { amount } => {
                let borrow_call = borrowCall {
                    amount: alloy_primitives::U256::from_be_bytes(amount.to_be_bytes()),
                };
                Ok(borrow_call.abi_encode())
            }
            FunctionMsgs::Withdraw { amount } => {
                let withdraw_call = withdrawCall {
                    amount: alloy_primitives::U256::from_be_bytes(amount.to_be_bytes()),
                };
                Ok(withdraw_call.abi_encode())
            }
            FunctionMsgs::Repay { amount } => {
                let withdraw_call = repayCall {
                    amount: alloy_primitives::U256::from_be_bytes(amount.to_be_bytes()),
                };
                Ok(withdraw_call.abi_encode())
            }
            FunctionMsgs::RepayWithATokens { amount } => {
                let withdraw_call = repayWithATokensCall {
                    amount: alloy_primitives::U256::from_be_bytes(amount.to_be_bytes()),
                };
                Ok(withdraw_call.abi_encode())
            }
        },
        ExecuteMsg::UpdateConfig { new_config } => {
            // Parse addresses
            let input_account = parse_address(&new_config.input_addr.to_string()?)?;
            let output_account = parse_address(&new_config.output_addr.to_string()?)?;
            let pool_address = parse_address(&new_config.pool)?;
            let supply_asset = parse_address(&new_config.supply_asset)?;
            let borrow_asset = parse_address(&new_config.borrow_asset)?;

            // Build config struct
            let config =
             valence_encoder_utils::libraries::aave_position_manager::solidity_types::AavePositionManagerConfig {
                poolAddress: pool_address,
                inputAccount: input_account,
                outputAccount: output_account,
                supplyAsset: supply_asset,
                borrowAsset: borrow_asset,
                referralCode: new_config.referral_code.unwrap_or_default(),
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
