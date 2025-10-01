// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test, console} from "forge-std/src/Test.sol";
import {ValenceXCV} from "../../src/vaults/ValenceXCV.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {MockERC20} from "../mocks/MockERC20.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

// run with: forge test --match-path test/vaults/ValenceXCV.t.sol -vvv

contract ValenceXCVTest is Test {
    event Deposit(address indexed sender, address indexed owner, uint256 assets, uint256 shares);
    event OperatorSet(address indexed controller, address indexed operator, bool approved);
    event SharePriceUpdated(uint256 indexed sharePrice, uint256 indexed updateTimestamp);

    // contracts
    ValenceXCV internal vault;
    MockERC20 internal underlyingToken;
    BaseAccount internal depositAccount;

    // test addresses
    address owner = address(1);
    address strategist = address(2);
    address user1 = address(3);
    address user2 = address(4);
    address operator = address(5);

    // vault config
    uint256 initialSharePrice = 10 ** 18; // 1:1 initial rate
    uint256 oneHourSecs = 1 hours;

    // start user balance
    uint256 startUserBalance = 1000 * 10 ** 18;

    uint8 UNDERLYING_PRECISION_DECIMALS = 18;
    uint256 ONE_SHARE = 10 ** UNDERLYING_PRECISION_DECIMALS;
    uint256 MAX_PRICE_CHANGE = 500; // 5% max price change

    function setUp() public {
        vm.startPrank(owner);

        // deploy mock token and deposit account
        underlyingToken = new MockERC20("Test Token", "TST", UNDERLYING_PRECISION_DECIMALS);
        depositAccount = new BaseAccount(owner, new address[](0));

        ValenceXCV vaultImpl = new ValenceXCV();

        bytes memory initData = abi.encodeWithSelector(
            ValenceXCV.initialize.selector,
            owner,
            strategist,
            address(underlyingToken),
            address(depositAccount),
            "ValenceXCV",
            "vXCV",
            initialSharePrice,
            oneHourSecs,
            MAX_PRICE_CHANGE
        );

        // Create proxy via create2 and initialize in one step
        bytes memory proxyCreationCode =
            abi.encodePacked(type(ERC1967Proxy).creationCode, abi.encode(address(vaultImpl), initData));

        address proxyAddress;
        assembly {
            proxyAddress := create2(0, add(proxyCreationCode, 0x20), mload(proxyCreationCode), 0)
        }

        vault = ValenceXCV(payable(proxyAddress));

        underlyingToken.mint(user1, startUserBalance);
        underlyingToken.mint(user2, startUserBalance);

        vm.stopPrank();

        // approve deposit tokens to be spent by the vault for users
        vm.prank(user1);
        underlyingToken.approve(address(vault), type(uint256).max);

        vm.prank(user2);
        underlyingToken.approve(address(vault), type(uint256).max);
    }

    function testSetUpVault() public view {
        // initial vault state
        assertEq(vault.totalSupply(), 0);
        assertEq(vault.balanceOf(user1), 0);
        assertEq(vault.balanceOf(user2), 0);
        assertEq(vault.balanceOf(address(vault)), 0);
        assertEq(vault.balanceOf(address(depositAccount)), 0);
        assertEq(vault.sharePrice(), initialSharePrice);
        assertEq(vault.owner(), owner);
        assertEq(vault.strategist(), strategist);
        assertEq(vault.depositAccount(), address(depositAccount));
        assertEq(vault.name(), "ValenceXCV");
        assertEq(vault.symbol(), "vXCV");

        // underlying token balances
        assertEq(underlyingToken.balanceOf(address(depositAccount)), 0);
        assertEq(underlyingToken.balanceOf(user1), startUserBalance);
        assertEq(underlyingToken.balanceOf(user2), startUserBalance);
        assertEq(underlyingToken.balanceOf(operator), 0);
        assertEq(underlyingToken.balanceOf(address(vault)), 0);
    }

    function testSetSharePriceUnauthorized() public {
        vm.prank(user1);
        vm.expectRevert(ValenceXCV.OnlyStrategistAllowed.selector);
        vault.setSharePrice(2 * initialSharePrice);
    }

    function testSetSharePriceInvalidAmount() public {
        vm.prank(strategist);
        vm.expectRevert(ValenceXCV.InvalidSharePrice.selector);
        vault.setSharePrice(0);
    }

    function testSetSharePrice() public {
        uint256 price_0 = vault.sharePrice();
        uint256 small_change = price_0 + (price_0 * 2) / 100;
        uint256 update_timestamp_0 = vault.lastUpdateTimestamp();

        vm.warp(update_timestamp_0 + 1);
        vm.prank(strategist);
        vm.expectEmit(true, true, true, true, address(vault));
        emit SharePriceUpdated(small_change, update_timestamp_0 + 1);
        vault.setSharePrice(small_change);

        uint256 price_1 = vault.sharePrice();
        uint256 update_timestamp_1 = vault.lastUpdateTimestamp();

        assertEq(update_timestamp_1, update_timestamp_0 + 1);
        assertNotEq(price_0, price_1);
        assertEq(small_change, price_1);
    }

    function testSetSharePriceRevertOnPriceIncreaseTooLarge() public {
        uint256 price_0 = vault.sharePrice();
        // 6% increase, while max is 5%
        uint256 large_increase = price_0 + (price_0 * 6) / 100;

        vm.prank(strategist);
        vm.expectRevert(ValenceXCV.SharePriceChangeDeltaTooLarge.selector);
        vault.setSharePrice(large_increase);
    }

    function testSetSharePriceRevertOnPriceDecreaseTooLarge() public {
        uint256 price_0 = vault.sharePrice();
        // 6% decrease, while max is 5%
        uint256 large_decrease = price_0 - (price_0 * 6) / 100;

        vm.prank(strategist);
        vm.expectRevert(ValenceXCV.SharePriceChangeDeltaTooLarge.selector);
        vault.setSharePrice(large_decrease);
    }

    function testDeposit4626() public {
        uint256 userDepositAmount = startUserBalance / 2;

        uint256 expectedShares = (userDepositAmount * ONE_SHARE) / initialSharePrice;

        vm.prank(user1);
        vm.expectEmit(true, true, true, true, address(vault));
        emit Deposit(user1, user1, userDepositAmount, expectedShares);
        uint256 shares = vault.deposit(userDepositAmount, user1);
        vm.stopPrank();

        assertNotEq(shares, 0);
        assertEq(vault.balanceOf(user1), shares);
        assertEq(vault.totalSupply(), shares);
        assertEq(underlyingToken.balanceOf(address(depositAccount)), userDepositAmount);
        assertEq(underlyingToken.balanceOf(address(vault)), 0);
        assertEq(underlyingToken.balanceOf(address(user1)), startUserBalance - userDepositAmount);
    }

    function testDepositStalenessChecks() public {
        // expire the share price
        vm.warp(block.timestamp + 2 days);

        // attempt a deposit and assert that it reverts
        vm.expectRevert(ValenceXCV.StaleSharePrice.selector);
        vm.prank(user1);
        vault.deposit(startUserBalance, user1);
        vm.stopPrank();

        // update the share price
        uint256 currentSharePrice = vault.sharePrice();
        uint256 small_change = (currentSharePrice * 2) / 100;
        vm.prank(strategist);
        vault.setSharePrice(currentSharePrice + small_change);
        vm.stopPrank();

        uint256 userDepositAmount = startUserBalance / 2;

        uint256 expectedShares = (userDepositAmount * ONE_SHARE) / vault.sharePrice();

        // perform a deposit with the new rate
        vm.prank(user1);
        vm.expectEmit(true, true, true, true, address(vault));
        emit Deposit(user1, user1, userDepositAmount, expectedShares);
        uint256 shares = vault.deposit(userDepositAmount, user1);
        vm.stopPrank();

        assertNotEq(shares, 0);
    }

    function testDeposit7540NotControllerNotOperator() public {
        // user1 tries to deposit on user2 behalf without approval
        vm.prank(user1);
        vm.expectRevert(ValenceXCV.NotControllerOrOperator.selector);
        vault.deposit(startUserBalance, user1, user2);
    }

    function testDeposit7540Operator() public {
        assertFalse(vault.isOperator(user1, operator));
        // first approve the operator
        vm.prank(user1);
        vm.expectEmit(true, true, true, true, address(vault));
        emit OperatorSet(user1, operator, true);
        vault.setOperator(operator, true);
        vm.stopPrank();

        assertTrue(vault.isOperator(user1, operator));

        uint256 userDepositAmount = startUserBalance / 2;

        uint256 expectedShares = (userDepositAmount * ONE_SHARE) / initialSharePrice;
        vm.prank(operator);
        vm.expectEmit(true, true, true, true, address(vault));
        emit Deposit(operator, user1, userDepositAmount, expectedShares);
        uint256 shares = vault.deposit(userDepositAmount, user1, user1);
        vm.stopPrank();

        assertNotEq(shares, 0);
        assertEq(vault.balanceOf(user1), shares);
        assertEq(vault.totalSupply(), shares);
        assertEq(underlyingToken.balanceOf(address(depositAccount)), userDepositAmount);
        assertEq(underlyingToken.balanceOf(address(vault)), 0);
        assertEq(underlyingToken.balanceOf(address(user1)), startUserBalance - userDepositAmount);
    }

    function testDeposit7540Controller() public {
        uint256 userDepositAmount = startUserBalance / 2;

        uint256 expectedShares = (userDepositAmount * ONE_SHARE) / initialSharePrice;

        vm.prank(user1);
        vm.expectEmit(true, true, true, true, address(vault));
        emit Deposit(user1, user1, userDepositAmount, expectedShares);
        uint256 shares = vault.deposit(userDepositAmount, user1, user1);
        vm.stopPrank();

        assertNotEq(shares, 0);
        assertEq(vault.balanceOf(user1), shares);
        assertEq(vault.totalSupply(), shares);
        assertEq(underlyingToken.balanceOf(address(depositAccount)), userDepositAmount);
        assertEq(underlyingToken.balanceOf(address(vault)), 0);
        assertEq(underlyingToken.balanceOf(address(user1)), startUserBalance - userDepositAmount);
    }

    function testSetOperator() public {
        assertFalse(vault.isOperator(user1, operator));

        vm.prank(user1);
        vm.expectEmit(true, true, true, true, address(vault));
        emit OperatorSet(user1, operator, true);
        vault.setOperator(operator, true);
        vm.stopPrank();

        assertTrue(vault.isOperator(user1, operator));

        vm.prank(user1);
        vm.expectEmit(true, true, true, true, address(vault));
        emit OperatorSet(user1, operator, false);
        vault.setOperator(operator, false);
        vm.stopPrank();

        assertFalse(vault.isOperator(user1, operator));
    }
}
