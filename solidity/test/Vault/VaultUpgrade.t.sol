// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {VaultHelper} from "./VaultHelper.t.sol";
import {ValenceVault} from "../../src/libraries/ValenceVault.sol";

/**
 * @title ValenceVaultV2
 * @dev Upgraded version of ValenceVault with emergency shutdown functionality.
 */
contract ValenceVaultV2 is ValenceVault {
    // New state variable
    bool public emergencyShutdown;

    // Event for the new feature
    event EmergencyShutdownSet(bool status);
    event V2Initialized(bool initialShutdownState);

    /**
     * @notice Initialize the V2 contract with specific settings
     * @dev Called via upgradeToAndCall
     * @param _initialShutdownState The initial emergency shutdown state
     */
    function initializeV2(bool _initialShutdownState) external onlyOwner {
        emergencyShutdown = _initialShutdownState;
        emit V2Initialized(_initialShutdownState);
    }

    /**
     * @notice Set the emergency shutdown status
     * @dev Only callable by the vault owner
     * @param _status New emergency shutdown status (true = enabled, false = disabled)
     */
    function setEmergencyShutdown(bool _status) external onlyOwner {
        emergencyShutdown = _status;
        emit EmergencyShutdownSet(_status);
    }

    /**
     * @notice Check if deposits are allowed based on emergency shutdown status
     * @dev This can be used externally to verify if deposits would succeed
     * @return bool True if deposits are allowed, false otherwise
     */
    function depositsAllowed() public view returns (bool) {
        return !emergencyShutdown;
    }

    /**
     * @notice Verify that the vault is not in emergency shutdown
     * @dev This function can be called before attempting deposits
     */
    function checkEmergencyShutdown() public view {
        require(!emergencyShutdown, "Vault is in emergency shutdown");
    }
}

contract ValenceVaultUpgradeTest is VaultHelper {
    ValenceVaultV2 internal vaultV2Implementation;

    function test_UpgradeToV2WithInitData() public {
        // First do a deposit to ensure we have state to preserve
        vm.startPrank(user);
        uint256 depositAmount = 1_000_000_000;
        vault.deposit(depositAmount, user);
        uint256 userShares = vault.balanceOf(user);
        assertGt(userShares, 0);
        vm.stopPrank();

        // Deploy V2 implementation
        vm.startPrank(owner);
        vaultV2Implementation = new ValenceVaultV2();

        // Prepare initialization data - start with emergency shutdown enabled
        bool initialShutdownState = true;
        bytes memory initData = abi.encodeWithSelector(ValenceVaultV2.initializeV2.selector, initialShutdownState);

        // Upgrade and initialize in one step
        vault.upgradeToAndCall(address(vaultV2Implementation), initData);
        vm.stopPrank();

        // Cast to V2 to access new functions
        ValenceVaultV2 vaultV2 = ValenceVaultV2(address(vault));

        // Verify that initialization worked
        assertTrue(vaultV2.emergencyShutdown(), "Emergency shutdown should be initialized to true");

        // Verify that state was preserved
        assertEq(vaultV2.balanceOf(user), userShares, "User shares should be preserved after upgrade");

        // Check helper functions with emergency shutdown active
        vm.startPrank(user);
        assertFalse(vaultV2.depositsAllowed(), "depositsAllowed should return false during emergency shutdown");
        vm.expectRevert("Vault is in emergency shutdown");
        vaultV2.checkEmergencyShutdown();
        vm.stopPrank();

        // Disable emergency shutdown
        vm.prank(owner);
        vaultV2.setEmergencyShutdown(false);

        // Verify helper functions after disabling emergency shutdown
        vm.startPrank(user);
        assertTrue(vaultV2.depositsAllowed(), "depositsAllowed should return true after emergency shutdown is disabled");
        vaultV2.checkEmergencyShutdown(); // Should not revert

        // Now we can deposit (since actual guards aren't in place, this is just to complete the flow)
        uint256 sharesBefore = vaultV2.balanceOf(user);
        vault.deposit(depositAmount, user);
        assertGt(vaultV2.balanceOf(user), sharesBefore, "User should have more shares after deposit");
        vm.stopPrank();
    }

    function test_UpgradeToV2WithoutEmergencyShutdown() public {
        // Deploy V2 implementation
        vm.startPrank(owner);
        vaultV2Implementation = new ValenceVaultV2();

        // Prepare initialization data - start with emergency shutdown disabled
        bool initialShutdownState = false;
        bytes memory initData = abi.encodeWithSelector(ValenceVaultV2.initializeV2.selector, initialShutdownState);

        // Upgrade and initialize in one step
        vault.upgradeToAndCall(address(vaultV2Implementation), initData);
        vm.stopPrank();

        // Cast to V2 to access new functions
        ValenceVaultV2 vaultV2 = ValenceVaultV2(address(vault));

        // Verify that initialization worked
        assertFalse(vaultV2.emergencyShutdown(), "Emergency shutdown should be initialized to false");

        // Verify helper functions with emergency shutdown disabled
        vm.startPrank(user);
        assertTrue(vaultV2.depositsAllowed(), "depositsAllowed should return true with emergency shutdown disabled");
        vaultV2.checkEmergencyShutdown(); // Should not revert

        // Make a deposit (regular deposit works since we don't have guards in the deposit function)
        uint256 depositAmount = 1_000_000_000;
        uint256 sharesBefore = vaultV2.balanceOf(user);
        vault.deposit(depositAmount, user);
        uint256 sharesAfter = vaultV2.balanceOf(user);
        assertGt(sharesAfter, sharesBefore, "Deposit should work with emergency shutdown disabled");
        vm.stopPrank();
    }
}
