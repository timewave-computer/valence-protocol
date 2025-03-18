use std::{error::Error, fs::File, io::Read};

use serde_json::Value;

use super::LOGS_FILE_PATH;

/// Helper to get the gRPC address of a chain from the local-ic logs file
pub fn get_grpc_address_from_logs(target_chain_id: &str) -> Result<String, Box<dyn Error>> {
    // Open the logs file
    let mut file = File::open(LOGS_FILE_PATH)?;

    // Read the file contents into a string
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    // Parse the string into a JSON value
    let json: Value = serde_json::from_str(&contents)?;

    // Get the gRPC address of Persistence chain
    let chains = json["chains"]
        .as_array()
        .ok_or("'chains' field not found or not an array")?;
    for chain in chains {
        if let Some(chain_id) = chain["chain_id"].as_str() {
            if chain_id == target_chain_id {
                if let Some(grpc_address) = chain["grpc_address"].as_str() {
                    return Ok(grpc_address.to_string());
                } else {
                    return Err("gRPC address not found for the specified chain".into());
                }
            }
        }
    }

    Err(format!("Chain with ID '{}' not found in logs file", target_chain_id).into())
}

pub fn get_grpc_address_and_port_from_logs(
    target_chain_id: &str,
) -> Result<(String, String), Box<dyn Error>> {
    // Get the gRPC address from the logs file
    let grpc_address = get_grpc_address_from_logs(target_chain_id)?;

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
