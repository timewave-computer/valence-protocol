use alloy_sol_types::{SolCall, SolValue};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, StdError, StdResult, Uint256};

use valence_encoder_utils::libraries::{
    balancer_v2_swap::solidity_types::{multiSwapCall, swapCall},
    updateConfigCall, Bytes32Address, ToFixedBytes,
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
    /// The Balancer V2 Vault contract address.
    pub vault: String,
}

#[cw_serde]
/// Enum representing the different function messages that can be sent.
pub enum FunctionMsgs {
    /// Message to perform a single swap
    Swap {
        /// The pool ID to swap through
        pool_id: Bytes32Address,
        /// The address of the token to swap from
        token_in: String,
        /// The address of the token to swap to
        token_out: String,
        /// The amount to swap from the first token
        amount: Uint256,
        /// The minimum amount to receive from the last token
        min_amount_out: Uint256,
        /// The timeout for the swap in seconds (e.g. transaction not executed directly because it's in mempool)
        timeout: Uint256,
        /// Any additional data which the pool requires to perform the swap. Can be empty for all current Balancer pools.
        /// Allows pools to have more flexible logic in the future
        user_data: Binary,
    },

    /// Message to perform a multi-hop swap
    MultiSwap {
        /// List of pool IDs to swap through in order
        /// The first pool ID is the one to swap from, the last one is the one to swap to
        pool_ids: Vec<Bytes32Address>,
        /// List of tokens to swap through
        /// The first token is the one to swap from, the last one is the one to swap to
        tokens: Vec<String>,
        /// The amount to swap from the first token
        amount: Uint256,
        /// The minimum amount to receive from the last token
        min_amount_out: Uint256,
        /// The timeout for the swap in seconds (e.g. transaction not executed directly because it's in mempool)
        timeout: Uint256,
        /// Any additional data which each pool requires to perform the swap. Can be empty for all current Balancer pools.
        /// Allows pools to have more flexible logic in the future
        user_data: Vec<Binary>,
    },
}

type BalancerV2SwapMsg = ExecuteMsg<FunctionMsgs, LibraryConfig>;

pub fn encode(msg: &Binary) -> StdResult<Vec<u8>> {
    // Extract the message from the binary and verify that it parses into a valid json for the library
    let msg: BalancerV2SwapMsg = serde_json::from_slice(msg.as_slice()).map_err(|_| {
        StdError::generic_err("Message sent is not a valid message for this library!".to_string())
    })?;

    match msg {
        ExecuteMsg::ProcessFunction(function) => match function {
            FunctionMsgs::Swap {
                pool_id,
                token_in,
                token_out,
                amount,
                min_amount_out,
                timeout,
                user_data,
            } => {
                let swap_call = swapCall {
                    poolId: pool_id.to_fixed_bytes()?.into(),
                    tokenIn: parse_address(&token_in)?,
                    tokenOut: parse_address(&token_out)?,
                    amount: alloy_primitives::U256::from_be_bytes(amount.to_be_bytes()),
                    minAmountOut: alloy_primitives::U256::from_be_bytes(
                        min_amount_out.to_be_bytes(),
                    ),
                    timeout: alloy_primitives::U256::from_be_bytes(timeout.to_be_bytes()),
                    userData: user_data.to_vec().into(),
                };
                Ok(swap_call.abi_encode())
            }
            FunctionMsgs::MultiSwap {
                pool_ids,
                tokens,
                amount,
                min_amount_out,
                timeout,
                user_data,
            } => {
                let multi_swap_call = multiSwapCall {
                    poolIds: pool_ids
                        .iter()
                        .map(|pool| pool.to_fixed_bytes().map(|bytes| bytes.into()))
                        .collect::<Result<Vec<_>, _>>()?,
                    tokens: tokens
                        .iter()
                        .map(|token| parse_address(token))
                        .collect::<Result<Vec<_>, _>>()?,
                    userDataArray: user_data
                        .iter()
                        .map(|data| alloy_primitives::Bytes::from(data.to_vec()))
                        .collect::<Vec<_>>(),
                    amount: alloy_primitives::U256::from_be_bytes(amount.to_be_bytes()),
                    minAmountOut: alloy_primitives::U256::from_be_bytes(
                        min_amount_out.to_be_bytes(),
                    ),
                    timeout: alloy_primitives::U256::from_be_bytes(timeout.to_be_bytes()),
                };
                Ok(multi_swap_call.abi_encode())
            }
        },
        ExecuteMsg::UpdateConfig { new_config } => {
            // Parse addresses
            let input_account = parse_address(&new_config.input_addr.to_string()?)?;
            let output_account = parse_address(&new_config.output_addr.to_string()?)?;
            let vault_address = parse_address(&new_config.vault)?;

            // Build config struct
            let config =
             valence_encoder_utils::libraries::balancer_v2_swap::solidity_types::BalancerV2SwapConfig {
                inputAccount: input_account,
                outputAccount: output_account,
                vaultAddress: vault_address,
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
