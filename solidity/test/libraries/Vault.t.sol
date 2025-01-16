// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {ValenceVault} from "../../src/libraries/ValenceVault.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {MockERC20} from "../mocks/MockERC20.sol";
import {Ownable} from "@openzeppelin-contracts/access/Ownable.sol";

contract VaultTest is Test {
    // Test contracts and addresses
    ValenceVault vault;
    BaseAccount depositAccount;
    BaseAccount withdrawAccount;
    MockERC20 token;

    address owner = address(1);
    address processor = address(2);
    address strategist = address(3);

    /**
     * @dev Setup test environment
     * Deploys token, accounts and forwarder with initial config
     */
    function setUp() public {
        // Set initial block timestamp and height
        vm.warp(5000);
        vm.roll(100);

        vm.startPrank(owner);
        // Mock underlying token for the vault
        token = new MockERC20("Test Token", "TEST");

        // Create accounts after forwarder
        depositAccount = new BaseAccount(owner, new address[](0));
        withdrawAccount = new BaseAccount(owner, new address[](0));

        ValenceVault.VaultConfig memory config = ValenceVault.VaultConfig(
            depositAccount,
            withdrawAccount
        );

        vault = new ValenceVault(
            owner,
            processor,
            strategist,
            abi.encode(config),
            address(token),
            "Valence Vault Token",
            "VVT"
        );
        withdrawAccount.approveLibrary(address(vault));
        vm.stopPrank();
    }

    function testUpdateConfig() public {
        vm.startPrank(owner);

        BaseAccount newDepositAccount = new BaseAccount(
            owner,
            new address[](0)
        );

        ValenceVault.VaultConfig memory newConfig = ValenceVault.VaultConfig(
            newDepositAccount,
            withdrawAccount
        );

        vault.updateConfig(abi.encode(newConfig));

        (BaseAccount depAcc, ) = vault.config();

        assert(depAcc == newDepositAccount);

        vm.stopPrank();
    }

// TODO: Change test once we change the vault contract logic
    function testTotalAssets() public {
        vm.startPrank(owner);

        token.mint(address(vault), 1000);

        uint256 totalAssets = vault.totalAssets();

        assert(totalAssets == 1000);
    }
}
