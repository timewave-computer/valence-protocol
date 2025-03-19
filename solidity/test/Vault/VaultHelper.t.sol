// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Test} from "forge-std/src/Test.sol";
import {ERC4626Upgradeable, ValenceVault} from "../../src/libraries/ValenceVault.sol";
import {BaseAccount} from "../../src/accounts/BaseAccount.sol";
import {MockERC20} from "../mocks/MockERC20.sol";
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";
import {ERC1967Proxy} from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

abstract contract VaultHelper is Test {
    using Math for uint256;

    // Constants
    uint32 internal constant BASIS_POINTS = 10000;
    uint256 internal constant INITIAL_USER_BALANCE = 100_000_000_000;
    uint32 internal constant MAX_WITHDRAW_FEE = 2000;
    uint256 internal constant INITIAL_TIMESTAMP = 5000;
    uint256 internal constant INITIAL_BLOCK = 100;
    uint64 internal constant ONE_DAY = 1 days;
    uint256 internal constant ONE_SHARE = 1e18;

    // Contracts
    ValenceVault internal vault;
    BaseAccount internal depositAccount;
    BaseAccount internal withdrawAccount;
    MockERC20 internal token;

    // Addresses
    address internal owner;
    address internal strategist;
    address internal user;

    // Address for fee distribution
    address internal strategistFeeAccount;
    address internal platformFeeAccount;

    // Events
    event Deposit(address indexed sender, address indexed owner, uint256 assets, uint256 shares);

    function setUp() public virtual {
        // Setup addresses
        owner = makeAddr("owner");
        strategist = makeAddr("strategist");
        user = makeAddr("user");
        strategistFeeAccount = makeAddr("strategistFeeAccount");
        platformFeeAccount = makeAddr("platformFeeAccount");

        // Setup initial block and time
        vm.warp(INITIAL_TIMESTAMP);
        vm.roll(INITIAL_BLOCK);

        // Deploy contracts
        vm.startPrank(owner);
        token = new MockERC20("Test Token", "TEST");
        depositAccount = new BaseAccount(owner, new address[](0));
        withdrawAccount = new BaseAccount(owner, new address[](0));

        // Setup vault configuration
        ValenceVault.VaultConfig memory config = ValenceVault.VaultConfig({
            depositAccount: depositAccount,
            withdrawAccount: withdrawAccount,
            strategist: strategist,
            depositCap: 0,
            maxWithdrawFeeBps: MAX_WITHDRAW_FEE,
            withdrawLockupPeriod: ONE_DAY * 3,
            fees: defaultFees(),
            feeDistribution: defaultDistributionFees()
        });

        ValenceVault implementation = new ValenceVault();
        bytes memory initializeData = abi.encodeWithSelector(
            ValenceVault.initialize.selector,
            owner,
            abi.encode(config),
            address(token),
            "Valence Vault Token",
            "VVT",
            ONE_SHARE
        );
        // Deploy the proxy
        ERC1967Proxy proxy = new ERC1967Proxy(address(implementation), initializeData);
        vault = ValenceVault(address(proxy));

        // Setup permissions
        depositAccount.approveLibrary(address(vault));
        withdrawAccount.approveLibrary(address(vault));
        vm.stopPrank();

        // Setup user state
        vm.startPrank(owner);
        token.mint(user, INITIAL_USER_BALANCE);
        vm.stopPrank();

        vm.startPrank(user);
        token.approve(address(vault), type(uint256).max);
        vm.stopPrank();

        vm.roll(vm.getBlockNumber() + 1);
        vm.warp(vm.getBlockTimestamp() + 12);
    }

    function defaultFees() public pure returns (ValenceVault.FeeConfig memory) {
        return
            ValenceVault.FeeConfig({depositFeeBps: 0, platformFeeBps: 0, performanceFeeBps: 0, solverCompletionFee: 0});
    }

    function defaultDistributionFees() public view returns (ValenceVault.FeeDistributionConfig memory) {
        return ValenceVault.FeeDistributionConfig({
            strategistAccount: strategistFeeAccount,
            platformAccount: platformFeeAccount,
            strategistRatioBps: 3000
        });
    }

    function setFeeDistribution(address strategistAccount, address platformAccount, uint32 strategistRatioBps)
        internal
    {
        vm.startPrank(owner);

        ValenceVault.FeeDistributionConfig memory feeDistConfig = ValenceVault.FeeDistributionConfig({
            strategistAccount: strategistAccount,
            platformAccount: platformAccount,
            strategistRatioBps: strategistRatioBps
        });

        ValenceVault.VaultConfig memory config = _getConfig();

        config.feeDistribution = feeDistConfig;

        vault.updateConfig(abi.encode(config));
        vm.stopPrank();
    }

    function setFees(uint32 depositFee, uint32 platformFee, uint32 performanceFee, uint64 solverCompletionFee)
        internal
        returns (ValenceVault.FeeConfig memory)
    {
        vm.startPrank(owner);
        ValenceVault.FeeConfig memory feeConfig = ValenceVault.FeeConfig({
            depositFeeBps: depositFee,
            platformFeeBps: platformFee,
            performanceFeeBps: performanceFee,
            solverCompletionFee: solverCompletionFee
        });

        ValenceVault.VaultConfig memory config = _getConfig();

        config.fees = feeConfig;

        vault.updateConfig(abi.encode(config));
        vm.stopPrank();

        return feeConfig;
    }

    function setDepositCap(uint128 newCap) internal {
        vm.startPrank(owner);
        ValenceVault.VaultConfig memory config = _getConfig();

        config.depositCap = newCap;

        vault.updateConfig(abi.encode(config));
        vm.stopPrank();
    }

    // Helper function to get current config
    function _getConfig() internal view returns (ValenceVault.VaultConfig memory) {
        (
            BaseAccount _depositAccount,
            BaseAccount _withdrawAccount,
            address _strategist,
            ValenceVault.FeeConfig memory _fees,
            ValenceVault.FeeDistributionConfig memory _feeDistribution,
            uint128 _depositCap,
            uint64 _withdrawLockupPeriod,
            uint32 _maxWithdrawFeeBps
        ) = vault.config();

        return ValenceVault.VaultConfig({
            depositAccount: _depositAccount,
            withdrawAccount: _withdrawAccount,
            strategist: _strategist,
            depositCap: _depositCap,
            maxWithdrawFeeBps: _maxWithdrawFeeBps,
            withdrawLockupPeriod: _withdrawLockupPeriod,
            fees: _fees,
            feeDistribution: _feeDistribution
        });
    }

    function _getPackedValues() internal view returns (ValenceVault.PackedValues memory) {
        (uint32 currentUpdateId, uint64 nextWithdrawRequestId, bool pauser, bool paused) = vault.packedValues();

        return ValenceVault.PackedValues({
            currentUpdateId: currentUpdateId,
            nextWithdrawRequestId: nextWithdrawRequestId,
            pauser: pauser,
            paused: paused
        });
    }

    function _update(uint256 newRate, uint32 newWithdrawFee, uint256 nettingAmount) public {
        // Add block and time
        vm.roll(vm.getBlockNumber() + 1);
        vm.warp(vm.getBlockTimestamp() + 12);

        vault.update(newRate, newWithdrawFee, nettingAmount);
    }
}
