use cosmos_sdk_proto::{cosmos::base::v1beta1::Coin as ProtoCoin, traits::MessageExt};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Coin, CosmosMsg, QuerierWrapper, QueryRequest, StdResult, Uint64};
use neutron_sdk::{
    bindings::{query::NeutronQuery, types::ProtobufAny},
    proto_types::neutron::{
        feerefunder::Fee,
        interchaintxs::v1::{MsgRegisterInterchainAccount, MsgSubmitTx},
    },
};
use prost_types::Any;

#[cw_serde]
pub struct OpenAckVersion {
    pub version: String,
    pub controller_connection_id: String,
    pub host_connection_id: String,
    pub address: String,
    pub encoding: String,
    pub tx_type: String,
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
) -> StdResult<Vec<Coin>> {
    let query_msg = get_ictxs_module_params_query_msg();
    let response: QueryParamsResponse = querier.query(&query_msg)?;
    Ok(response.params.register_fee)
}

#[cw_serde]
pub struct Params {
    pub msg_submit_tx_max_messages: Uint64,
    pub register_fee: Vec<Coin>,
}

#[cw_serde]
pub struct QueryParamsResponse {
    pub params: Params,
}

pub fn register_ica_msg(
    sender: String,
    connection_id: String,
    interchain_account_id: String,
    ica_registration_fee: &Coin,
) -> CosmosMsg {
    // Transform the coins to the ProtoCoin type
    let register_fee = vec![ProtoCoin {
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

pub fn submit_tx(
    sender: String,
    connection_id: String,
    interchain_account_id: String,
    msgs: Vec<ProtobufAny>,
    memo: String,
    timeout: u64,
    fee: Fee,
) -> CosmosMsg {
    // Transform the messages into what MsgSubmitTx expects
    let any_msgs: Vec<Any> = msgs
        .into_iter()
        .map(|msg| Any {
            type_url: msg.type_url,
            value: msg.value.to_vec(),
        })
        .collect();

    let msg_submit_tx = MsgSubmitTx {
        from_address: sender.to_string(),
        interchain_account_id,
        connection_id,
        msgs: any_msgs,
        memo,
        timeout,
        fee: Some(fee),
    };

    #[allow(deprecated)]
    CosmosMsg::Stargate {
        type_url: "/neutron.interchaintxs.v1.MsgSubmitTx".to_string(),
        value: Binary::from(msg_submit_tx.to_bytes().unwrap()),
    }
}
