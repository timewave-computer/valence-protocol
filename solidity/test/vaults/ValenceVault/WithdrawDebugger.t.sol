// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/extensions/ERC4626.sol";
import {ValenceVault} from "../../../src/vaults/ValenceVault.sol";

contract WithdrawDebugger is Test {
    function debugWithdrawRequest(ValenceVault vault, address owner)
        public
        view
        returns (bool isClaimable, string memory failureReason)
    {
        // Get the withdrawal request
        (, uint64 claimTime, uint32 maxLossBps,, uint32 updateId,, uint256 sharesAmount) =
            vault.userWithdrawRequest(owner);

        // Check if request exists
        if (sharesAmount == 0) {
            return (false, "No active withdraw request");
        }

        // Check timing
        if (block.timestamp < claimTime) {
            return (
                false,
                string.concat(
                    "Lockup period not elapsed. Current time: ",
                    vm.toString(block.timestamp),
                    " Claim time: ",
                    vm.toString(claimTime)
                )
            );
        }

        // Get update info
        (uint256 withdrawRate,, uint32 updateWithdrawFee) = vault.updateInfos(updateId);

        // Check rates and potential loss
        uint256 currentRate = vault.redemptionRate();
        (,,, bool paused) = vault.packedValues();

        if (currentRate < withdrawRate) {
            uint256 currentWithdrawRate = currentRate - updateWithdrawFee;
            uint256 lossBps = ((withdrawRate - currentWithdrawRate) * 10000) / withdrawRate;

            if (lossBps > maxLossBps) {
                return (
                    false,
                    string.concat(
                        "Loss exceeds maximum. Current loss: ",
                        vm.toString(lossBps),
                        " Max allowed: ",
                        vm.toString(maxLossBps)
                    )
                );
            }
        }

        if (paused) {
            return (false, "Vault is paused");
        }

        return (true, "Withdraw should be claimable");
    }

    function debugVaultState(ValenceVault vault, address withdrawAccount)
        public
        view
        returns (
            uint256 totalAssets,
            uint256 totalShares,
            uint256 withdrawBalance,
            uint256 redemptionRate,
            uint256 maxHistoricalRate
        )
    {
        totalAssets = vault.totalAssets();
        totalShares = vault.totalSupply();
        withdrawBalance = IERC20(vault.asset()).balanceOf(withdrawAccount);
        redemptionRate = vault.redemptionRate();
        maxHistoricalRate = vault.maxHistoricalRate();
    }
}
