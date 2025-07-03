use std::collections::BTreeMap;

use cosmos_sdk_proto::cosmos::base::v1beta1::Coin;
use cosmos_sdk_proto::traits::MessageExt;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_string, Addr, Binary, CosmosMsg, DepsMut, Env, QuerierWrapper, QueryRequest, StdError,
    StdResult, Uint128, Uint64,
};
use cw_denom::CheckedDenom;
use neutron_sdk::bindings::msg::NeutronMsg;
use neutron_sdk::{
    bindings::{msg::IbcFee, query::NeutronQuery},
    proto_types::neutron::interchaintxs::v1::MsgRegisterInterchainAccount,
    query::min_ibc_fee::query_min_ibc_fee,
};

use crate::types::{ForwardMetadata, PacketForwardMiddlewareConfig, PacketMetadata};

// Default timeout for IbcTransfer is 600 seconds
const DEFAULT_TIMEOUT_SECONDS: u64 = 600;
const NTRN_DENOM: &str = "untrn";

#[allow(clippy::too_many_arguments)]
pub fn ibc_send_message(
    deps: DepsMut<NeutronQuery>,
    env: Env,
    channel: String,
    sender: &Addr,
    to: String,
    denom: &CheckedDenom,
    amount: u128,
    memo: String,
    timeout_seconds: Option<u64>,
    denom_to_pfm_map: BTreeMap<String, PacketForwardMiddlewareConfig>,
) -> StdResult<CosmosMsg> {
    // contract must pay for relaying of acknowledgements
    // See more info here: https://docs.neutron.org/neutron/feerefunder/overview
    let ibc_fee = min_ntrn_ibc_fee(
        query_min_ibc_fee(deps.as_ref())
            .map_err(|err| StdError::generic_err(err.to_string()))?
            .min_fee,
    );
    let total_fee = flatten_ntrn_ibc_fee(&ibc_fee);

    // Sender's account balance for fee denom (NTRN)
    let sender_ntrn_balance = deps.querier.query_balance(sender, NTRN_DENOM)?.amount;

    let transfer_amount = if denom.to_string() == NTRN_DENOM {
        if Uint128::from(amount) == sender_ntrn_balance {
            // The full balance is being transferred .. deduct the fee from the transfer amount
            Uint128::from(amount).checked_sub(total_fee)?
        } else {
            // Check that the balance is sufficient to cover the fees
            let amount_plus_fee = total_fee.checked_add(amount.into())?;
            if sender_ntrn_balance < amount_plus_fee {
                return Err(StdError::generic_err(format!(
                    "Insufficient balance to cover for IBC fees '{NTRN_DENOM}' in sender account (required: {amount_plus_fee}, available: {sender_ntrn_balance}).",
                )));
            }
            Uint128::from(amount)
        }
    } else {
        if sender_ntrn_balance < total_fee {
            return Err(StdError::generic_err(format!(
                "Insufficient balance to cover for IBC fees '{NTRN_DENOM}' in sender account (required: {total_fee}, available: {sender_ntrn_balance})."
            )));
        }
        amount.into()
    };

    let coin = Coin {
        denom: denom.to_string(),
        amount: transfer_amount.to_string(),
    };

    let msg = match denom_to_pfm_map.get(&denom.to_string()) {
        None => neutron_sdk::proto_types::neutron::transfer::MsgTransfer {
            source_port: "transfer".to_string(),
            source_channel: channel.clone(),
            sender: sender.to_string(),
            receiver: to.clone(),
            token: Some(coin),
            timeout_height: None,
            timeout_timestamp: env
                .block
                .time
                .plus_seconds(timeout_seconds.unwrap_or(DEFAULT_TIMEOUT_SECONDS))
                .nanos(),
            memo,
            fee: Some(get_transfer_fee(ibc_fee)),
        },
        Some(pfm_config) => {
            neutron_sdk::proto_types::neutron::transfer::MsgTransfer {
                source_port: "transfer".to_string(),
                source_channel: pfm_config.local_to_hop_chain_channel_id.to_string(),
                sender: sender.to_string(),
                receiver: pfm_config
                    .hop_chain_receiver_address
                    .clone()
                    .unwrap_or("pfm".to_string()),
                token: Some(coin),
                timeout_height: None,
                timeout_timestamp: env
                    .block
                    .time
                    .plus_seconds(timeout_seconds.unwrap_or(DEFAULT_TIMEOUT_SECONDS))
                    .nanos(),
                memo: to_json_string(&PacketMetadata {
                    forward: Some(ForwardMetadata {
                        receiver: to.clone(),
                        port: "transfer".to_string(),
                        // hop chain to final receiver chain channel
                        channel: pfm_config.hop_to_destination_chain_channel_id.to_string(),
                    }),
                })?,
                fee: Some(get_transfer_fee(ibc_fee)),
            }
        }
    };

    #[allow(deprecated)]
    Ok(CosmosMsg::Stargate {
        type_url: "/neutron.transfer.MsgTransfer".to_string(),
        value: Binary::from(msg.to_bytes().unwrap()),
    })
}

pub fn min_ntrn_ibc_fee(fee: IbcFee) -> IbcFee {
    IbcFee {
        recv_fee: fee.recv_fee,
        ack_fee: fee
            .ack_fee
            .into_iter()
            .filter(|a| a.denom == NTRN_DENOM)
            .collect(),
        timeout_fee: fee
            .timeout_fee
            .into_iter()
            .filter(|a| a.denom == NTRN_DENOM)
            .collect(),
    }
}

pub fn flatten_ntrn_ibc_fee(ibc_fee: &IbcFee) -> Uint128 {
    let mut total = Uint128::zero();

    for coin in &ibc_fee.recv_fee {
        total += coin.amount;
    }

    for coin in &ibc_fee.ack_fee {
        total += coin.amount;
    }

    for coin in &ibc_fee.timeout_fee {
        total += coin.amount;
    }

    total
}

pub fn get_transfer_fee(ibc_fee: IbcFee) -> neutron_sdk::proto_types::neutron::feerefunder::Fee {
    neutron_sdk::proto_types::neutron::feerefunder::Fee {
        recv_fee: ibc_fee
            .recv_fee
            .into_iter()
            .map(|c| Coin {
                denom: c.denom,
                amount: c.amount.to_string(),
            })
            .collect(),
        ack_fee: ibc_fee
            .ack_fee
            .into_iter()
            .map(|c| Coin {
                denom: c.denom,
                amount: c.amount.to_string(),
            })
            .collect(),
        timeout_fee: ibc_fee
            .timeout_fee
            .into_iter()
            .map(|c| Coin {
                denom: c.denom,
                amount: c.amount.to_string(),
            })
            .collect(),
    }
}

#[cw_serde]
pub struct Transfer {
    pub recipient: String,
    pub sender: String,
    pub denom: String,
    pub amount: String,
}

#[cw_serde]
pub struct OpenAckVersion {
    pub version: String,
    pub controller_connection_id: String,
    pub host_connection_id: String,
    pub address: String,
    pub encoding: String,
    pub tx_type: String,
}

#[cw_serde]
pub struct Params {
    pub msg_submit_tx_max_messages: Uint64,
    pub register_fee: Vec<cosmwasm_std::Coin>,
}

#[cw_serde]
pub struct QueryParamsResponse {
    pub params: Params,
}

pub fn get_ictxs_module_params_query_msg() -> QueryRequest<NeutronQuery> {
    #[allow(deprecated)]
    QueryRequest::Stargate {
        path: "/neutron.interchaintxs.v1.Query/Params".to_string(),
        data: Binary::new(vec![]),
    }
}

pub fn query_ica_registration_fee(
    querier: QuerierWrapper<'_, NeutronQuery>,
) -> StdResult<Vec<cosmwasm_std::Coin>> {
    let query_msg = get_ictxs_module_params_query_msg();
    let response: QueryParamsResponse = querier.query(&query_msg)?;
    Ok(response.params.register_fee)
}

pub fn register_ica_msg(
    sender: String,
    connection_id: String,
    interchain_account_id: String,
    ica_registration_fee: &cosmwasm_std::Coin,
) -> CosmosMsg<NeutronMsg> {
    // Transform the coins to the ProtoCoin type
    let register_fee = vec![Coin {
        denom: ica_registration_fee.denom.to_string(),
        amount: ica_registration_fee.amount.to_string(),
    }];

    let msg_register_interchain_account = MsgRegisterInterchainAccount {
        from_address: sender,
        connection_id,
        interchain_account_id,
        register_fee,
    };

    #[allow(deprecated)]
    CosmosMsg::Stargate {
        type_url: "/neutron.interchaintxs.v1.MsgRegisterInterchainAccount".to_string(),
        value: Binary::from(msg_register_interchain_account.to_bytes().unwrap()),
    }
}
