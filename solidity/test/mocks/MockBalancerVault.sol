// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {IAsset, IBalancerVault} from "../../src/libraries/interfaces/balancerV2/IBalancerVault.sol";

/**
 * @title MockBalancerVault
 * @dev Mock implementation of Balancer V2 Vault for testing purposes.
 */
contract MockBalancerVault {
    // Mock implementation of swap function
    function swap(IBalancerVault.SingleSwap memory, IBalancerVault.FundManagement memory, uint256, uint256)
        external
        pure
        returns (uint256)
    {
        // Empty implementation, just return a fixed amount
        return 1 ether;
    }

    // Mock implementation of batchSwap function
    function batchSwap(
        IBalancerVault.SwapKind,
        IBalancerVault.BatchSwapStep[] memory,
        IAsset[] memory assets,
        IBalancerVault.FundManagement memory,
        int256[] memory,
        uint256
    ) external pure returns (int256[] memory) {
        // Empty implementation, just return a fixed array
        int256[] memory assetDeltas = new int256[](assets.length);
        return assetDeltas;
    }
}
