use alloy_sol_types::sol;

sol! {
    struct PancakeSwapV3PositionManagerConfig {
        address inputAccount;
        address outputAccount;
        address positionManager;
        address masterChef;
        address token0;
        address token1;
        uint24 poolFeeBps;
        uint16 slippageBps;
        uint256 timeout;
    }

    function createPosition(int24 tickLower, int24 tickUpper, uint256 amount0, uint256 amount1) external;

    function withdrawPosition(uint256 tokenId) external;
}
