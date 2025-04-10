// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test, console} from "forge-std/src/Test.sol";
import {BalancerV2Swap} from "../../src/libraries/BalancerV2Swap.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {IAsset, IBalancerVault} from "../../src/libraries/interfaces/balancerV2/IBalancerVault.sol";
import {MockERC20} from "../mocks/MockERC20.sol";
import {MockBalancerVault} from "../mocks/MockBalancerVault.sol";

contract BalancerV2SwapTest is Test {
    // Contract under test
    BalancerV2Swap public balancerV2Swap;

    // Mock contracts
    MockBalancerVault public mockVault;
    BaseAccount public inputAccount;
    BaseAccount public outputAccount;
    MockERC20 public tokenA;
    MockERC20 public tokenB;
    MockERC20 public tokenC;

    // Test addresses
    address public owner;
    address public processor;

    // Test data
    bytes32 public poolId;
    bytes32 public poolId2;

    function setUp() public {
        // Setup test addresses
        owner = makeAddr("owner");
        processor = makeAddr("processor");

        // Deploy mock tokens
        tokenA = new MockERC20("Token A", "TA");
        tokenB = new MockERC20("Token B", "TB");
        tokenC = new MockERC20("Token C", "TC");

        // Create mock accounts
        vm.startPrank(owner);
        inputAccount = new BaseAccount(owner, new address[](0));
        outputAccount = new BaseAccount(owner, new address[](0));
        vm.stopPrank();

        // Deploy mock Balancer vault
        mockVault = new MockBalancerVault();

        // Create test pool IDs
        poolId = bytes32(uint256(1));
        poolId2 = bytes32(uint256(2));

        // Deploy BalancerV2Swap contract
        vm.startPrank(owner);

        // Create and encode config
        BalancerV2Swap.BalancerV2SwapConfig memory config = BalancerV2Swap.BalancerV2SwapConfig({
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            vaultAddress: address(mockVault)
        });

        balancerV2Swap = new BalancerV2Swap(owner, processor, abi.encode(config));
        inputAccount.approveLibrary(address(balancerV2Swap));
        vm.stopPrank();
    }

    // ============== Configuration Tests ==============

    function testConfigValidation() public {
        // Test invalid input account
        BalancerV2Swap.BalancerV2SwapConfig memory invalidConfig = BalancerV2Swap.BalancerV2SwapConfig({
            inputAccount: BaseAccount(payable(address(0))),
            outputAccount: outputAccount,
            vaultAddress: address(mockVault)
        });

        vm.prank(owner);
        vm.expectRevert("Input account can't be zero address");
        balancerV2Swap.updateConfig(abi.encode(invalidConfig));

        // Test invalid output account
        invalidConfig = BalancerV2Swap.BalancerV2SwapConfig({
            inputAccount: inputAccount,
            outputAccount: BaseAccount(payable(address(0))),
            vaultAddress: address(mockVault)
        });

        vm.prank(owner);
        vm.expectRevert("Output account can't be zero address");
        balancerV2Swap.updateConfig(abi.encode(invalidConfig));

        // Test invalid vault address
        invalidConfig = BalancerV2Swap.BalancerV2SwapConfig({
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            vaultAddress: address(0)
        });

        vm.prank(owner);
        vm.expectRevert("Vault address can't be zero address");
        balancerV2Swap.updateConfig(abi.encode(invalidConfig));
    }

    function testUpdateConfig() public {
        // Create a new configuration with different values
        address newVaultAddress = makeAddr("newVault");

        BalancerV2Swap.BalancerV2SwapConfig memory newConfig = BalancerV2Swap.BalancerV2SwapConfig({
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            vaultAddress: newVaultAddress
        });

        // Update config as owner
        vm.prank(owner);
        balancerV2Swap.updateConfig(abi.encode(newConfig));

        // Verify the configuration was updated
        (,, address vaultAddress) = balancerV2Swap.config();
        assertEq(vaultAddress, newVaultAddress);
    }

    function testUnauthorizedConfigUpdate() public {
        address unauthorized = makeAddr("unauthorized");

        BalancerV2Swap.BalancerV2SwapConfig memory config = BalancerV2Swap.BalancerV2SwapConfig({
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            vaultAddress: address(mockVault)
        });

        vm.prank(unauthorized);
        vm.expectRevert();
        balancerV2Swap.updateConfig(abi.encode(config));
    }

    // ============== Single Swap Validation Tests ==============

    function testSingleSwapValidations() public {
        // Test empty pool ID
        bytes32 emptyPoolId = bytes32(0);
        IAsset tokenInAsset = IAsset(address(tokenA));
        IAsset tokenOutAsset = IAsset(address(tokenB));
        bytes memory userData = "";
        uint256 amount = 100 * 10 ** 18;
        uint256 minAmountOut = 90 * 10 ** 18;
        uint256 timeout = 3600; // 1 hour

        vm.prank(processor);
        vm.expectRevert("Pool ID can't be empty for single swap");
        balancerV2Swap.swap(emptyPoolId, tokenInAsset, tokenOutAsset, userData, amount, minAmountOut, timeout);

        // Test zero address for token in
        vm.prank(processor);
        vm.expectRevert("Token in can't be zero address for single swap");
        balancerV2Swap.swap(poolId, IAsset(address(0)), tokenOutAsset, userData, amount, minAmountOut, timeout);

        // Test zero address for token out
        vm.prank(processor);
        vm.expectRevert("Token out can't be zero address for single swap");
        balancerV2Swap.swap(poolId, tokenInAsset, IAsset(address(0)), userData, amount, minAmountOut, timeout);

        // Test same token for in and out
        vm.prank(processor);
        vm.expectRevert("Token in and out can't be the same");
        balancerV2Swap.swap(poolId, tokenInAsset, tokenInAsset, userData, amount, minAmountOut, timeout);

        // Test zero timeout
        vm.prank(processor);
        vm.expectRevert("Timeout can't be zero");
        balancerV2Swap.swap(poolId, tokenInAsset, tokenOutAsset, userData, amount, minAmountOut, 0);

        // Test unauthorized caller
        address unauthorized = makeAddr("unauthorized");
        vm.prank(unauthorized);
        vm.expectRevert();
        balancerV2Swap.swap(poolId, tokenInAsset, tokenOutAsset, userData, amount, minAmountOut, timeout);
    }

    // ============== Multi Swap Validation Tests ==============

    function testMultiSwapValidations() public {
        // Setup test data for multi-hop swap
        bytes32[] memory poolIds = new bytes32[](2);
        poolIds[0] = poolId;
        poolIds[1] = poolId2;

        IAsset[] memory assets = new IAsset[](3);
        assets[0] = IAsset(address(tokenA));
        assets[1] = IAsset(address(tokenB));
        assets[2] = IAsset(address(tokenC));

        bytes[] memory userDataArray = new bytes[](2);
        userDataArray[0] = "";
        userDataArray[1] = "";

        uint256 amount = 100 * 10 ** 18;
        uint256 minAmountOut = 80 * 10 ** 18;
        uint256 timeout = 3600; // 1 hour

        // Test empty poolIds array
        bytes32[] memory emptyPoolIds = new bytes32[](0);

        vm.prank(processor);
        vm.expectRevert("Pool IDs array can't be empty for multi-hop swap");
        balancerV2Swap.multiSwap(emptyPoolIds, assets, userDataArray, amount, minAmountOut, timeout);

        // Test empty assets array
        IAsset[] memory emptyAssets = new IAsset[](0);

        vm.prank(processor);
        vm.expectRevert("Assets array can't be empty for multi-hop swap");
        balancerV2Swap.multiSwap(poolIds, emptyAssets, userDataArray, amount, minAmountOut, timeout);

        // Test assets array length mismatch
        IAsset[] memory invalidAssets = new IAsset[](2); // Should be 3 for 2 pool IDs
        invalidAssets[0] = IAsset(address(tokenA));
        invalidAssets[1] = IAsset(address(tokenB));

        vm.prank(processor);
        vm.expectRevert("Assets array must contain at least poolIds.length + 1 elements");
        balancerV2Swap.multiSwap(poolIds, invalidAssets, userDataArray, amount, minAmountOut, timeout);

        // Test userData array length mismatch
        bytes[] memory invalidUserData = new bytes[](1); // Should be 2 for 2 pool IDs
        invalidUserData[0] = "";

        vm.prank(processor);
        vm.expectRevert("userData array length must match poolIds length");
        balancerV2Swap.multiSwap(poolIds, assets, invalidUserData, amount, minAmountOut, timeout);

        // Test empty pool ID in array
        bytes32[] memory invalidPoolIds = new bytes32[](2);
        invalidPoolIds[0] = poolId;
        invalidPoolIds[1] = bytes32(0);

        vm.prank(processor);
        vm.expectRevert("Pool ID can't be empty in poolIds array");
        balancerV2Swap.multiSwap(invalidPoolIds, assets, userDataArray, amount, minAmountOut, timeout);

        // Test zero address in assets array
        IAsset[] memory invalidAssetsZero = new IAsset[](3);
        invalidAssetsZero[0] = IAsset(address(tokenA));
        invalidAssetsZero[1] = IAsset(address(0));
        invalidAssetsZero[2] = IAsset(address(tokenC));

        vm.prank(processor);
        vm.expectRevert("Asset can't be zero address in assets array");
        balancerV2Swap.multiSwap(poolIds, invalidAssetsZero, userDataArray, amount, minAmountOut, timeout);

        // Test zero timeout
        vm.prank(processor);
        vm.expectRevert("Timeout can't be zero");
        balancerV2Swap.multiSwap(poolIds, assets, userDataArray, amount, minAmountOut, 0);

        // Test unauthorized caller
        address unauthorized = makeAddr("unauthorized");
        vm.prank(unauthorized);
        vm.expectRevert();
        balancerV2Swap.multiSwap(poolIds, assets, userDataArray, amount, minAmountOut, timeout);
    }
}
