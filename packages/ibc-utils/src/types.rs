use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, IbcDstCallback};

#[cw_serde]
pub struct PacketForwardMiddlewareConfig {
    pub local_to_hop_chain_channel_id: String,
    pub hop_to_destination_chain_channel_id: String,
    pub hop_chain_receiver_address: Option<String>,
}

// https://github.com/strangelove-ventures/packet-forward-middleware/blob/main/router/types/forward.go
#[cw_serde]
pub struct PacketMetadata {
    pub forward: Option<ForwardMetadata>,
}

#[cw_serde]
pub struct ForwardMetadata {
    pub receiver: String,
    pub port: String,
    pub channel: String,
}


#[cw_serde]
pub struct EurekaConfig {
    /// The address of the contract on intermediate chain that will be used to trigger the Eureka Transfer and send the fee
    pub action_contract: String,
    /// Recover address on intermediate chain in case the transfer fails
    pub recover_address: String,
    /// Source channel on the intermediate chain (e.g. "08-wasm-1369")
    pub source_channel: String,
    /// Optional memo for the Eureka transfer triggered by the contract. Not used right now but could eventually be used.
    pub memo: Option<String>,
    /// Timeout in seconds to be used for the Eureka transfer. For reference, Skip Go uses 12 hours (43200). If not passed we will use that default value
    pub timeout: Option<u64>,
}

// Used for IBC Eureka transfers
// Leverages https://github.com/cosmos/ibc-go/blob/16f51eb5635bc16c6361c44f2a963f4736d1cf8b/docs/docs/04-middleware/01-callbacks/05-end-users.md
#[cw_serde]
pub struct EurekaMemo {
    dest_callback: IbcDstCallback,
    wasm: WasmData,
}

#[cw_serde]
pub struct WasmData {
    contract: String,
    msg: WasmMessage,
}

#[cw_serde]
pub struct WasmMessage {
    action: ActionWrapper,
}

#[cw_serde]
pub struct ActionWrapper {
    action: ActionData,
    exact_out: bool,
    timeout_timestamp: u64,
}

#[cw_serde]
pub struct ActionData {
    ibc_transfer: IbcTransfer,
}

#[cw_serde]
pub struct IbcTransfer {
    ibc_info: IbcInfo,
}

#[cw_serde]
pub struct IbcInfo {
    encoding: String,
    eureka_fee: EurekaFee,
    memo: String,
    receiver: String,
    recover_address: String,
    source_channel: String,
}

#[cw_serde]
pub struct EurekaFee {
    coin: Coin,
    receiver: String,
    timeout_timestamp: u64,
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{from_json, to_json_string, Uint128};

    use super::*;

    #[test]
    fn test_eureka_memo_serialization() {
        // Create a sample struct
        let wrapper = EurekaMemo {
            dest_callback: IbcDstCallback {
                address: "cosmos198plfkpwzpxxrlpvprhfmdkcf3frpa7kvduq9cw8lh02mm327tgqhh3s55".to_string(),
                gas_limit: None,
            },
            wasm: WasmData {
                contract: "cosmos1zvesudsdfxusz06jztpph4d3h5x6veglqsspxns2v2jqml9nhywshhfp5j".to_string(),
                msg: WasmMessage {
                    action: ActionWrapper {
                        action: ActionData {
                            ibc_transfer: IbcTransfer {
                                ibc_info: IbcInfo {
                                    encoding: "application/x-solidity-abi".to_string(),
                                    eureka_fee: EurekaFee {
                                        coin: Coin {
                                            amount: Uint128::new(250000),
                                            denom: "uatom".to_string(),
                                        },
                                        receiver: "cosmos1066ea436np9m6gf4q95q0nte2ctq84wuzahttk".to_string(),
                                        timeout_timestamp: 1744811052000000000,
                                    },
                                    memo: "".to_string(),
                                    receiver: "0x0000000000000000000000000000000000000001".to_string(),
                                    recover_address: "cosmos1ep2umj6kn34g2ttjalsc5r9w8pt7sv4x9z0q26".to_string(),
                                    source_channel: "08-wasm-1369".to_string(),
                                },
                            },
                        },
                        exact_out: false,
                        timeout_timestamp: 1744852503,
                    },
                },
            },
        };
        
        // Serialize to JSON
        let memo = to_json_string(&wrapper).unwrap();

        println!("Serialized Memo: {}", memo);
        
        // Assert it contains expected values
        assert!(memo.contains("cosmos198plfkpwzpxxrlpvprhfmdkcf3frpa7kvduq9cw8lh02mm327tgqhh3s55"));
        assert!(memo.contains("250000"));
        assert!(memo.contains("uatom"));
        assert!(memo.contains("0x0000000000000000000000000000000000000001"));
        
        // Test deserialization
        let deserialized: EurekaMemo = from_json(&memo).unwrap();
        
        // Verify some values from the reconstructed object
        assert_eq!(deserialized.dest_callback.address, 
                  "cosmos198plfkpwzpxxrlpvprhfmdkcf3frpa7kvduq9cw8lh02mm327tgqhh3s55");
        assert_eq!(deserialized.wasm.msg.action.action.ibc_transfer.ibc_info.receiver, 
                  "0x0000000000000000000000000000000000000001");
    }
}
