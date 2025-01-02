use alloy_sol_types::sol;

sol! {
    struct ForwardingConfig {
        address tokenAddress;
        uint128 maxAmount;
    }

    enum IntervalType {
        TIME,
        BLOCKS
    }

    struct ForwarderConfig {
        // We can use address here directly because that's what Smart contracts are under the hood in Solidity
        address inputAccount;
        address outputAccount;
        ForwardingConfig[] forwardingConfigs;
        IntervalType intervalType;
        uint64 minInterval;
    }

    function forward() external view;
}
