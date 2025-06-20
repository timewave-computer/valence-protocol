// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test, console} from "forge-std/src/Test.sol";
import {CompoundV3PositionManager} from "../src/libraries/CompoundV3PositionManager.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {BaseAccount} from "../src/accounts/BaseAccount.sol";
import {Processor} from "../src/processor/Processor.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {CometMainInterface} from "../src/libraries/interfaces/compoundV3/CometMainInterface.sol";

contract CompoundV3PositionManagerScript is Test {
    // Contract under test
    CompoundV3PositionManager public compoundV3PositionManager;

    // Mock contracts
    BaseAccount public inputAccount;
    BaseAccount public outputAccount;
    CometMainInterface public comet = CometMainInterface(0xc3d688B66703497DAA19211EEdff47f25384cdc3);
    IERC20 public baseToken;

    // current states
    uint256 balanceOfCometBefore;

    // Test addresses
    address public owner;
    address public processor;

    // run function that tests all functionalities
    function run() public {
        vm.createSelectFork("https://mainnet.gateway.tenderly.co", 22638149); // Adjust the block number as needed
        // Setup test addresses
        owner = makeAddr("owner");
        _setUpSystem();

        vm.startPrank(owner);
        inputAccount = new BaseAccount(owner, new address[](0));
        outputAccount = new BaseAccount(owner, new address[](0));
        vm.stopPrank();

        // Deploy CompoundV3PositionManager contract
        vm.startPrank(owner);

        // Create and encode config directly
        CompoundV3PositionManager.CompoundV3PositionManagerConfig memory config = CompoundV3PositionManager
            .CompoundV3PositionManagerConfig({
            inputAccount: BaseAccount(payable(address(inputAccount))),
            outputAccount: BaseAccount(payable(address(outputAccount))),
            baseAsset: address(baseToken),
            marketProxyAddress: address(comet)
        });

        compoundV3PositionManager = new CompoundV3PositionManager(owner, processor, abi.encode(config));
        inputAccount.approveLibrary(address(compoundV3PositionManager));
        vm.stopPrank();

        _fetchStates();
        _label();

        test_GivenPositionIsCreated_WhenWithdrawAfterOneWeek_ThenWithdrawWithInterest();
    }

    function test_GivenPositionIsCreated_WhenWithdrawAfterOneWeek_ThenWithdrawWithInterest() internal {
        // given
        uint256 dealAmount = 1000e6; // 1000 USDC
        deal(address(baseToken), address(inputAccount), dealAmount); // 1000 USDC

        vm.prank(processor);
        compoundV3PositionManager.supply(0);

        uint256 inputBalance = comet.balanceOf(address(inputAccount));

        // when
        _skipTime(1 weeks);

        uint256 interest = comet.balanceOf(address(inputAccount)) - inputBalance;
        assertGt(interest, 0, "Interest should be accrued over time");

        vm.prank(processor);
        compoundV3PositionManager.withdraw(0);

        // then
        assertEq(comet.balanceOf(address(inputAccount)), 0);
        assertEq(
            baseToken.balanceOf(address(outputAccount)),
            interest + inputBalance,
            "Output account should receive accrued interest"
        );
    }

    function _setUpSystem() internal {
        processor = makeAddr("processor");
        baseToken = IERC20(comet.baseToken());
    }

    function _fetchStates() internal {
        balanceOfCometBefore = baseToken.balanceOf(address(comet));
    }

    function _label() internal {
        vm.label(address(inputAccount), "inputAccount");
        vm.label(address(outputAccount), "outputAccount");
        vm.label(address(baseToken), "baseToken");
        vm.label(address(comet), "comet");
        vm.label(address(compoundV3PositionManager), "CompoundV3PositionManager");
    }

    function _skipTime(uint256 second) internal {
        uint256 ts = block.timestamp;
        uint256 newTs = ts + second;
        vm.warp(newTs);
        vm.roll(block.number + (newTs - ts) / 12); // Assuming 12 seconds per block
        _fetchStates();
    }
}
