// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test, console} from "forge-std/src/Test.sol";
import {CompoundV3PositionManager} from "../../src/libraries/CompoundV3PositionManager.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {MockERC20} from "../mocks/MockERC20.sol";
import {MockCompoundV3Market} from "../mocks/MockCompoundV3Market.sol";
import {MockBaseAccount} from "../mocks/MockBaseAccount.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {CometMainInterface} from "../../src/libraries/interfaces/compoundV3/CometMainInterface.sol";

contract CompoundV3PositionManagerTest is Test {
    // Contract under test
    CompoundV3PositionManager public compoundV3PositionManager;

    // Mock contracts
    MockBaseAccount public inputAccount;
    MockBaseAccount public outputAccount;
    MockERC20 public baseToken;
    address public marketProxyAddress;

    // Test addresses
    address public owner;
    address public processor;

    // Setup function to initialize test environment
    function setUp() public {
        // Setup test addresses
        owner = makeAddr("owner");
        processor = makeAddr("processor");

        // Deploy mock tokens
        baseToken = new MockERC20("Base Token", "BT", 18);
        marketProxyAddress = address(new MockCompoundV3Market(address(baseToken)));
        // Create mock accounts
        vm.startPrank(owner);
        inputAccount = new MockBaseAccount();
        outputAccount = new MockBaseAccount();
        vm.stopPrank();

        // Deploy CompoundV3PositionManager contract
        vm.startPrank(owner);

        // Create and encode config directly
        CompoundV3PositionManager.CompoundV3PositionManagerConfig memory config = CompoundV3PositionManager
            .CompoundV3PositionManagerConfig({
            inputAccount: BaseAccount(payable(address(inputAccount))),
            outputAccount: BaseAccount(payable(address(outputAccount))),
            baseAsset: address(baseToken),
            marketProxyAddress: marketProxyAddress
        });

        compoundV3PositionManager = new CompoundV3PositionManager(owner, processor, abi.encode(config));
        // inputAccount.approveLibrary(address(compoundV3PositionManager));
        vm.stopPrank();

        vm.label(address(inputAccount), "inputAccount");
        vm.label(address(outputAccount), "outputAccount");
        vm.label(address(baseToken), "baseToken");
        vm.label(marketProxyAddress, "marketProxyAddress");
    }

    // ============== Configuration Tests ==============

    function test_GivenValidConfig_WhenContractIsDeployed_ThenConfigIsSet() public view {
        (
            BaseAccount actualInputAccount,
            BaseAccount actualOutputAccount,
            address actualBaseAsset,
            address actualMarketProxyAddress
        ) = compoundV3PositionManager.config();

        assertEq(address(actualInputAccount), address(inputAccount));
        assertEq(address(actualOutputAccount), address(outputAccount));
        assertEq(actualBaseAsset, address(baseToken));
        assertEq(actualMarketProxyAddress, marketProxyAddress);
    }

    function test_GivenValidConfig_WhenUpdateConfigIsCalled_ThenConfigIsUpdated() public {
        // given
        MockERC20 newBaseToken = new MockERC20("New Base Token", "NBT", 18);
        CompoundV3PositionManager.CompoundV3PositionManagerConfig memory newConfig = CompoundV3PositionManager
            .CompoundV3PositionManagerConfig({
            inputAccount: new BaseAccount(owner, new address[](0)),
            outputAccount: new BaseAccount(owner, new address[](0)),
            baseAsset: address(newBaseToken),
            marketProxyAddress: address(new MockCompoundV3Market(address(newBaseToken)))
        });

        // when
        vm.prank(owner);
        compoundV3PositionManager.updateConfig(abi.encode(newConfig));

        // then
        (
            BaseAccount actualInputAccount,
            BaseAccount actualOutputAccount,
            address actualBaseAsset,
            address actualMarketProxyAddress
        ) = compoundV3PositionManager.config();
        assertEq(address(actualInputAccount), address(newConfig.inputAccount));
        assertEq(address(actualOutputAccount), address(newConfig.outputAccount));
        assertEq(actualBaseAsset, newConfig.baseAsset);
        assertEq(actualMarketProxyAddress, newConfig.marketProxyAddress);
    }

    function test_RevertUpdateConfig_WithInvalidConfig_WhenInputAccountIsZeroAddress() public {
        // given
        CompoundV3PositionManager.CompoundV3PositionManagerConfig memory newConfig = CompoundV3PositionManager
            .CompoundV3PositionManagerConfig({
            inputAccount: BaseAccount(payable(address(0))),
            outputAccount: new BaseAccount(owner, new address[](0)),
            baseAsset: vm.randomAddress(),
            marketProxyAddress: makeAddr("newMarketProxyAddress")
        });

        // expect
        vm.expectRevert("Input account can't be zero address");

        // when
        vm.prank(owner);
        compoundV3PositionManager.updateConfig(abi.encode(newConfig));
    }

    function test_RevertUpdateConfig_WithInvalidConfig_WhenOutputAccountIsZeroAddress() public {
        // given
        CompoundV3PositionManager.CompoundV3PositionManagerConfig memory newConfig = CompoundV3PositionManager
            .CompoundV3PositionManagerConfig({
            inputAccount: new BaseAccount(owner, new address[](0)),
            outputAccount: BaseAccount(payable(address(0))),
            baseAsset: vm.randomAddress(),
            marketProxyAddress: makeAddr("newMarketProxyAddress")
        });

        // expect
        vm.expectRevert("Output account can't be zero address");

        // when
        vm.prank(owner);
        compoundV3PositionManager.updateConfig(abi.encode(newConfig));
    }

    function test_RevertUpdateConfig_WithInvalidConfig_WhenMarketBaseAssetAndGivenBaseAssetAreNotSame() public {
        // given
        CompoundV3PositionManager.CompoundV3PositionManagerConfig memory newConfig = CompoundV3PositionManager
            .CompoundV3PositionManagerConfig({
            inputAccount: new BaseAccount(owner, new address[](0)),
            outputAccount: new BaseAccount(owner, new address[](0)),
            baseAsset: vm.randomAddress(),
            marketProxyAddress: marketProxyAddress
        });

        // expect
        vm.expectRevert("Market base asset and given base asset are not same");

        // when
        vm.prank(owner);
        compoundV3PositionManager.updateConfig(abi.encode(newConfig));
    }

    function test_RevertUpdateConfig_WithInvalidConfig_WhenMarketProxyAddressIsZeroAddress() public {
        // given
        CompoundV3PositionManager.CompoundV3PositionManagerConfig memory newConfig = CompoundV3PositionManager
            .CompoundV3PositionManagerConfig({
            inputAccount: new BaseAccount(owner, new address[](0)),
            outputAccount: new BaseAccount(owner, new address[](0)),
            baseAsset: address(baseToken),
            marketProxyAddress: address(0)
        });

        // expect
        vm.expectRevert("Market proxy address can't be zero address");

        // when
        vm.prank(owner);
        compoundV3PositionManager.updateConfig(abi.encode(newConfig));
    }

    function test_RevertUpdateConfig_WithUnauthorized_WhenCallerIsNotOwner() public {
        // given
        address unauthorized = makeAddr("unauthorized");
        MockERC20 newBaseToken = new MockERC20("New Base Token", "NBT", 18);
        CompoundV3PositionManager.CompoundV3PositionManagerConfig memory newConfig = CompoundV3PositionManager
            .CompoundV3PositionManagerConfig({
            inputAccount: new BaseAccount(owner, new address[](0)),
            outputAccount: new BaseAccount(owner, new address[](0)),
            baseAsset: address(newBaseToken),
            marketProxyAddress: address(new MockCompoundV3Market(address(newBaseToken)))
        });

        // expect
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, unauthorized));

        // when
        vm.prank(unauthorized);
        compoundV3PositionManager.updateConfig(abi.encode(newConfig));
    }

    // ============== Supply Tests ==============

    function test_GivenValidAmount_WhenSupplyIsCalled_ThenSupplyAmountIsEqual() public {
        // given
        uint256 exactAmount = 1000 * 10 ** 18;
        vm.prank(owner);
        baseToken.mint(address(inputAccount), exactAmount * 2);

        // when
        vm.prank(processor);
        compoundV3PositionManager.supply(exactAmount);

        // then
        vm.expectRevert();
        MockBaseAccount(inputAccount).executeParams(2);
        (address target, uint256 amount, bytes memory data) = MockBaseAccount(inputAccount).executeParams(1);
        assertEq(target, marketProxyAddress, "Target should be the market proxy address");
        assertEq(amount, 0, "Value should be zero for supply call");
        bytes memory expectedData =
            abi.encodeWithSelector(CometMainInterface.supply.selector, address(baseToken), exactAmount);
        assertEq(data, expectedData, "Data should be the encoded supply call");
    }

    function test_GivenZeroAmount_WhenSupplyIsCalled_ThenSupplyAmountIsEntireBalance() public {
        // given
        uint256 balance = 500 * 10 ** 18;
        vm.prank(owner);
        baseToken.mint(address(inputAccount), balance);

        // when
        vm.prank(processor);
        compoundV3PositionManager.supply(0);

        // then
        vm.expectRevert();
        MockBaseAccount(inputAccount).executeParams(2);
        (address target, uint256 amount, bytes memory data) = MockBaseAccount(inputAccount).executeParams(1);
        assertEq(target, marketProxyAddress, "Target should be the market proxy address");
        assertEq(amount, 0, "Value should be zero for supply call");
        bytes memory expectedData =
            abi.encodeWithSelector(CometMainInterface.supply.selector, address(baseToken), balance);
        assertEq(data, expectedData, "Data should be the encoded supply call");
    }

    function test_GivenValidAmount_WhenSupplyIsCalled_ThenApproveAmountOnMarketProxy() public {
        // given
        uint256 balance = 500 * 10 ** 18;
        vm.prank(owner);
        baseToken.mint(address(inputAccount), balance);

        // when
        vm.prank(processor);
        compoundV3PositionManager.supply(0);

        // then
        vm.expectRevert();
        MockBaseAccount(inputAccount).executeParams(2);
        (address target, uint256 value, bytes memory data) = MockBaseAccount(inputAccount).executeParams(0);
        assertEq(target, address(baseToken), "Target should be the token address");
        assertEq(value, 0, "Value should be zero for approve call");
        bytes memory expectedData = abi.encodeWithSelector(IERC20.approve.selector, marketProxyAddress, balance);
        assertEq(data, expectedData, "Data should be the encoded approve call");
    }

    function test_RevertSupply_WhenCallerIsNotProcessor() public {
        // given
        address unauthorized = makeAddr("unauthorized");
        uint256 amount = 1000 * 10 ** 18;

        // expect
        vm.expectRevert("Only the processor can call this function");

        // when
        vm.prank(unauthorized);
        compoundV3PositionManager.supply(amount);
    }

    // ============== Withdraw Tests ==============

    function test_GivenValidAmount_WhenWithdrawIsCalled_ThenWithdrawAmountIsEqual() public {
        // given
        uint256 exactAmount = 250 ether;

        // when
        vm.prank(processor);
        compoundV3PositionManager.withdraw(exactAmount);

        // then
        vm.expectRevert();
        MockBaseAccount(inputAccount).executeParams(1);
        (address target, uint256 value, bytes memory data) = MockBaseAccount(inputAccount).executeParams(0);
        assertEq(target, address(marketProxyAddress), "Target should be the token address");
        assertEq(value, 0, "Value should be zero for withdraw to call");
        bytes memory expectedData = abi.encodeWithSelector(
            CometMainInterface.withdrawTo.selector, address(outputAccount), address(baseToken), exactAmount
        );
        assertEq(data, expectedData, "Data should be the encoded withdraw to call");
    }

    function test_GivenZeroAmount_WhenWithdrawIsCalled_ThenWithdrawAmountIsUintMax() public {
        // given
        uint256 exactAmount = 0;

        // when
        vm.prank(processor);
        compoundV3PositionManager.withdraw(exactAmount);

        // then
        vm.expectRevert();
        MockBaseAccount(inputAccount).executeParams(1);
        (address target, uint256 value, bytes memory data) = MockBaseAccount(inputAccount).executeParams(0);
        assertEq(target, address(marketProxyAddress), "Target should be the token address");
        assertEq(value, 0, "Value should be zero for withdraw to call");
        bytes memory expectedData = abi.encodeWithSelector(
            CometMainInterface.withdrawTo.selector, address(outputAccount), address(baseToken), UINT256_MAX
        );
        assertEq(data, expectedData, "Data should be the encoded withdraw to call");
    }

    function test_RevertWithdraw_WhenCallerIsNotProcessor() public {
        // given
        address unauthorized = makeAddr("unauthorized");
        uint256 amount = 1000 * 10 ** 18;

        // expect
        vm.expectRevert("Only the processor can call this function");

        // when
        vm.prank(unauthorized);
        compoundV3PositionManager.withdraw(amount);
    }

    // ============== Withdraw Collateral Tests ==============

    function test_GivenValidAmount_WhenWithdrawCollateralIsCalled_ThenWithdrawAmountIsEqual() public {
        // given
        uint256 exactAmount = 250 ether;
        address token = vm.randomAddress();

        // when
        vm.prank(processor);
        compoundV3PositionManager.withdrawCollateral(token, exactAmount);

        // then
        vm.expectRevert();
        MockBaseAccount(inputAccount).executeParams(1);
        (address target, uint256 value, bytes memory data) = MockBaseAccount(inputAccount).executeParams(0);
        assertEq(target, address(marketProxyAddress), "Target should be the market proxy address");
        assertEq(value, 0, "Value should be zero for withdraw to call");
        bytes memory expectedData =
            abi.encodeWithSelector(CometMainInterface.withdrawTo.selector, address(outputAccount), token, exactAmount);
        assertEq(data, expectedData, "Data should be the encoded withdraw to call");
    }

    function test_GivenZeroAmount_WhenWithdrawCollateralIsCalled_ThenWithdrawAmountIsUintMax() public {
        // given
        uint256 exactAmount = 0;
        address token = vm.randomAddress();

        // when
        vm.prank(processor);
        compoundV3PositionManager.withdrawCollateral(token, exactAmount);

        // then
        vm.expectRevert();
        MockBaseAccount(inputAccount).executeParams(1);
        (address target, uint256 value, bytes memory data) = MockBaseAccount(inputAccount).executeParams(0);
        assertEq(target, address(marketProxyAddress), "Target should be the market proxy address");
        assertEq(value, 0, "Value should be zero for withdraw to call");
        bytes memory expectedData =
            abi.encodeWithSelector(CometMainInterface.withdrawTo.selector, address(outputAccount), token, UINT256_MAX);
        assertEq(data, expectedData, "Data should be the encoded withdraw to call");
    }

    function test_RevertWithdrawCollateral_WhenCallerIsNotProcessor() public {
        // given
        address unauthorized = makeAddr("unauthorized");
        uint256 amount = 1000 * 10 ** 18;
        address token = vm.randomAddress();

        // expect
        vm.expectRevert("Only the processor can call this function");

        // when
        vm.prank(unauthorized);
        compoundV3PositionManager.withdrawCollateral(token, amount);
    }

    // ============== Supply Collateral Tests ==============

    function test_GivenValidAmount_WhenSupplyCollateralIsCalled_ThenSupplyAmountIsEqual() public {
        // given
        uint256 exactAmount = 1000 * 10 ** 18;
        vm.prank(owner);
        MockERC20 newToken = new MockERC20("New Token", "NT", 18);
        newToken.mint(address(inputAccount), exactAmount * 2);

        // when
        vm.prank(processor);
        compoundV3PositionManager.supplyCollateral(address(newToken), exactAmount);

        // then
        vm.expectRevert();
        MockBaseAccount(inputAccount).executeParams(2);
        (address target, uint256 amount, bytes memory data) = MockBaseAccount(inputAccount).executeParams(1);
        assertEq(target, marketProxyAddress, "Target should be the market proxy address");
        assertEq(amount, 0, "Value should be zero for supply call");
        bytes memory expectedData =
            abi.encodeWithSelector(CometMainInterface.supply.selector, address(newToken), exactAmount);
        assertEq(data, expectedData, "Data should be the encoded supply call");
    }

    function test_GivenZeroAmount_WhenSupplyCollateralIsCalled_ThenSupplyAmountIsEntireBalance() public {
        // given
        uint256 balance = 500 * 10 ** 18;
        vm.prank(owner);
        MockERC20 newToken = new MockERC20("New Token", "NT", 18);
        newToken.mint(address(inputAccount), balance);

        // when
        vm.prank(processor);
        compoundV3PositionManager.supplyCollateral(address(newToken), 0);

        // then
        vm.expectRevert();
        MockBaseAccount(inputAccount).executeParams(2);
        (address target, uint256 amount, bytes memory data) = MockBaseAccount(inputAccount).executeParams(1);
        assertEq(target, marketProxyAddress, "Target should be the market proxy address");
        assertEq(amount, 0, "Value should be zero for supply call");
        bytes memory expectedData =
            abi.encodeWithSelector(CometMainInterface.supply.selector, address(newToken), balance);
        assertEq(data, expectedData, "Data should be the encoded supply call");
    }

    function test_GivenValidAmount_WhenSupplyCollateralIsCalled_ThenApproveAmountOnMarketProxy() public {
        // given
        uint256 balance = 500 * 10 ** 18;
        MockERC20 newToken = new MockERC20("New Token", "NT", 18);
        newToken.mint(address(inputAccount), balance);

        // when
        vm.prank(processor);
        compoundV3PositionManager.supplyCollateral(address(newToken), balance);

        // then
        vm.expectRevert();
        MockBaseAccount(inputAccount).executeParams(2);
        (address target, uint256 value, bytes memory data) = MockBaseAccount(inputAccount).executeParams(0);
        assertEq(target, address(newToken), "Target should be the token address");
        assertEq(value, 0, "Value should be zero for approve call");
        bytes memory expectedData = abi.encodeWithSelector(IERC20.approve.selector, marketProxyAddress, balance);
        assertEq(data, expectedData, "Data should be the encoded approve call");
    }

    function test_RevertSupplyCollateral_WhenCallerIsNotProcessor() public {
        // given
        address unauthorized = makeAddr("unauthorized");
        uint256 amount = 1000 * 10 ** 18;

        // expect
        vm.expectRevert("Only the processor can call this function");

        // when
        vm.prank(unauthorized);
        compoundV3PositionManager.supplyCollateral(address(baseToken), amount);
    }
}
