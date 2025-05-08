// SPDX-License-Identifier: Apache-2.0
// Adapted from: https://github.com/skip-mev/skip-go-evm-contracts
pragma solidity ^0.8.28;

interface IEurekaHandler {
    struct TransferParams {
        address token;
        string recipient;
        string sourceClient;
        string destPort;
        uint64 timeoutTimestamp;
        string memo;
    }

    struct Fees {
        uint256 relayFee;
        address relayFeeRecipient;
        uint64 quoteExpiry;
    }

    function transfer(
        uint256 amount,
        TransferParams memory transferParams,
        Fees memory fees
    ) external;
}
