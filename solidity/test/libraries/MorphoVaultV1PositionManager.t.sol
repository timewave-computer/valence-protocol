// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test, console} from "forge-std/src/Test.sol";
import {MorphoVaultV1PositionManager} from "../../src/libraries/MorphoVaultV1PositionManager.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {IERC4626} from "forge-std/src/interfaces/IERC4626.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {MockERC20} from "../mocks/MockERC20.sol";
import {MockBaseAccount} from "../mocks/MockBaseAccount.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

contract MorphoVaultV1PositionManagerTest is Test {
    // Contract under test
    MorphoVaultV1PositionManager public morphoVaultV1PositionManager;

    // Mock contracts
    MockBaseAccount public inputAccount;
    MockBaseAccount public outputAccount;
    MockERC20 public assetToken;
    address public vaultAddress;

    // Test addresses
    address public owner;
    address public processor;

    // Setup function to initialize test environment
    function setUp() public {
        // Setup test addresses
        owner = makeAddr("owner");
        processor = makeAddr("processor");
        vaultAddress = makeAddr("morphoVaultV1");

        // Deploy mock tokens
        assetToken = new MockERC20("Asset Token", "AT", 18);

        // Create mock accounts
        vm.startPrank(owner);
        inputAccount = new MockBaseAccount();
        outputAccount = new MockBaseAccount();
        vm.stopPrank();

        // Deploy MorphoVaultV1PositionManager contract
        vm.startPrank(owner);

        // Create and encode config directly
        MorphoVaultV1PositionManager.MorphoVaultV1PositionManagerConfig memory config = MorphoVaultV1PositionManager
            .MorphoVaultV1PositionManagerConfig({
            inputAccount: BaseAccount(payable(address(inputAccount))),
            outputAccount: BaseAccount(payable(address(outputAccount))),
            vaultAddress: vaultAddress,
            assetAddress: address(assetToken)
        });

        vm.mockCall(vaultAddress, abi.encodeWithSignature("asset()"), abi.encode(address(assetToken)));
        morphoVaultV1PositionManager = new MorphoVaultV1PositionManager(owner, processor, abi.encode(config));
        vm.stopPrank();

        vm.label(address(inputAccount), "inputAccount");
        vm.label(address(outputAccount), "outputAccount");
        vm.label(address(assetToken), "assetToken");
        vm.label(vaultAddress, "vaultAddress");
    }

    // ============== Configuration Tests ==============

    function test_GivenValidConfig_WhenContractIsDeployed_ThenConfigIsSet() public view {
        (
            BaseAccount actualInputAccount,
            BaseAccount actualOutputAccount,
            address actualVaultAddress,
            address actualAssetAddress
        ) = morphoVaultV1PositionManager.config();

        assertEq(address(actualInputAccount), address(inputAccount));
        assertEq(address(actualOutputAccount), address(outputAccount));
        assertEq(actualAssetAddress, address(assetToken));
        assertEq(actualVaultAddress, vaultAddress);
    }

    function test_GivenValidConfig_WhenUpdateConfigIsCalled_ThenConfigIsUpdated() public {
        // given
        MockERC20 newBaseToken = new MockERC20("New Base Token", "NBT", 18);
        MorphoVaultV1PositionManager.MorphoVaultV1PositionManagerConfig memory newConfig = MorphoVaultV1PositionManager
            .MorphoVaultV1PositionManagerConfig({
            inputAccount: new BaseAccount(owner, new address[](0)),
            outputAccount: new BaseAccount(owner, new address[](0)),
            assetAddress: address(newBaseToken),
            vaultAddress: makeAddr("morphoVaultV1New")
        });
        vm.mockCall(newConfig.vaultAddress, abi.encodeWithSignature("asset()"), abi.encode(address(newBaseToken)));

        // when
        vm.prank(owner);
        morphoVaultV1PositionManager.updateConfig(abi.encode(newConfig));

        // then
        (
            BaseAccount actualInputAccount,
            BaseAccount actualOutputAccount,
            address actualVaultAddress,
            address actualAssetAddress
        ) = morphoVaultV1PositionManager.config();

        assertEq(address(actualInputAccount), address(newConfig.inputAccount));
        assertEq(address(actualOutputAccount), address(newConfig.outputAccount));
        assertEq(actualAssetAddress, address(newConfig.assetAddress));
        assertEq(actualVaultAddress, newConfig.vaultAddress);
    }

    function test_RevertUpdateConfig_WithInvalidConfig_WhenInputAccountIsZeroAddress() public {
        // given
        MorphoVaultV1PositionManager.MorphoVaultV1PositionManagerConfig memory newConfig = MorphoVaultV1PositionManager
            .MorphoVaultV1PositionManagerConfig({
            inputAccount: BaseAccount(payable(address(0))),
            outputAccount: new BaseAccount(owner, new address[](0)),
            assetAddress: address(assetToken),
            vaultAddress: vaultAddress
        });

        // expect
        vm.expectRevert("Input account can't be zero address");

        // when
        vm.prank(owner);
        morphoVaultV1PositionManager.updateConfig(abi.encode(newConfig));
    }

    function test_RevertUpdateConfig_WithInvalidConfig_WhenOutputAccountIsZeroAddress() public {
        // given
        MorphoVaultV1PositionManager.MorphoVaultV1PositionManagerConfig memory newConfig = MorphoVaultV1PositionManager
            .MorphoVaultV1PositionManagerConfig({
            inputAccount: new BaseAccount(owner, new address[](0)),
            outputAccount: BaseAccount(payable(address(0))),
            assetAddress: address(assetToken),
            vaultAddress: vaultAddress
        });

        // expect
        vm.expectRevert("Output account can't be zero address");

        // when
        vm.prank(owner);
        morphoVaultV1PositionManager.updateConfig(abi.encode(newConfig));
    }

    function test_RevertUpdateConfig_WithInvalidConfig_WhenMarketBaseAssetAndGivenBaseAssetAreNotSame() public {
        // given
        vm.mockCall(vaultAddress, abi.encodeWithSignature("asset()"), abi.encode(vm.randomAddress()));
        MorphoVaultV1PositionManager.MorphoVaultV1PositionManagerConfig memory newConfig = MorphoVaultV1PositionManager
            .MorphoVaultV1PositionManagerConfig({
            inputAccount: new BaseAccount(owner, new address[](0)),
            outputAccount: new BaseAccount(owner, new address[](0)),
            assetAddress: address(assetToken),
            vaultAddress: vaultAddress
        });

        // expect
        vm.expectRevert("Vault asset and given asset are not same");

        // when
        vm.prank(owner);
        morphoVaultV1PositionManager.updateConfig(abi.encode(newConfig));
    }

    function test_RevertUpdateConfig_WithInvalidConfig_WhenMarketProxyAddressIsZeroAddress() public {
        // given
        MorphoVaultV1PositionManager.MorphoVaultV1PositionManagerConfig memory newConfig = MorphoVaultV1PositionManager
            .MorphoVaultV1PositionManagerConfig({
            inputAccount: new BaseAccount(owner, new address[](0)),
            outputAccount: new BaseAccount(owner, new address[](0)),
            assetAddress: address(assetToken),
            vaultAddress: address(0)
        });

        // expect
        vm.expectRevert("Vault address can't be zero address");

        // when
        vm.prank(owner);
        morphoVaultV1PositionManager.updateConfig(abi.encode(newConfig));
    }

    function test_RevertUpdateConfig_WithUnauthorized_WhenCallerIsNotOwner() public {
        // given
        address unauthorized = makeAddr("unauthorized");
        MockERC20 newBaseToken = new MockERC20("New Base Token", "NBT", 18);
        MorphoVaultV1PositionManager.MorphoVaultV1PositionManagerConfig memory newConfig = MorphoVaultV1PositionManager
            .MorphoVaultV1PositionManagerConfig({
            inputAccount: new BaseAccount(owner, new address[](0)),
            outputAccount: new BaseAccount(owner, new address[](0)),
            assetAddress: address(newBaseToken),
            vaultAddress: vaultAddress
        });

        // expect
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, unauthorized));

        // when
        vm.prank(unauthorized);
        morphoVaultV1PositionManager.updateConfig(abi.encode(newConfig));
    }

    // ============== Get Balance Tests ==============

    function test_WhenBalanceIsCalled_ThenValueIsEqualToIPreviewRedeemOfVault() public {
        // given
        uint256 mTokenBalance = 1000 * 10 ** 18;
        uint256 previewRedeem = 100 * 10 ** 18;
        vm.mockCall(
            vaultAddress,
            abi.encodeWithSignature("balanceOf(address)", address(inputAccount)),
            abi.encode(mTokenBalance)
        );
        vm.mockCall(
            vaultAddress, abi.encodeWithSignature("previewRedeem(uint256)", mTokenBalance), abi.encode(previewRedeem)
        );

        // when
        uint256 balance = morphoVaultV1PositionManager.balance();

        // then
        assertEq(balance, previewRedeem);
    }

    // ============== Withdraw Tests ==============

    function test_GivenValidAmount_WhenWithdrawIsCalled_ThenWithdrawAmountIsEqual() public {
        // given
        uint256 exactAmount = 250 ether;

        // when
        vm.prank(processor);
        morphoVaultV1PositionManager.withdraw(exactAmount);

        // then
        vm.expectRevert();
        MockBaseAccount(inputAccount).executeParams(1);
        (address target, uint256 value, bytes memory data) = MockBaseAccount(inputAccount).executeParams(0);
        assertEq(target, vaultAddress, "Target should be the vault address");
        assertEq(value, 0, "Value should be zero for withdraw call");
        bytes memory expectedData = abi.encodeWithSelector(
            IERC4626.withdraw.selector, exactAmount, address(outputAccount), address(inputAccount)
        );
        assertEq(data, expectedData, "Data should be the encoded withdraw call");
    }

    function test_GivenZeroAmount_WhenWithdrawIsCalled_ThenWithdrawAmountIsMaxWithdrawFromVault() public {
        // given
        uint256 exactAmount = 0;
        uint256 maxWithdraw = 1000 * 10 ** 18;
        vm.mockCall(
            vaultAddress,
            abi.encodeWithSignature("maxWithdraw(address)", address(inputAccount)),
            abi.encode(maxWithdraw)
        );

        // when
        vm.prank(processor);
        morphoVaultV1PositionManager.withdraw(exactAmount);

        // then
        vm.expectRevert();
        MockBaseAccount(inputAccount).executeParams(1);
        (address target, uint256 value, bytes memory data) = MockBaseAccount(inputAccount).executeParams(0);
        assertEq(target, vaultAddress, "Target should be the vault address");
        assertEq(value, 0, "Value should be zero for withdraw call");
        bytes memory expectedData = abi.encodeWithSelector(
            IERC4626.withdraw.selector, maxWithdraw, address(outputAccount), address(inputAccount)
        );
        assertEq(data, expectedData, "Data should be the encoded withdraw call");
    }

    function test_RevertWithdraw_WhenCallerIsNotProcessor() public {
        // given
        address unauthorized = makeAddr("unauthorized");
        uint256 amount = 1000 * 10 ** 18;

        // expect
        vm.expectRevert("Only the processor can call this function");

        // when
        vm.prank(unauthorized);
        morphoVaultV1PositionManager.withdraw(amount);
    }

    // ============== Deposit Tests ==============

    function test_GivenValidAmount_WhenDepositIsCalled_ThenApproveAmountOnVault() public {
        // given
        uint256 balance = 500 * 10 ** 18;
        vm.prank(owner);
        assetToken.mint(address(inputAccount), balance);

        // when
        vm.prank(processor);
        morphoVaultV1PositionManager.deposit(balance);

        // then
        vm.expectRevert();
        MockBaseAccount(inputAccount).executeParams(2);
        (address target, uint256 value, bytes memory data) = MockBaseAccount(inputAccount).executeParams(0);
        assertEq(target, address(assetToken), "Target should be the token address");
        assertEq(value, 0, "Value should be zero for approve call");
        bytes memory expectedData = abi.encodeWithSelector(IERC20.approve.selector, vaultAddress, balance);
        assertEq(data, expectedData, "Data should be the encoded approve call");
    }

    function test_GivenValidAmount_WhenDepositIsCalled_ThenDepositAmountIsEqual() public {
        // given
        uint256 exactAmount = 1000 * 10 ** 18;
        vm.prank(owner);
        assetToken.mint(address(inputAccount), exactAmount * 2);

        // when
        vm.prank(processor);
        morphoVaultV1PositionManager.deposit(exactAmount);

        // then
        vm.expectRevert();
        MockBaseAccount(inputAccount).executeParams(2);
        (address target, uint256 amount, bytes memory data) = MockBaseAccount(inputAccount).executeParams(1);
        assertEq(target, vaultAddress, "Target should be the vault address");
        assertEq(amount, 0, "Value should be zero for deposit call");
        bytes memory expectedData =
            abi.encodeWithSelector(IERC4626.deposit.selector, exactAmount, address(inputAccount));
        assertEq(data, expectedData, "Data should be the encoded deposit call");
    }

    function test_GivenZeroAmount_WhenDepositIsCalled_ThenDepositAmountIsEntireBalance() public {
        // given
        uint256 balance = 500 * 10 ** 18;
        vm.prank(owner);
        assetToken.mint(address(inputAccount), balance);

        // when
        vm.prank(processor);
        morphoVaultV1PositionManager.deposit(0);

        // then
        vm.expectRevert();
        MockBaseAccount(inputAccount).executeParams(2);
        (address target, uint256 amount, bytes memory data) = MockBaseAccount(inputAccount).executeParams(1);
        assertEq(target, vaultAddress, "Target should be the vault address");
        assertEq(amount, 0, "Value should be zero for deposit call");
        bytes memory expectedData = abi.encodeWithSelector(IERC4626.deposit.selector, balance, address(inputAccount));
        assertEq(data, expectedData, "Data should be the encoded deposit call");
    }

    function test_RevertDeposit_WhenCallerIsNotProcessor() public {
        // given
        address unauthorized = makeAddr("unauthorized");
        uint256 amount = 1000 * 10 ** 18;

        // expect
        vm.expectRevert("Only the processor can call this function");

        // when
        vm.prank(unauthorized);
        morphoVaultV1PositionManager.deposit(amount);
    }
}
