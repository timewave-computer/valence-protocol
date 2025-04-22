use std::collections::BTreeMap;

use cosmwasm_std::{coin, to_json_string, CosmosMsg, Env, IbcDstCallback, IbcTimeout, StdResult};

use crate::types::{
    ActionData, ActionWrapper, EurekaConfig, EurekaFee, EurekaMemo, ForwardMetadata, IbcInfo,
    IbcTransfer, PacketForwardMiddlewareConfig, PacketMetadata, WasmData, WasmMessage,
};

// Default timeout for IbcTransfer is 600 seconds
const DEFAULT_TIMEOUT_SECONDS: u64 = 600;

// Default timeout for EurekaTransfers is 12 hours
const DEFAULT_EUREKA_TIMEOUT_SECONDS: u64 = 43200;

#[allow(clippy::too_many_arguments)]
pub fn ibc_send_message(
    env: Env,
    channel: String,
    to: String,
    denom: String,
    amount: u128,
    memo: String,
    timeout_seconds: Option<u64>,
    denom_to_pfm_map: BTreeMap<String, PacketForwardMiddlewareConfig>,
) -> StdResult<CosmosMsg> {
    let msg = match denom_to_pfm_map.get(&denom) {
        None => CosmosMsg::Ibc(cosmwasm_std::IbcMsg::Transfer {
            channel_id: channel,
            to_address: to,
            amount: coin(amount, denom),
            timeout: IbcTimeout::with_timestamp(
                env.block
                    .time
                    .plus_seconds(timeout_seconds.unwrap_or(DEFAULT_TIMEOUT_SECONDS)),
            ),
            memo: Some(memo),
        }),
        Some(pfm_config) => CosmosMsg::Ibc(cosmwasm_std::IbcMsg::Transfer {
            channel_id: pfm_config.local_to_hop_chain_channel_id.to_string(),
            to_address: pfm_config
                .hop_chain_receiver_address
                .clone()
                .unwrap_or("pfm".to_string()),
            amount: coin(amount, denom),
            timeout: IbcTimeout::with_timestamp(
                env.block
                    .time
                    .plus_seconds(timeout_seconds.unwrap_or(DEFAULT_TIMEOUT_SECONDS)),
            ),
            memo: Some(to_json_string(&PacketMetadata {
                forward: Some(ForwardMetadata {
                    receiver: to.clone(),
                    port: "transfer".to_string(),
                    // hop chain to final receiver chain channel
                    channel: pfm_config.hop_to_destination_chain_channel_id.to_string(),
                }),
            })?),
        }),
    };

    Ok(msg)
}

/// Build a memo for the Eureka transfer
pub fn build_eureka_memo(
    env: &Env,
    receiver: String,
    eureka_fee: EurekaFee,
    eureka_config: EurekaConfig,
) -> StdResult<String> {
    let eureka_memo = EurekaMemo {
        dest_callback: IbcDstCallback {
            address: eureka_config.callback_contract,
            gas_limit: None,
        },
        wasm: WasmData {
            contract: eureka_config.action_contract,
            msg: WasmMessage {
                action: ActionWrapper {
                    action: ActionData {
                        ibc_transfer: IbcTransfer {
                            ibc_info: IbcInfo {
                                encoding: "application/x-solidity-abi".to_string(),
                                eureka_fee,
                                memo: eureka_config.memo.unwrap_or_default(),
                                receiver,
                                recover_address: eureka_config.recover_address,
                                source_channel: eureka_config.source_channel,
                            },
                        },
                    },
                    exact_out: false,
                    timeout_timestamp: env
                        .block
                        .time
                        .plus_seconds(
                            eureka_config
                                .timeout
                                .unwrap_or(DEFAULT_EUREKA_TIMEOUT_SECONDS),
                        )
                        .seconds(),
                },
            },
        },
    };
    to_json_string(&eureka_memo)
}
