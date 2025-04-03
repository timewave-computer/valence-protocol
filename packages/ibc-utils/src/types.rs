use cosmwasm_schema::cw_serde;

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
