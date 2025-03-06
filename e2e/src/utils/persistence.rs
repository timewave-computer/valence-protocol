use std::{error::Error, str::FromStr};

use cosmos_grpc_client::{BroadcastMode, CoinType, GrpcClient, ProstMsgToAny, Wallet};
use cosmwasm_std_old::Decimal;
use localic_std::{errors::LocalError, transactions::ChainRequestBuilder};
use log::info;
use persistence_std::types::pstake::liquidstakeibc::v1beta1::{KvUpdate, MsgUpdateHostChain};
use serde_json::Value;
use tokio::runtime::Runtime;

use crate::utils::file::get_grpc_address_and_port_from_logs;

use super::{
    ADMIN_MNEMONIC, PERSISTENCE_CHAIN_ADMIN_ADDR, PERSISTENCE_CHAIN_DENOM, PERSISTENCE_CHAIN_ID,
    PERSISTENCE_CHAIN_PREFIX,
};

pub fn register_host_zone(
    rb: &ChainRequestBuilder,
    chain_id: &str,
    connection_id: &str,
    channel_id: &str,
    native_denom: &str,
    from_key: &str,
) -> Result<Value, LocalError> {
    // Check that it's not registered yet
    if is_host_zone_registered(rb, chain_id).is_some() {
        return Ok(Value::Null);
    }

    let cmd = format!(
        "tx liquidstakeibc register-host-chain {} {} transfer 0 0.05 0 0.005 {} 1 4 2 --from={} --gas auto --gas-adjustment 1.3 --output=json",
        connection_id,
        channel_id,
        native_denom,
        from_key,
    );
    rb.tx(&cmd, true)
}

pub fn activate_host_zone(target_chain_id: &str) -> Result<(), Box<dyn Error>> {
    // Because of RPC on local-ic escaping the " character and Persistence being strict in wanting exactly the precise JSON payload, I'm using gRPC instead
    // Open and parse the logs file
    let (target_grpc_address, target_port) =
        get_grpc_address_and_port_from_logs(PERSISTENCE_CHAIN_ID)?;

    // Send the activation via gRPC
    let rt = Runtime::new()?;
    rt.block_on(send_grpc_activation(
        target_chain_id,
        &format!("{}:{}", target_grpc_address, target_port),
    ))?;
    info!("Host zone activated successfully");

    Ok(())
}

async fn send_grpc_activation(
    target_chain_id: &str,
    target_grpc_address: &str,
) -> Result<(), Box<dyn Error>> {
    let grpc_client = GrpcClient::new(target_grpc_address).await?;

    let mut wallet = Wallet::from_seed_phrase(
        grpc_client,
        ADMIN_MNEMONIC,
        PERSISTENCE_CHAIN_PREFIX,
        CoinType::Cosmos,
        0,
        Decimal::from_str("0.0025").unwrap(),
        Decimal::from_str("1.5").unwrap(),
        PERSISTENCE_CHAIN_DENOM,
    )
    .await?;

    let update = KvUpdate {
        key: "active".to_string(),
        value: "true".to_string(),
    };

    let update_host_chain_msg = MsgUpdateHostChain {
        authority: PERSISTENCE_CHAIN_ADMIN_ADDR.to_string(),
        chain_id: target_chain_id.to_string(),
        updates: vec![update],
    }
    .build_any_with_type_url("/pstake.liquidstakeibc.v1beta1.MsgUpdateHostChain");

    wallet
        .broadcast_tx(vec![update_host_chain_msg], None, None, BroadcastMode::Sync)
        .await?;

    Ok(())
}

pub fn is_host_zone_registered(rb: &ChainRequestBuilder, target_chain_id: &str) -> Option<Value> {
    let query_cmd = "liquidstakeibc host-chains --output=json".to_string();
    let host_chains_response = rb.q(&query_cmd, false);

    if let Some(host_chains) = host_chains_response["host_chains"].as_array() {
        for chain in host_chains {
            if let Some(chain_id) = chain["chain_id"].as_str() {
                if chain_id == target_chain_id {
                    return Some(chain.clone());
                }
            }
        }
    }

    None
}
