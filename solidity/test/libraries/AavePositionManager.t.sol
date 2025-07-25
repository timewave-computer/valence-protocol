// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test, console} from "forge-std/src/Test.sol";
import {AavePositionManager} from "../../src/libraries/AavePositionManager.sol";
import {IPool} from "aave-v3-origin/interfaces/IPool.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {MockAavePool} from "../mocks/MockAavePool.sol";
import {MockERC20} from "../mocks/MockERC20.sol";
import {AToken} from "aave-v3-origin/protocol/tokenization/AToken.sol";
import {IAaveIncentivesController} from "aave-v3-origin/interfaces/IAaveIncentivesController.sol";

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
    address public rewardsController;
    address public aToken;
    address public debtToken;
    uint16 public referralCode = 0;

    // Setup function to initialize test environment
    function setUp() public {
        // Setup test addresses
        owner = makeAddr("owner");
        processor = makeAddr("processor");
        rewardsController = makeAddr("rewardsController");
        aToken = makeAddr("aToken");
        debtToken = makeAddr("debtToken");

        // Deploy mock tokens
        supplyToken = new MockERC20("Supply Token", "ST", 18);
        borrowToken = new MockERC20("Borrow Token", "BT", 18);

        // Create mock accounts
        vm.startPrank(owner);
        inputAccount = new BaseAccount(owner, new address[](0));
        outputAccount = new BaseAccount(owner, new address[](0));
        vm.stopPrank();

        // Deploy mock Aave pool
        mockPool = new MockAavePool();

        // Deploy AavePositionManager contract
        vm.startPrank(owner);
        vm.mockCall(
            address(mockPool),
            abi.encodeWithSignature("getReserveAToken(address)", address(supplyToken)),
            abi.encode(aToken)
        );
        vm.mockCall(
            address(mockPool),
            abi.encodeWithSignature("getReserveVariableDebtToken(address)", address(borrowToken)),
            abi.encode(debtToken)
        );
        vm.mockCall(aToken, abi.encodeWithSignature("getIncentivesController()"), abi.encode(rewardsController));

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
        MockERC20 newSupplyToken = new MockERC20("New Supply Token", "NST", 18);
        MockERC20 newBorrowToken = new MockERC20("New Borrow Token", "NBT", 18);

        AavePositionManager.AavePositionManagerConfig memory newConfig = AavePositionManager.AavePositionManagerConfig({
            poolAddress: IPool(address(mockPool)),
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            supplyAsset: address(newSupplyToken),
            borrowAsset: address(newBorrowToken),
            referralCode: newReferralCode
        });
        vm.mockCall(
            address(mockPool),
            abi.encodeWithSignature("getReserveAToken(address)", address(newSupplyToken)),
            abi.encode(aToken)
        );
        vm.mockCall(
            address(mockPool),
            abi.encodeWithSignature("getReserveVariableDebtToken(address)", address(newBorrowToken)),
            abi.encode(debtToken)
        );

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

    function testDerivedConfig() public view {
        (IAaveIncentivesController _rewardsController, address _aToken, address _debtToken) =
            aavePositionManager.derivedConfig();
        assertEq(address(_rewardsController), rewardsController);
        assertEq(_aToken, aToken);
        assertEq(_debtToken, debtToken);
    }

    function testDerivedConfigUpdate() public {
        vm.mockCall(address(aToken), abi.encodeWithSignature("getIncentivesController()"), abi.encode(address(0xd)));
        AavePositionManager.AavePositionManagerConfig memory newConfig = AavePositionManager.AavePositionManagerConfig({
            poolAddress: IPool(address(mockPool)),
            inputAccount: inputAccount,
            outputAccount: outputAccount,
            supplyAsset: address(supplyToken),
            borrowAsset: address(borrowToken),
            referralCode: referralCode
        });
        vm.prank(owner);
        aavePositionManager.updateConfig(abi.encode(newConfig));

        (IAaveIncentivesController _rewardsController, address _aToken, address _debtToken) =
            aavePositionManager.derivedConfig();
        assertEq(address(_rewardsController), address(0xd));
        assertEq(_aToken, aToken);
        assertEq(_debtToken, debtToken);
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

    // ============== Repay With Shares Tests ==============

    function testRepayWithShares() public {
        // Execute repayWithShares as processor
        uint256 amount = 100 * 10 ** 18;
        vm.prank(processor);
        aavePositionManager.repayWithShares(amount);
        (amount);
    }

    function testUnauthorizedRepayWithShares() public {
        address unauthorized = makeAddr("unauthorized");

        // Attempt to repayWithShares as unauthorized user
        vm.prank(unauthorized);
        vm.expectRevert();
        aavePositionManager.repayWithShares(100 * 10 ** 18);
    }

    //  ============== Rewards Tests ==============

    function testGetAllRewards() public {
        // Execute claimRewards as processor
        // given
        address[] memory assets = new address[](2);
        assets[0] = aToken;
        assets[1] = debtToken;
        address[] memory rewardTokens = new address[](2);
        rewardTokens[0] = address(0x11);
        rewardTokens[1] = address(0x12);
        uint256[] memory rewardAmounts = new uint256[](2);
        rewardAmounts[0] = 100;
        rewardAmounts[1] = 200;
        vm.mockCall(
            address(rewardsController),
            abi.encodeWithSignature("getAllUserRewards(address[],address)", assets, address(inputAccount)),
            abi.encode(rewardTokens, rewardAmounts)
        );

        // when
        // vm.prank(processor);
        (address[] memory _rewardTokens, uint256[] memory _rewardAmounts) = aavePositionManager.getAllRewards();

        // then
        assertEq(_rewardTokens.length, 2);
        assertEq(_rewardAmounts.length, 2);
        assertEq(_rewardTokens[0], rewardTokens[0]);
        assertEq(_rewardAmounts[0], rewardAmounts[0]);
        assertEq(_rewardTokens[1], rewardTokens[1]);
        assertEq(_rewardAmounts[1], rewardAmounts[1]);
    }
}
