use alloy_sol_types::sol;

sol! {
    struct BalancerV2SwapConfig {
        address inputAccount;
        address outputAccount;
        address vaultAddress;
    }

    function swap(
        bytes32 poolId,
        address tokenIn,
        address tokenOut,
        bytes memory userData,
        uint256 amount,
        uint256 minAmountOut,
        uint256 timeout
    ) external;

    function multiSwap(
        bytes32[] calldata poolIds,
        address[] calldata tokens,
        bytes[] calldata userDataArray,
        uint256 amount,
        uint256 minAmountOut,
        uint256 timeout
    ) external;
}
