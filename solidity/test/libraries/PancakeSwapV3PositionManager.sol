// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {PancakeSwapV3PositionManager} from "../../src/libraries/PancakeSwapV3PositionManager.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {MockERC20} from "../mocks/MockERC20.sol";

/**
 * @title PancakeSwapV3PositionManagerTest
 * @dev Test suite for PancakeSwapV3PositionManager contract validation logic
 */
contract PancakeSwapV3PositionManagerTest is Test {
    PancakeSwapV3PositionManager public positionManager;
    BaseAccount inputAccount;
    BaseAccount outputAccount;
    MockERC20 token0;
    MockERC20 token1;

    address owner = address(1);
    address processor = address(2);
    address mockPositionManager = address(3);
    address mockMasterChef = address(4);
    uint24 poolFee = 500; // 0.05%
    uint16 slippageBps = 100; // 1%
    uint256 timeout = 1800; // 30 minutes

    /**
     * @dev Setup test environment
     * Deploys accounts, tokens, and the PancakeSwapV3PositionManager with initial config
     */
    function setUp() public {
        vm.startPrank(owner);

        // Create accounts and tokens
        inputAccount = new BaseAccount(owner, new address[](0));
        outputAccount = new BaseAccount(owner, new address[](0));
        token0 = new MockERC20("TOKEN0", "TKN0", 18);
        token1 = new MockERC20("TOKEN1", "TKN1", 18);

        // Create a valid configuration
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig memory validConfig =
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig({
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            positionManager: mockPositionManager,
            masterChef: mockMasterChef,
            token0: address(token0),
            token1: address(token1),
            poolFee: poolFee,
            slippageBps: slippageBps,
            timeout: timeout
        });

        bytes memory configBytes = abi.encode(validConfig);
        positionManager = new PancakeSwapV3PositionManager(owner, processor, configBytes);

        // Approve the position manager to use the accounts
        inputAccount.approveLibrary(address(positionManager));
        outputAccount.approveLibrary(address(positionManager));

        vm.stopPrank();
    }

    function testUpdateConfigFailsZeroInputAccount() public {
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig memory invalidConfig =
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig({
            inputAccount: BaseAccount(payable(address(0))), // Zero address (invalid)
            outputAccount: outputAccount,
            positionManager: mockPositionManager,
            masterChef: mockMasterChef,
            token0: address(token0),
            token1: address(token1),
            poolFee: poolFee,
            slippageBps: slippageBps,
            timeout: timeout
        });

        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Input account can't be zero address");
        positionManager.updateConfig(configBytes);
    }

    function testUpdateConfigFailsZeroOutputAccount() public {
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig memory invalidConfig =
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig({
            inputAccount: inputAccount,
            outputAccount: BaseAccount(payable(address(0))), // Zero address (invalid)
            positionManager: mockPositionManager,
            masterChef: mockMasterChef,
            token0: address(token0),
            token1: address(token1),
            poolFee: poolFee,
            slippageBps: slippageBps,
            timeout: timeout
        });

        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Output account can't be zero address");
        positionManager.updateConfig(configBytes);
    }

    function testUpdateConfigFailsZeroPositionManager() public {
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig memory invalidConfig =
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig({
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            positionManager: address(0), // Zero address (invalid)
            masterChef: mockMasterChef,
            token0: address(token0),
            token1: address(token1),
            poolFee: poolFee,
            slippageBps: slippageBps,
            timeout: timeout
        });

        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Position manager address can't be zero address");
        positionManager.updateConfig(configBytes);
    }

    function testUpdateConfigFailsZeroMasterChef() public {
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig memory invalidConfig =
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig({
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            positionManager: mockPositionManager,
            masterChef: address(0), // Zero address (invalid)
            token0: address(token0),
            token1: address(token1),
            poolFee: poolFee,
            slippageBps: slippageBps,
            timeout: timeout
        });

        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Master chef address can't be zero address");
        positionManager.updateConfig(configBytes);
    }

    function testUpdateConfigFailsZeroToken0() public {
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig memory invalidConfig =
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig({
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            positionManager: mockPositionManager,
            masterChef: mockMasterChef,
            token0: address(0), // Zero address (invalid)
            token1: address(token1),
            poolFee: poolFee,
            slippageBps: slippageBps,
            timeout: timeout
        });

        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Token0 address can't be zero address");
        positionManager.updateConfig(configBytes);
    }

    function testUpdateConfigFailsZeroToken1() public {
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig memory invalidConfig =
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig({
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            positionManager: mockPositionManager,
            masterChef: mockMasterChef,
            token0: address(token0),
            token1: address(0), // Zero address (invalid)
            poolFee: poolFee,
            slippageBps: slippageBps,
            timeout: timeout
        });

        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Token1 address can't be zero address");
        positionManager.updateConfig(configBytes);
    }

    function testUpdateConfigFailsZeropoolFee() public {
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig memory invalidConfig =
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig({
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            positionManager: mockPositionManager,
            masterChef: mockMasterChef,
            token0: address(token0),
            token1: address(token1),
            poolFee: 0, // Zero value (invalid)
            slippageBps: slippageBps,
            timeout: timeout
        });

        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Pool fee can't be zero");
        positionManager.updateConfig(configBytes);
    }

    function testUpdateConfigFailsZeroTimeout() public {
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig memory invalidConfig =
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig({
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            positionManager: mockPositionManager,
            masterChef: mockMasterChef,
            token0: address(token0),
            token1: address(token1),
            poolFee: poolFee,
            slippageBps: slippageBps,
            timeout: 0 // Zero value (invalid)
        });

        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Timeout can't be zero");
        positionManager.updateConfig(configBytes);
    }

    function testUpdateConfigFailsTooHighSlippage() public {
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig memory invalidConfig =
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig({
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            positionManager: mockPositionManager,
            masterChef: mockMasterChef,
            token0: address(token0),
            token1: address(token1),
            poolFee: poolFee,
            slippageBps: 10001, // Over 100% (invalid)
            timeout: timeout
        });

        bytes memory configBytes = abi.encode(invalidConfig);
        vm.prank(owner);
        vm.expectRevert("Slippage can't be more than 100%");
        positionManager.updateConfig(configBytes);
    }

    function testUpdateConfigSucceedsWithValidParams() public {
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig memory validConfig =
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig({
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            positionManager: address(5), // Different address
            masterChef: address(6), // Different address
            token0: address(token1), // Swapped tokens
            token1: address(token0), // Swapped tokens
            poolFee: 3000, // Different fee
            slippageBps: 50, // Different slippage
            timeout: 900 // Different timeout
        });

        bytes memory configBytes = abi.encode(validConfig);
        vm.prank(owner);
        // This should succeed with all valid parameters
        positionManager.updateConfig(configBytes);

        // Verify config was updated correctly
        (
            BaseAccount newInputAccount,
            BaseAccount newOutputAccount,
            address newPositionManager,
            address newMasterChef,
            address newToken0,
            address newToken1,
            uint24 newpoolFee,
            uint16 newSlippageBps,
            uint256 newTimeout
        ) = positionManager.config();

        assertEq(address(newInputAccount), address(inputAccount), "Input account should match");
        assertEq(address(newOutputAccount), address(outputAccount), "Output account should match");
        assertEq(newPositionManager, address(5), "Position manager should be updated");
        assertEq(newMasterChef, address(6), "Master chef should be updated");
        assertEq(newToken0, address(token1), "Token0 should be updated");
        assertEq(newToken1, address(token0), "Token1 should be updated");
        assertEq(newpoolFee, 3000, "Pool fee should be updated");
        assertEq(newSlippageBps, 50, "Slippage should be updated");
        assertEq(newTimeout, 900, "Timeout should be updated");
    }

    function testUpdateConfigSucceedsWithMaxSlippage() public {
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig memory validConfig =
        PancakeSwapV3PositionManager.PancakeSwapV3PositionManagerConfig({
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            positionManager: mockPositionManager,
            masterChef: mockMasterChef,
            token0: address(token0),
            token1: address(token1),
            poolFee: poolFee,
            slippageBps: 10000, // 100% (valid, but max allowed)
            timeout: timeout
        });

        bytes memory configBytes = abi.encode(validConfig);
        vm.prank(owner);
        // This should succeed with maximum allowed slippage
        positionManager.updateConfig(configBytes);

        // Verify slippage was updated correctly
        (,,,,,,, uint16 newSlippageBps,) = positionManager.config();
        assertEq(newSlippageBps, 10000, "Slippage should be updated to maximum allowed value");
    }
}
