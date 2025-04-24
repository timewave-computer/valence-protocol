use alloy_sol_types::{SolCall, SolValue};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, StdError, StdResult, Uint256};

use valence_encoder_utils::libraries::{
    pancake_v3_position_manager::solidity_types::{createPositionCall, withdrawPositionCall},
    updateConfigCall,
};
use valence_library_utils::{msg::ExecuteMsg, LibraryAccountType};

use crate::{parse_address, validate_i24_value};

use super::{get_update_ownership_call, get_update_processor_call};

// We need to define a config and functions for this library as we don't have a CosmWasm equivalent
#[cw_serde]
/// Struct representing the library configuration.
pub struct LibraryConfig {
    /// The input address for the library.
    pub input_addr: LibraryAccountType,
    /// The output address for the library.
    pub output_addr: LibraryAccountType,
    /// The address of the position manager.
    pub position_manager: String,
    /// The address of the master chef.
    pub master_chef: String,
    /// The address of the token0.
    pub token0: String,
    /// The address of the token1.
    pub token1: String,
    /// The pool fee.
    pub pool_fee_bps: u32,
    /// The slippage used in basis points.
    pub slippage_bps: u16,
    /// The timeout for the transactions.
    pub timeout: Uint256,
}

#[cw_serde]
/// Enum representing the different function messages that can be sent.
pub enum FunctionMsgs {
    /// Message to create a position
    CreatePosition {
        tick_lower: i32, // Lower tick value, must be an i24 value, which rust doesn't have (i.e. -8388608 to 8388607)
        tick_upper: i32, // Upper tick value, must be an i24 value, which rust doesn't have (i.e. -8388608 to 8388607)
        amount0: Uint256,
        amount1: Uint256,
    },
    /// Message to withdraw a position, token_id represents the NFT id
    WithdrawPosition { token_id: Uint256 },
}

type PancakeV3PositionManagerMsg = ExecuteMsg<FunctionMsgs, LibraryConfig>;

pub fn encode(msg: &Binary) -> StdResult<Vec<u8>> {
    // Extract the message from the binary and verify that it parses into a valid json for the library
    let msg: PancakeV3PositionManagerMsg =
        serde_json::from_slice(msg.as_slice()).map_err(|_| {
            StdError::generic_err(
                "Message sent is not a valid message for this library!".to_string(),
            )
        })?;

    match msg {
        ExecuteMsg::ProcessFunction(function) => match function {
            FunctionMsgs::CreatePosition {
                tick_lower,
                tick_upper,
                amount0,
                amount1,
            } => {
                let create_position_call = createPositionCall {
                    tickLower: validate_i24_value(tick_lower)?,
                    tickUpper: validate_i24_value(tick_upper)?,
                    amount0: alloy_primitives::U256::from_be_bytes(amount0.to_be_bytes()),
                    amount1: alloy_primitives::U256::from_be_bytes(amount1.to_be_bytes()),
                };
                Ok(create_position_call.abi_encode())
            }
            FunctionMsgs::WithdrawPosition { token_id } => {
                let withdraw_position_call = withdrawPositionCall {
                    tokenId: alloy_primitives::U256::from_be_bytes(token_id.to_be_bytes()),
                };
                Ok(withdraw_position_call.abi_encode())
            }
        },
        ExecuteMsg::UpdateConfig { new_config } => {
            // Parse addresses
            let input_account = parse_address(&new_config.input_addr.to_string()?)?;
            let output_account = parse_address(&new_config.output_addr.to_string()?)?;
            let position_manager = parse_address(&new_config.position_manager)?;
            let master_chef = parse_address(&new_config.master_chef)?;
            let token0 = parse_address(&new_config.token0)?;
            let token1 = parse_address(&new_config.token1)?;

            // Build config struct
            let config =
             valence_encoder_utils::libraries::pancake_v3_position_manager::solidity_types::PancakeSwapV3PositionManagerConfig {
                inputAccount: input_account,
                outputAccount: output_account,
                positionManager: position_manager,
                masterChef: master_chef,
                token0,
                token1,
                poolFeeBps: new_config.pool_fee_bps,
                timeout: alloy_primitives::U256::from_be_bytes(new_config.timeout.to_be_bytes()),
                slippageBps: new_config.slippage_bps,
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
