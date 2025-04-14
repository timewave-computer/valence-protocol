use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Map of bridges available between 2 chains
/// `HashMap<BRIDGE_NAME, BridgeInfo>`
pub type BridgesMap = HashMap<String, BridgeInfo>;
/// Map for list of bridges possible between 2 chains
/// `HashMap<SOURCE_CHAIN, HashMap<DESTINATION_CHAIN, BridgesMap>>`
pub type BridgesConfig = HashMap<String, HashMap<String, BridgesMap>>;
trait IsBridgeInfo {}

pub enum Bridgetype {
    Polytone,
    Hyperlane,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", untagged)]
pub enum BridgeInfo {
    /// This is the details we need for the bridge to predict the address of the proxy
    /// https://github.com/DA0-DA0/polytone/blob/main/contracts/main/voice/src/contract.rs#L186
    ///
    /// Polytone is 1-to-1 bridge (1 note <> 1 voice), so we need this
    /// information for every chain that we are connected to.
    Polytone(PolytoneBridgeInfo),
    Hyperlane(HyperlaneBridgeInfo),
}

// impl<'de, A: BridgeInfo> Deserialize<'de> for Bridge<A> {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         // First deserialize into an intermediate structure
//         #[derive(Deserialize)]
//         struct BridgeIntermediate {
//             polytone: Option<PolytoneBridgeInfo>,
//         }

//         // Parse into the intermediate structure first
//         let intermediate = BridgeIntermediate::deserialize(deserializer)?;

//         if let Some(bridge_info) = intermediate.polytone {
//             // Check if the bridge info is empty
//             Ok(Bridge::Polytone(bridge_info))
//         } else {
//             panic!("Bridge info is missing");
//         };

//         // Then return the Bridge enum
//         Ok(Bridge::Polytone(intermediate.polytone))
//     }
// }

/// This struct represent the data that we need for polytone in a single chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolytoneBridgeInfo {
    /// The address of the voice on the chain
    pub voice_addr: String,
    /// The note address
    pub note_addr: String,
    /// The note port on the other chain
    pub other_note_port: String,
    /// The connection id to the other chain
    pub connection_id: String,
    /// The channel id to the other chain
    pub channel_id: String,
}

/// This struct represent the data that we need for polytone in a single chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperlaneBridgeInfo {
    /// The address of the mailbox contract
    pub mailbox: String,
    /// The id of the chain to communicate with
    pub chain_id: u64,
}

impl IsBridgeInfo for PolytoneBridgeInfo {}
impl IsBridgeInfo for HyperlaneBridgeInfo {}

// impl Bridge {
//     pub fn get_polytone_info(&self) -> A {
//         match self {
//             Bridge::Polytone(polytone_bridge) => polytone_bridge.clone(),
//             Bridge::Hyperlane(hyperlane_bridge) => hyperlane_bridge.clone(),
//             // _ => unimplemented!("Bridge is not Polytone"),
//         }
//     }
// }

pub fn get_bridge_info<A: IsBridgeInfo>(bridge_info: &BridgeInfo) -> Box<dyn IsBridgeInfo> {
    match bridge_info {
        BridgeInfo::Polytone(polytone_bridge_info) => Box::new(polytone_bridge_info.clone()),
        BridgeInfo::Hyperlane(hyperlane_bridge_info) => Box::new(hyperlane_bridge_info.clone()),
    }
}
