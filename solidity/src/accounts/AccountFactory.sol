// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import "./JitAccount.sol";

/// Account Factory with historical block entropy validation
contract AccountFactory {
    address public implementation;
    uint256 public constant MAX_BLOCK_AGE = 200;

    mapping(bytes32 => address) public accounts;
    mapping(address => mapping(uint64 => bool)) public usedAccountRequestIds;

    event AccountCreated(
        address indexed account,
        address indexed controller,
        string programId,
        uint64 accountRequestId,
        uint256 historicalBlock
    );

    error HistoricalBlockTooOld(uint256 currentBlock, uint256 historicalBlock);
    error HistoricalBlockNotAvailable(uint256 blockNumber);
    error AccountRequestIdAlreadyUsed(address controller, uint64 accountRequestId);
    error AccountCreationFailed();

    constructor(address _implementation) {
        implementation = _implementation;
    }

    function createAccount(
        address controller,
        string memory programId,
        uint64 accountRequestId,
        uint8 accountType,
        uint256 historicalBlockNumber
    ) external returns (address) {
        // Validate historical block is recent enough
        if (block.number - historicalBlockNumber > MAX_BLOCK_AGE) {
            revert HistoricalBlockTooOld(block.number, historicalBlockNumber);
        }

        // Check account request ID hasn't been used
        if (usedAccountRequestIds[controller][accountRequestId]) {
            revert AccountRequestIdAlreadyUsed(controller, accountRequestId);
        }

        // Get historical entropy
        bytes32 historicalEntropy = _getHistoricalEntropy(historicalBlockNumber);

        // Generate salt with historical entropy
        bytes32 salt = keccak256(
            abi.encodePacked(
                controller, programId, accountRequestId, accountType, historicalEntropy, historicalBlockNumber
            )
        );

        // Deploy with CREATE2
        bytes memory bytecode = abi.encodePacked(type(JitAccount).creationCode, abi.encode(controller, accountType));

        address account;
        assembly {
            account := create2(0, add(bytecode, 0x20), mload(bytecode), salt)
        }

        if (account == address(0)) {
            revert AccountCreationFailed();
        }

        // Mark account request ID as used and store account
        usedAccountRequestIds[controller][accountRequestId] = true;
        accounts[salt] = account;

        emit AccountCreated(account, controller, programId, accountRequestId, historicalBlockNumber);
        return account;
    }

    function computeAccountAddress(
        address controller,
        string memory programId,
        uint64 accountRequestId,
        uint8 accountType,
        uint256 historicalBlockNumber
    ) external view returns (address) {
        // Get historical entropy (view function, will revert if not available)
        bytes32 historicalEntropy = _getHistoricalEntropy(historicalBlockNumber);

        bytes32 salt = keccak256(
            abi.encodePacked(
                controller, programId, accountRequestId, accountType, historicalEntropy, historicalBlockNumber
            )
        );

        bytes memory bytecode = abi.encodePacked(type(JitAccount).creationCode, abi.encode(controller, accountType));

        bytes32 hash = keccak256(abi.encodePacked(hex"ff", address(this), salt, keccak256(bytecode)));
        return address(uint160(uint256(hash)));
    }

    function _getHistoricalEntropy(uint256 blockNumber) internal view returns (bytes32) {
        bytes32 blockHash = blockhash(blockNumber);
        if (blockHash == 0) {
            revert HistoricalBlockNotAvailable(blockNumber);
        }

        return keccak256(abi.encode(blockHash, blockNumber));
    }

    function isAccountRequestIdUsed(address controller, uint64 accountRequestId) external view returns (bool) {
        return usedAccountRequestIds[controller][accountRequestId];
    }

    function getMaxBlockAge() external pure returns (uint256) {
        return MAX_BLOCK_AGE;
    }
}
