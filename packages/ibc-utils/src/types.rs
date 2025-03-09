use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;

#[cw_serde]
pub struct PacketForwardMiddlewareConfig {
    pub local_to_hop_chain_channel_id: String,
    pub hop_to_destination_chain_channel_id: String,
    pub hop_chain_receiver_address: String,
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

// We want a serializable version of Any using the Binary wrapper and not take it from neutron-sdk because it injects neutron feature into the contract
#[cw_serde]
pub struct ProtobufAny {
    pub type_url: String,
    pub value: Binary,
}
