// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

interface IValenceVaultStrategist {
    /// @dev Emitted on successful share price update by the strategist.
    /// @param sharePrice newly posted share price
    /// @param updateTimestamp block.time of the update
    event SharePriceUpdated(
        uint256 indexed sharePrice,
        uint256 indexed updateTimestamp
    );

    /// Sets the share price.
    /// @param newSharePrice The new share price.
    function setSharePrice(uint256 newSharePrice) external;
}
