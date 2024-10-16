use std::{error::Error, fs::File, io::Read, str::FromStr};

use cosmos_grpc_client::{BroadcastMode, CoinType, GrpcClient, ProstMsgToAny, Wallet};
use cosmwasm_std_old::Decimal;
use localic_std::{errors::LocalError, transactions::ChainRequestBuilder};
use persistence_std::types::pstake::liquidstakeibc::v1beta1::{KvUpdate, MsgUpdateHostChain};
use serde_json::Value;
use tokio::runtime::Runtime;

use super::{LOGS_FILE_PATH, PERSISTENCE_CHAIN_ADMIN_ADDR, PERSISTENCE_CHAIN_ID};

pub fn register_host_zone(
    rb: &ChainRequestBuilder,
    chain_id: &str,
    connection_id: &str,
    channel_id: &str,
    native_denom: &str,
    from_key: &str,
) -> Result<Value, LocalError> {
    // Check that it's not registered yet
    if query_host_zone(rb, chain_id) {
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
    // Because of RPC escaping the " character and Persistence being strict in wanting exactly the precise JSON payload, I can't do this via RPC so I'm using GRPC instead
    // Parse into a JSON the Logs file
    let mut file = File::open(LOGS_FILE_PATH)?;

    // Read the file contents into a string
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    // Parse the string into a JSON value
    let json: Value = serde_json::from_str(&contents)?;

    // Get the GRPC address
    let chains = json["chains"]
        .as_array()
        .ok_or("'chains' field not found or not an array")?;

    let mut target_grpc_address = "";
    for chain in chains {
        if let Some(chain_id) = chain["chain_id"].as_str() {
            if chain_id == PERSISTENCE_CHAIN_ID {
                if let Some(grpc_address) = chain["grpc_address"].as_str() {
                    target_grpc_address = grpc_address;
                } else {
                    return Err("gRPC address not found for the specified chain".into());
                }
            }
        }
    }

    // Send it via GRPC
    let rt = Runtime::new()?;
    rt.block_on(async {
        let mut wallet = Wallet::from_seed_phrase(
            GrpcClient::new(format!("http://{}", target_grpc_address)).await.unwrap(),
            "decorate bright ozone fork gallery riot bus exhaust worth way bone indoor calm squirrel merry zero scheme cotton until shop any excess stage laundry",
            "persistence",
            CoinType::Cosmos,
            0,
            Decimal::from_str("0.0025").unwrap(),
            Decimal::from_str("1.5").unwrap(),
            "uxrpt",
        ).await.unwrap();

        let update_host_chain_msg = MsgUpdateHostChain { 
            authority: PERSISTENCE_CHAIN_ADMIN_ADDR.to_string(), 
            chain_id: target_chain_id.to_string(), 
            updates: vec![KvUpdate { 
                key: "active".to_string(), 
                value: "true".to_string()}
                ] 
            }.build_any_with_type_url("/pstake.liquidstakeibc.v1beta1.MsgUpdateHostChain");

        wallet.broadcast_tx(vec![update_host_chain_msg], None, None, BroadcastMode::Sync).await.unwrap();
    });

    Ok(())
}

pub fn query_host_zone(rb: &ChainRequestBuilder, target_chain_id: &str) -> bool {
    let query_cmd = format!("liquidstakeibc host-chains --output=json");
    let host_chains_response = rb.q(&query_cmd, false);

    if let Some(host_chains) = host_chains_response["host_chains"].as_array() {
        for chain in host_chains {
            if let Some(chain_id) = chain["chain_id"].as_str() {
                if chain_id == target_chain_id {
                    return true;
                }
            }
        }
    }

    false
}
