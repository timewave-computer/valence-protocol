// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test, console} from "forge-std/src/Test.sol";
import {AavePositionManager} from "../../src/libraries/AavePositionManager.sol";
import {IPool} from "aave-v3-origin/interfaces/IPool.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {MockAavePool} from "../mocks/MockAavePool.sol";
import {MockERC20} from "../mocks/MockERC20.sol";

contract AavePositionManagerTest is Test {
    // Contract under test
    AavePositionManager public aavePositionManager;

    // Mock contracts
    MockAavePool public mockPool;
    BaseAccount public inputAccount;
    BaseAccount public outputAccount;
    MockERC20 public supplyToken;
    MockERC20 public borrowToken;

    // Test addresses
    address public owner;
    address public processor;
    uint16 public referralCode = 0;

    // Setup function to initialize test environment
    function setUp() public {
        // Setup test addresses
        owner = makeAddr("owner");
        processor = makeAddr("processor");

        // Deploy mock tokens
        supplyToken = new MockERC20("Supply Token", "ST");
        borrowToken = new MockERC20("Borrow Token", "BT");

        // Create mock accounts
        vm.startPrank(owner);
        inputAccount = new BaseAccount(owner, new address[](0));
        outputAccount = new BaseAccount(owner, new address[](0));
        vm.stopPrank();

        // Deploy mock Aave pool
        mockPool = new MockAavePool();

        // Deploy AavePositionManager contract
        vm.startPrank(owner);

        // Create and encode config directly
        AavePositionManager.AavePositionManagerConfig memory config = AavePositionManager.AavePositionManagerConfig({
            poolAddress: IPool(address(mockPool)),
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            supplyAsset: address(supplyToken),
            borrowAsset: address(borrowToken),
            referralCode: referralCode
        });

        aavePositionManager = new AavePositionManager(owner, processor, abi.encode(config));
        inputAccount.approveLibrary(address(aavePositionManager));
        vm.stopPrank();
    }

    // ============== Configuration Tests ==============

    // Test configuration validation
    function testConfigValidation() public {
        // Test invalid input account
        AavePositionManager.AavePositionManagerConfig memory invalidConfig = AavePositionManager
            .AavePositionManagerConfig({
            poolAddress: IPool(address(mockPool)),
            inputAccount: BaseAccount(payable(address(0))),
            outputAccount: outputAccount,
            supplyAsset: address(supplyToken),
            borrowAsset: address(borrowToken),
            referralCode: referralCode
        });

        vm.prank(owner);
        vm.expectRevert("Input account can't be zero address");
        aavePositionManager.updateConfig(abi.encode(invalidConfig));

        // Test invalid Aave pool address
        invalidConfig = AavePositionManager.AavePositionManagerConfig({
            poolAddress: IPool(address(0)),
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            supplyAsset: address(supplyToken),
            borrowAsset: address(borrowToken),
            referralCode: referralCode
        });

        vm.prank(owner);
        vm.expectRevert("Aave pool address can't be zero address");
        aavePositionManager.updateConfig(abi.encode(invalidConfig));

        // Test invalid output account
        invalidConfig = AavePositionManager.AavePositionManagerConfig({
            poolAddress: IPool(address(mockPool)),
            inputAccount: inputAccount,
            outputAccount: BaseAccount(payable(address(0))),
            supplyAsset: address(supplyToken),
            borrowAsset: address(borrowToken),
            referralCode: referralCode
        });

        vm.prank(owner);
        vm.expectRevert("Output account can't be zero address");
        aavePositionManager.updateConfig(abi.encode(invalidConfig));

        // Test invalid supply asset
        invalidConfig = AavePositionManager.AavePositionManagerConfig({
            poolAddress: IPool(address(mockPool)),
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            supplyAsset: address(0),
            borrowAsset: address(borrowToken),
            referralCode: referralCode
        });

        vm.prank(owner);
        vm.expectRevert("Supply asset can't be zero address");
        aavePositionManager.updateConfig(abi.encode(invalidConfig));

        // Test invalid borrow asset
        invalidConfig = AavePositionManager.AavePositionManagerConfig({
            poolAddress: IPool(address(mockPool)),
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            supplyAsset: address(supplyToken),
            borrowAsset: address(0),
            referralCode: referralCode
        });

        vm.prank(owner);
        vm.expectRevert("Borrow asset can't be zero address");
        aavePositionManager.updateConfig(abi.encode(invalidConfig));
    }

    function testUpdateConfig() public {
        // Create a new configuration with different values
        uint16 newReferralCode = 1;
        MockERC20 newSupplyToken = new MockERC20("New Supply Token", "NST");
        MockERC20 newBorrowToken = new MockERC20("New Borrow Token", "NBT");

        AavePositionManager.AavePositionManagerConfig memory newConfig = AavePositionManager.AavePositionManagerConfig({
            poolAddress: IPool(address(mockPool)),
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            supplyAsset: address(newSupplyToken),
            borrowAsset: address(newBorrowToken),
            referralCode: newReferralCode
        });

        // Update config as owner
        vm.prank(owner);
        aavePositionManager.updateConfig(abi.encode(newConfig));

        // Access components returned by the config() function
        (,,, address supplyAsset, address borrowAsset,) = aavePositionManager.config();

        // Verify the configuration was updated
        assertEq(supplyAsset, address(newSupplyToken));
        assertEq(borrowAsset, address(newBorrowToken));
    }

    function testUnauthorizedConfigUpdate() public {
        address unauthorized = makeAddr("unauthorized");

        AavePositionManager.AavePositionManagerConfig memory config = AavePositionManager.AavePositionManagerConfig({
            poolAddress: IPool(address(mockPool)),
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            supplyAsset: address(supplyToken),
            borrowAsset: address(borrowToken),
            referralCode: referralCode
        });

        vm.prank(unauthorized);
        vm.expectRevert();
        aavePositionManager.updateConfig(abi.encode(config));
    }

    // ============== Supply Tests ==============

    function testSupplyWithSpecificAmount() public {
        // Mint tokens to input account
        uint256 amount = 1000 * 10 ** 18;
        vm.prank(owner);
        supplyToken.mint(address(inputAccount), amount);

        // Execute supply as processor
        vm.prank(processor);
        aavePositionManager.supply(amount);
    }

    function testSupplyWithZeroAmount() public {
        // Mint tokens to input account
        uint256 balance = 500 * 10 ** 18;
        vm.prank(owner);
        supplyToken.mint(address(inputAccount), balance);

        // Execute supply with 0 (should use entire balance)
        vm.prank(processor);
        aavePositionManager.supply(0);
    }

    function testSupplyWithNoBalance() public {
        // Don't mint any tokens (zero balance)

        // Execute supply operation (should fail)
        vm.prank(processor);
        vm.expectRevert("No supply asset balance available");
        aavePositionManager.supply(100 * 10 ** 18);
    }

    function testSupplyWithInsufficientBalance() public {
        // Mint tokens to input account
        uint256 balance = 100 * 10 ** 18;
        vm.prank(owner);
        supplyToken.mint(address(inputAccount), balance);

        // Execute supply with amount larger than balance (should fail)
        vm.prank(processor);
        vm.expectRevert("Insufficient supply asset balance");
        aavePositionManager.supply(200 * 10 ** 18);
    }

    function testUnauthorizedSupply() public {
        address unauthorized = makeAddr("unauthorized");

        // Mint tokens to input account
        uint256 amount = 1000 * 10 ** 18;
        vm.prank(owner);
        supplyToken.mint(address(inputAccount), amount);

        // Attempt to supply as unauthorized user
        vm.prank(unauthorized);
        vm.expectRevert();
        aavePositionManager.supply(amount);
    }

    // ============== Borrow Tests ==============

    function testBorrow() public {
        // Execute borrow as processor
        uint256 amount = 500 * 10 ** 18;
        vm.prank(processor);
        aavePositionManager.borrow(amount);
    }

    function testUnauthorizedBorrow() public {
        address unauthorized = makeAddr("unauthorized");

        // Attempt to borrow as unauthorized user
        vm.prank(unauthorized);
        vm.expectRevert();
        aavePositionManager.borrow(100 * 10 ** 18);
    }

    // ============== Withdraw Tests ==============

    function testWithdraw() public {
        // Execute withdraw as processor
        uint256 amount = 300 * 10 ** 18;
        vm.prank(processor);
        aavePositionManager.withdraw(amount);
    }

    function testUnauthorizedWithdraw() public {
        address unauthorized = makeAddr("unauthorized");

        // Attempt to withdraw as unauthorized user
        vm.prank(unauthorized);
        vm.expectRevert();
        aavePositionManager.withdraw(100 * 10 ** 18);
    }

    // ============== Repay Tests ==============

    function testRepayWithSpecificAmount() public {
        // Mint tokens to input account
        uint256 amount = 200 * 10 ** 18;
        vm.prank(owner);
        borrowToken.mint(address(inputAccount), amount);

        // Execute repay as processor
        vm.prank(processor);
        aavePositionManager.repay(amount);
    }

    function testRepayWithZeroAmount() public {
        // Mint tokens to input account
        uint256 balance = 150 * 10 ** 18;
        vm.prank(owner);
        borrowToken.mint(address(inputAccount), balance);

        // Execute repay with 0 (should use entire balance)
        vm.prank(processor);
        aavePositionManager.repay(0);
    }

    function testRepayWithNoBalance() public {
        // Don't mint any tokens (zero balance)

        // Execute repay operation (should fail)
        vm.prank(processor);
        vm.expectRevert("No borrow asset balance available");
        aavePositionManager.repay(100 * 10 ** 18);
    }

    function testRepayWithInsufficientBalance() public {
        // Mint tokens to input account
        uint256 balance = 50 * 10 ** 18;
        vm.prank(owner);
        borrowToken.mint(address(inputAccount), balance);

        // Execute repay with amount larger than balance (should fail)
        vm.prank(processor);
        vm.expectRevert("Insufficient borrow asset balance");
        aavePositionManager.repay(100 * 10 ** 18);
    }

    function testUnauthorizedRepay() public {
        address unauthorized = makeAddr("unauthorized");

        // Mint tokens to input account
        uint256 amount = 100 * 10 ** 18;
        vm.prank(owner);
        borrowToken.mint(address(inputAccount), amount);

        // Attempt to repay as unauthorized user
        vm.prank(unauthorized);
        vm.expectRevert();
        aavePositionManager.repay(amount);
    }

    // ============== Repay With ATokens Tests ==============

    function testRepayWithATokens() public {
        // Execute repayWithATokens as processor
        uint256 amount = 100 * 10 ** 18;
        vm.prank(processor);
        aavePositionManager.repayWithATokens(amount);
    }

    function testUnauthorizedRepayWithATokens() public {
        address unauthorized = makeAddr("unauthorized");

        // Attempt to repayWithATokens as unauthorized user
        vm.prank(unauthorized);
        vm.expectRevert();
        aavePositionManager.repayWithATokens(100 * 10 ** 18);
    }
}
