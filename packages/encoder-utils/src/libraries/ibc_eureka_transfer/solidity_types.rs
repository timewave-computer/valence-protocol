use alloy_sol_types::sol;

sol! {
    struct IBCEurekaTransferConfig {
        uint256 amount;
        address transferToken;
        address inputAccount;
        string recipient;
        string sourceClient;
        uint64 timeout;
        address eurekaHandler;
    }

    struct Fees {
        uint256 relayFee;
        address relayFeeRecipient;
        uint64 quoteExpiry;
    }

    function transfer(Fees calldata fees, string calldata memo) external;
}
