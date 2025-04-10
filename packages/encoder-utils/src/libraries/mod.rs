use alloy_sol_types::sol;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, StdError, StdResult};

pub mod aave_position_manager;
pub mod balancer_v2_swap;
pub mod cctp_transfer;
pub mod forwarder;
pub mod stargate_transfer;

#[cw_serde]
pub struct Bytes32Address(Binary);

pub trait ToFixedBytes<const N: usize> {
    /// Convert into a fixed-size byte array
    fn to_fixed_bytes(&self) -> Result<[u8; N], StdError>;
}

impl ToFixedBytes<32> for Bytes32Address {
    fn to_fixed_bytes(&self) -> Result<[u8; 32], StdError> {
        self.0.as_slice().try_into().map_err(|e| {
            StdError::generic_err(format!(
                "Error converting Bytes32Address to fixed size: {}",
                e
            ))
        })
    }
}

// You can also implement useful methods for Bytes32Address
impl Bytes32Address {
    pub fn new(binary: Binary) -> StdResult<Self> {
        // Validate the binary can be converted to [u8; 32]
        let _: [u8; 32] = binary.as_slice().try_into().map_err(|e| {
            StdError::generic_err(format!(
                "Error converting mint recipient to fixed size: {}",
                e
            ))
        })?;
        Ok(Bytes32Address(binary))
    }

    pub fn as_binary(&self) -> &Binary {
        &self.0
    }
}

// All libraries will have these functions
sol! {
    function updateProcessor(address _processor) external;
    function updateConfig(bytes memory _config) public;
    function renounceOwnership() external;
    function transferOwnership(address newOwner) external;
}
