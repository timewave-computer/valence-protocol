use std::collections::BTreeMap;

use cosmwasm_std::{coin, CosmosMsg, Env, IbcTimeout, IbcTimeoutBlock, StdResult, Timestamp};

use crate::types::PacketForwardMiddlewareConfig;

// Default timeout for IbcTransfer is 600 seconds
const DEFAULT_TIMEOUT_TIMESTAMP: u64 = 600;

#[allow(clippy::too_many_arguments)]
pub fn ibc_send_message(
    env: Env,
    channel: String,
    to: String,
    denom: String,
    amount: u128,
    memo: String,
    timeout_height: Option<u64>,
    timeout_timestamp: Option<u64>,
    _denom_to_pfm_map: BTreeMap<String, PacketForwardMiddlewareConfig>,
) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Ibc(cosmwasm_std::IbcMsg::Transfer {
        channel_id: channel,
        to_address: to,
        amount: coin(amount, denom),
        timeout: match (timeout_height, timeout_timestamp) {
            (Some(height), None) => IbcTimeout::with_block(IbcTimeoutBlock {
                revision: 2,
                height,
            }),
            (None, Some(timestamp)) => IbcTimeout::with_timestamp(Timestamp::from_nanos(timestamp)),
            (Some(height), Some(timestamp)) => IbcTimeout::with_both(
                IbcTimeoutBlock {
                    revision: 2,
                    height,
                },
                Timestamp::from_nanos(timestamp),
            ),
            _ => IbcTimeout::with_timestamp(env.block.time.plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP)),
        },
        memo: Some(to_json_string(&PacketMetadata {
            forward: Some(ForwardMetadata {
                receiver: to.clone(),
                port: "transfer".to_string(),
                // hop chain to final receiver chain channel
                channel: pfm_config.hop_to_destination_chain_channel_id.to_string(),
            }),
        })?),
    }))
}
