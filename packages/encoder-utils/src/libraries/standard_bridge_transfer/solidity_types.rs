use alloy_sol_types::sol;

sol! {
    struct StandardBridgeTransferConfig {
        uint256 amount;
        address inputAccount;
        address recipient;
        address standardBridge;
        address token;
        address remoteToken;
        uint32 minGasLimit;
        bytes extraData;
    }

    function transfer() external view;
}
