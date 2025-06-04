// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {IERC20} from "forge-std/src/interfaces/IERC20.sol";

/**
 * @title IDynamicRatioOracle
 * @notice Interface for dynamic ratio oracle contracts
 */
interface IDynamicRatioOracle {
    /**
     * @notice Query the dynamic ratio for a given token and parameters
     * @param token The token address to get ratio for
     * @param params Encoded parameters for the oracle
     * @return ratio The dynamic ratio (scaled by 10^18)
     */
    function queryDynamicRatio(IERC20 token, bytes calldata params) external view returns (uint256 ratio);
}
