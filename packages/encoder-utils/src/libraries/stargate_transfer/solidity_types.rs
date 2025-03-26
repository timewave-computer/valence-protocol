use alloy_sol_types::sol;

sol! {
    struct StargateConfig {
        bytes32 recipient;
        address inputAccount;
        uint32 destinationDomain;
        address stargateAddress;
        address transferToken;
        uint256 amount;
        uint256 minAmountToReceive;
        address refundAddress;
        bytes extraOptions;
        bytes composeMsg;
        bytes oftCmd;
    }

    function transfer() external view;
}
