use alloy_sol_types::sol;

sol! {
    struct CCTPTransferConfig {
        uint256 amountToTransfer; // If we want to transfer all tokens, we can set this to 0.
        bytes32 mintRecipient;
        address inputAccount;
        uint32 destinationDomain;
        address cctpTokenMessenger;
        address transferToken;
    }

    function transfer() external view;
}
