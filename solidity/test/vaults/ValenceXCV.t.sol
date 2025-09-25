// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test, console} from "forge-std/src/Test.sol";
import {ValenceXCV} from "../../src/vaults/ValenceXCV.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {MockERC20} from "../mocks/MockERC20.sol";

// run with: forge test --match-path test/vaults/ValenceXCV.t.sol -vvv

contract ValenceXCVTest is Test {
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

    function setUp() public {
        vm.startPrank(owner);

        // deploy mock token and deposit account
        underlyingToken = new MockERC20("Test Token", "TST", 18);
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

        underlyingToken.mint(user1, 1000 * 10 ** 18);
        underlyingToken.mint(user2, 1000 * 10 ** 18);

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
}
