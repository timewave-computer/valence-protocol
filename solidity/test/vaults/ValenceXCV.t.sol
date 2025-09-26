// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test, console} from "forge-std/src/Test.sol";
import {ValenceXCV} from "../../src/vaults/ValenceXCV.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {MockERC20} from "../mocks/MockERC20.sol";

// run with: forge test --match-path test/vaults/ValenceXCV.t.sol -vvv

contract ValenceXCVTest is Test {
    event Deposit(
        address indexed sender,
        address indexed owner,
        uint256 assets,
        uint256 shares
    );

    // contracts
    ValenceXCV internal vault;
    MockERC20 internal underlyingToken;
    BaseAccount internal depositAccount;

    // test addresses
    address owner = address(1);
    address strategist = address(2);
    address user1 = address(3);
    address user2 = address(4);

    // vault config
    uint256 initialSharePrice = 10 ** 18; // 1:1 initial rate

    // start user balance
    uint256 startUserBalance = 1000 * 10 ** 18;

    uint8 UNDERLYING_PRECISION_DECIMALS = 18;
    uint256 ONE_SHARE = 10 ** UNDERLYING_PRECISION_DECIMALS;

    function setUp() public {
        vm.startPrank(owner);

        // deploy mock token and deposit account
        underlyingToken = new MockERC20(
            "Test Token",
            "TST",
            UNDERLYING_PRECISION_DECIMALS
        );
        depositAccount = new BaseAccount(owner, new address[](0));

        vault = new ValenceXCV();

        // initialize the vault
        vault.initialize(
            owner,
            strategist,
            address(underlyingToken),
            address(depositAccount),
            "ValenceXCV",
            "vXCV",
            initialSharePrice
        );

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
        assertEq(vault.owner(), owner);
        assertEq(vault.strategist(), strategist);
        assertEq(vault.depositAccount(), address(depositAccount));
        assertEq(vault.name(), "ValenceXCV");
        assertEq(vault.symbol(), "vXCV");
        assertEq(vault.sharePrice(), initialSharePrice);
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

        vm.prank(strategist);
        vault.setSharePrice(2 * price_0);

        uint256 price_1 = vault.sharePrice();

        assertNotEq(price_0, price_1);
        assertEq(2 * price_0, price_1);
    }

    function testDeposit() public {
        assertEq(vault.totalSupply(), 0);
        assertEq(underlyingToken.balanceOf(address(depositAccount)), 0);
        assertEq(underlyingToken.balanceOf(user1), startUserBalance);
        assertEq(underlyingToken.balanceOf(address(vault)), 0);
        assertEq(vault.balanceOf(user1), 0);
        assertEq(vault.sharePrice(), initialSharePrice);

        uint256 userDepositAmount = startUserBalance / 2;

        uint256 expectedShares = (userDepositAmount * ONE_SHARE) /
            initialSharePrice;

        vm.prank(user1);
        vm.expectEmit(true, true, true, true, address(vault));
        emit Deposit(user1, user1, userDepositAmount, expectedShares);
        uint256 shares = vault.deposit(userDepositAmount, user1);
        vm.stopPrank();

        assertNotEq(shares, 0);
        assertEq(vault.balanceOf(user1), shares);
        assertEq(vault.totalSupply(), shares);
        assertEq(
            underlyingToken.balanceOf(address(depositAccount)),
            userDepositAmount
        );
        assertEq(underlyingToken.balanceOf(address(vault)), 0);
        assertEq(
            underlyingToken.balanceOf(address(user1)),
            startUserBalance - userDepositAmount
        );
    }
}
