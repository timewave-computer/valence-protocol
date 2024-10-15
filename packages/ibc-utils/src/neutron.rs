use cosmos_sdk_proto::cosmos::base::v1beta1::Coin;
use cosmos_sdk_proto::traits::MessageExt;
use cosmwasm_std::{Binary, CosmosMsg, DepsMut, Env, StdError, StdResult, Uint128};
use neutron_sdk::{
    bindings::{msg::IbcFee, query::NeutronQuery},
    query::min_ibc_fee::query_min_ibc_fee,
};

// Default timeout for IbcTransfer is 300 blocks
const DEFAULT_TIMEOUT_HEIGHT: u64 = 300;
// Default timeout for IbcTransfer is 600 seconds
const DEFAULT_TIMEOUT_TIMESTAMP: u64 = 600;
const NTRN_DENOM: &str = "untrn";

#[allow(clippy::too_many_arguments)]
pub fn ibc_send_message(
    deps: DepsMut<NeutronQuery>,
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
) -> StdResult<CosmosMsg> {
    // contract must pay for relaying of acknowledgements
    // See more info here: https://docs.neutron.org/neutron/feerefunder/overview
    let ibc_fee = min_ntrn_ibc_fee(
        query_min_ibc_fee(deps.as_ref())
            .map_err(|err| StdError::generic_err(err.to_string()))?
            .min_fee,
    );
    let total_fee = flatten_ntrn_ibc_fee(&ibc_fee);
    let amount_minus_fee = amount
        .checked_sub(total_fee.u128())
        .ok_or_else(|| StdError::generic_err("Amount too low to pay for IBC transfer fees."))?;
    let coin = Coin {
        denom: denom.clone(),
        amount: amount_minus_fee.to_string(),
    };

    let msg = neutron_sdk::proto_types::neutron::transfer::MsgTransfer {
        source_port: port.unwrap_or("transfer".to_string()),
        source_channel: channel.clone(),
        sender,
        receiver: to.clone(),
        token: Some(coin),
        timeout_height: Some(cosmos_sdk_proto::ibc::core::client::v1::Height {
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
        fee: Some(neutron_sdk::proto_types::neutron::feerefunder::Fee {
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
        }),
    };

    #[allow(deprecated)]
    Ok(CosmosMsg::Stargate {
        type_url: "/neutron.transfer.MsgTransfer".to_string(),
        value: Binary::from(msg.to_bytes().unwrap()),
    })
}

fn min_ntrn_ibc_fee(fee: IbcFee) -> IbcFee {
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

fn flatten_ntrn_ibc_fee(ibc_fee: &IbcFee) -> Uint128 {
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
