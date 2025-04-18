use alloy_sol_types::sol;

sol! {
    struct UnionTransferConfig {
        uint8 protocolVersion;
        uint8 transferTokenDecimals;
        uint32 channelId;
        uint64 timeout;
        address inputAccount;
        address zkGM;
        uint256 amount;
        uint256 quoteTokenAmount;
        uint256 transferTokenUnwrappingPath;
        bytes recipient;
        bytes transferToken;
        bytes quoteToken;
        string transferTokenName;
        string transferTokenSymbol;
    }

    function transfer(uint256 _quoteAmount) external;
}
