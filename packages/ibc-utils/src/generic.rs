use std::collections::BTreeMap;

use cosmos_sdk_proto_0_24::{cosmos::base::v1beta1::Coin, traits::MessageExt};
use cosmwasm_std::{to_json_string, Binary, CosmosMsg, Env, StdResult};

use crate::types::{ForwardMetadata, PacketForwardMiddlewareConfig, PacketMetadata};

// Default timeout for IbcTransfer is 300 blocks
const DEFAULT_TIMEOUT_HEIGHT: u64 = 300;
// Default timeout for IbcTransfer is 600 seconds
const DEFAULT_TIMEOUT_TIMESTAMP: u64 = 600;

#[allow(clippy::too_many_arguments)]
pub fn ibc_send_message(
    env: Env,
    channel: String,
    port: Option<String>,
    sender: String,
    to: String,
    denom: String,
    amount: u128,
    memo: String,
    timeout_height: Option<u64>,
    timeout_timestamp: Option<u64>,
    denom_to_pfm_map: BTreeMap<String, PacketForwardMiddlewareConfig>,
) -> StdResult<CosmosMsg> {
    let coin = Coin {
        denom: denom.clone(),
        amount: amount.to_string(),
    };
    let msg = match denom_to_pfm_map.get(&denom) {
        None => cosmos_sdk_proto_0_24::ibc::applications::transfer::v1::MsgTransfer {
            source_port: port.unwrap_or("transfer".to_string()),
            source_channel: channel,
            sender,
            receiver: to,
            token: Some(coin),
            timeout_height: Some(cosmos_sdk_proto_0_24::ibc::core::client::v1::Height {
                revision_number: 2,
                revision_height: timeout_height.unwrap_or(DEFAULT_TIMEOUT_HEIGHT),
            }),
            timeout_timestamp: timeout_timestamp.unwrap_or(
                env.block
                    .time
                    .plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP)
                    .nanos(),
            ),
            memo,
        },
        Some(pfm_config) => cosmos_sdk_proto_0_24::ibc::applications::transfer::v1::MsgTransfer {
            source_port: port.unwrap_or("transfer".to_string()),
            source_channel: pfm_config.local_to_hop_chain_channel_id.to_string(),
            sender,
            receiver: pfm_config.hop_chain_receiver_address.to_string(),
            token: Some(coin),
            timeout_height: Some(cosmos_sdk_proto_0_24::ibc::core::client::v1::Height {
                revision_number: 2,
                revision_height: timeout_height.unwrap_or(DEFAULT_TIMEOUT_HEIGHT),
            }),
            timeout_timestamp: timeout_timestamp.unwrap_or(
                env.block
                    .time
                    .plus_seconds(DEFAULT_TIMEOUT_TIMESTAMP)
                    .nanos(),
            ),
            memo: to_json_string(&PacketMetadata {
                forward: Some(ForwardMetadata {
                    receiver: to.clone(),
                    port: "transfer".to_string(),
                    // hop chain to final receiver chain channel
                    channel: pfm_config.hop_to_destination_chain_channel_id.to_string(),
                }),
            })?,
        },
    };

    #[allow(deprecated)]
    Ok(CosmosMsg::Stargate {
        type_url: "/ibc.applications.transfer.v1.MsgTransfer".to_string(),
        value: Binary::from(msg.to_bytes().unwrap()),
    })
}
