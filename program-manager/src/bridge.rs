use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Bridge {
    /// This is the details we need for the bridge to predict the address of the proxy
    /// https://github.com/DA0-DA0/polytone/blob/main/contracts/main/voice/src/contract.rs#L186
    ///
    /// Polytone is 1-to-1 bridge (1 note <> 1 voice), so we need this
    /// information for every chain that we are connected to.
    Polytone(PolytoneBridgeInfo),
}

/// This type represents the bridge info between 2 chains.
/// should always and only hold 2 elements, where the key is the
/// chain name and the value is the bridge info between those 2 chains.
pub type PolytoneBridgeInfo = HashMap<String, PolytoneSingleChainInfo>;

impl<'de> Deserialize<'de> for Bridge {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // First deserialize into an intermediate structure
        #[derive(Deserialize)]
        struct BridgeIntermediate {
            polytone: PolytoneBridgeInfo,
        }

        // Parse into the intermediate structure first
        let intermediate = BridgeIntermediate::deserialize(deserializer)?;

        // Then return the Bridge enum
        Ok(Bridge::Polytone(intermediate.polytone))
    }
}

/// This struct represent the data that we need for polytone in a single chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolytoneSingleChainInfo {
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

impl Bridge {
    pub fn get_polytone_info(&self) -> PolytoneBridgeInfo {
        match self {
            Bridge::Polytone(polytone_bridge) => polytone_bridge.clone(),
            // _ => unimplemented!("Bridge is not Polytone"),
        }
    }
}
