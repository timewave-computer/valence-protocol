// SPDX-License-Identifier: Apache-2.0
// Adapted from: https://github.com/pancakeswap/pancake-v3-contracts
pragma solidity ^0.8.28;

interface IMasterChefV3 {
    struct CollectParams {
        uint256 tokenId;
        address recipient;
        uint128 amount0Max;
        uint128 amount1Max;
    }

    /// @notice Collects up to a maximum amount of fees owed to a specific position to the recipient, then refund.
    /// @param params CollectParams.
    /// @param to Refund recipent.
    /// @return amount0 The amount of fees collected in token0
    /// @return amount1 The amount of fees collected in token1
    function collectTo(CollectParams memory params, address to) external returns (uint256 amount0, uint256 amount1);

    /// @notice harvest cake from pool.
    /// @param _tokenId Token Id of NFT.
    /// @param _to Address to.
    /// @return reward Cake reward.
    function harvest(uint256 _tokenId, address _to) external returns (uint256 reward);

    /// @notice Withdraw LP tokens from pool.
    /// @param _tokenId Token Id of NFT to deposit.
    /// @param _to Address to which NFT token to withdraw.
    /// @return reward Cake reward.
    function withdraw(uint256 _tokenId, address _to) external returns (uint256 reward);
}
