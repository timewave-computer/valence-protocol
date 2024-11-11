use std::collections::BTreeMap;

use cosmwasm_std::{coin, CosmosMsg, Env, IbcTimeout, StdResult, Timestamp};

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
    timeout_timestamp: Option<u64>,
    _denom_to_pfm_map: BTreeMap<String, PacketForwardMiddlewareConfig>,
) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Ibc(cosmwasm_std::IbcMsg::Transfer {
        channel_id: channel,
        to_address: to,
        amount: coin(amount, denom),
        timeout: match timeout_timestamp {
            Some(timestamp) => IbcTimeout::with_timestamp(Timestamp::from_nanos(timestamp)),
            None => {
                IbcTimeout::with_timestamp(env.block.time.plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP))
            }
        },
        memo: Some(memo),
    }))
}
