use alloy_sol_types::sol;

pub mod forwarder;

// All libraries will have these functions
sol! {
    function updateProcessor(address _processor) external;
    function updateConfig(bytes memory _config) public;
    function renounceOwnership() external;
    function transferOwnership(address newOwner) external;
}
