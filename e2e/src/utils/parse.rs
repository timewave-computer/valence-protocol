use std::{error::Error, fs::File, io::Read};

use serde_json::Value;

use super::LOGS_FILE_PATH;

/// Helper to extract a given field from a given chain entry in the the local-ic logs file.
/// available options:
/// - rpc_address
/// - rest_address
/// - grpc_address
/// - p2p_address
pub fn get_chain_field_from_local_ic_log(
    target_chain_id: &str,
    target_field: &str,
) -> Result<String, Box<dyn Error>> {
    // Open the logs file
    let mut file = File::open(LOGS_FILE_PATH)?;

    // Read the file contents into a string
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    // Parse the string into a JSON value
    let json: Value = serde_json::from_str(&contents)?;

    let chains = json["chains"]
        .as_array()
        .ok_or("'chains' field not found or not an array")?;
    for chain in chains {
        if let Some(chain_id) = chain["chain_id"].as_str() {
            if chain_id == target_chain_id {
                if let Some(field) = chain[target_field].as_str() {
                    return Ok(field.to_string());
                } else {
                    return Err(
                        format!("{} not found for the specified chain", target_field).into(),
                    );
                }
            }
        }
    }

    Err(format!("Chain with ID '{}' not found in logs file", target_chain_id).into())
}

pub fn get_grpc_address_and_port_from_url(
    grpc_address: &str,
) -> Result<(String, String), Box<dyn Error>> {
    // Split the input on ':'
    let parts: Vec<&str> = grpc_address.split(':').collect();

    // Ensure we have exactly two parts: address and port
    if parts.len() != 2 {
        return Err("Invalid grpc_address format".into());
    }

    // Prepend "http://" to the address part
    let http_address = format!("http://{}", parts[0]);

    Ok((http_address, parts[1].to_string()))
}
