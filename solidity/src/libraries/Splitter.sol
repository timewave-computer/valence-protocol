// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {Library} from "./Library.sol";
import {Account} from "../accounts/Account.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";
import {IDynamicRatioOracle} from "./interfaces/splitter/IDynamicRatioOracle.sol";
import {BaseAccount} from "../accounts/BaseAccount.sol";

/**
 * @title Splitter
 * @dev The Valence Splitter library allows to split funds from one input account to one or more output account(s),
 * for one or more token denom(s) according to the configured ratio(s).
 * It is typically used as part of a Valence Program. In that context,
 * a Processor contract will be the main contract interacting with the Splitter library.
 */
contract Splitter is Library {
    uint256 public constant DECIMALS = 18;

    /**
     * @title SplitterConfig
     * @notice Configuration struct for splitting operations
     * @dev Defines splitting parameters
     * @param inputAccount Address of the input account
     * @param splits Split configuration per token address
     */
    struct SplitterConfig {
        BaseAccount inputAccount;
        SplitConfig[] splits;
    }

    /**
     * @title SplitConfig
     * @notice Split config for specified account
     * @dev Used to define the split config for a token to an account
     * @param outputAccount Address of the output account
     * @param token Address of the token account. Use address(0) to send ETH
     * @param splitType type of the split
     * @param amount encoded configuration based on the type of split
     */
    struct SplitConfig {
        Account outputAccount;
        address token;
        SplitType splitType;
        bytes splitData;
    }

    /**
     * @title SplitType
     * @notice enum defining allowed variants of split config
     */
    enum SplitType {
        FixedAmount,
        FixedRatio,
        DynamicRatio
    }

    /**
     * @title DynamicRatioAmount
     * @notice Params for dynamic ratio split
     * @dev Used to define the config when split type is DynamicRatio
     * @param contractAddress Address of the dynamic ratio oracle contract
     * @param params Encoded parameters for the oracle
     */
    struct DynamicRatioAmount {
        address contractAddress;
        bytes params;
    }

    /// @notice Holds the current configuration for the Splitter.
    SplitterConfig public config;

    /// @notice Holds the splitConfig against output account against split token.
    mapping(address => mapping(Account => SplitConfig)) splitConfigMapping;
    mapping(address => uint256) tokenRatioSplitSum;
    mapping(address => uint256) tokenAmountSplitSum;

    /**
     * @title TransferData
     * @notice data for dynamic ratio split
     * @dev Used to save transfer data during split execution
     * @param token address of token to be transferred, address(0) when native
     * @param outputAccount the account where token needs to be transferred
     * @param amount absolute amount of tokens to be transferred
     */
    struct TransferData {
        address token;
        Account outputAccount;
        uint256 amount;
    }

    /**
     * @dev Constructor initializes the contract with the owner, processor, and initial configuration.
     * @param _owner Address of the contract owner.
     * @param _processor Address of the processor that can execute functions.
     * @param _config Encoded configuration parameters for the Splitter.
     */
    constructor(address _owner, address _processor, bytes memory _config) Library(_owner, _processor, _config) {}

    /**
     * @notice Validates the provided configuration parameters
     * @dev Checks for validity of input account, and splits
     * @param _config The encoded configuration bytes to validate
     * @return SplitterConfig A validated configuration struct
     */
    function validateConfig(bytes memory _config) internal returns (SplitterConfig memory) {
        // Decode the configuration bytes into the SplitterConfig struct.
        SplitterConfig memory decodedConfig = abi.decode(_config, (SplitterConfig));

        // Ensure the input account address is valid (non-zero).
        if (decodedConfig.inputAccount == Account(payable(address(0)))) {
            revert("Input account can't be zero address");
        }

        deleteSplitsInState();
        validateSplits(decodedConfig.splits);

        return decodedConfig;
    }

    /**
     * @notice Validates the provided splits configuration
     * @dev Checks for duplicate split, sum of ratios to 1 and dynamic ratio contract address to be valid smart contract
     * @param splits The array of SplitConfig to validate
     */
    function validateSplits(SplitConfig[] memory splits) internal {
        require(splits.length > 0, "No split configuration provided.");

        for (uint256 i = 0; i < splits.length; i++) {
            SplitConfig memory splitConfig = splits[i];

            if (address(splitConfigMapping[splitConfig.token][splitConfig.outputAccount].outputAccount) != address(0)) {
                revert("Duplicate split in split config.");
            }

            if (splitConfig.splitType == SplitType.FixedAmount) {
                uint256 decodedAmount = abi.decode(splitConfig.splitData, (uint256));
                require(decodedAmount > 0, "Invalid split config: amount cannot be zero.");

                tokenAmountSplitSum[splitConfig.token] += decodedAmount;
            } else if (splitConfig.splitType == SplitType.FixedRatio) {
                uint256 decodedRatio = abi.decode(splitConfig.splitData, (uint256));
                require(decodedRatio > 0, "Invalid split config: ratio cannot be zero.");

                tokenRatioSplitSum[splitConfig.token] += decodedRatio;
            } else {
                DynamicRatioAmount memory dynamicRatioAmount = abi.decode(splitConfig.splitData, (DynamicRatioAmount));
                require(
                    tokenAmountSplitSum[splitConfig.token] == 0 && tokenRatioSplitSum[splitConfig.token] == 0,
                    "Invalid split config: cannot combine different split types for same token."
                );
                require(
                    dynamicRatioAmount.contractAddress.code.length > 0,
                    "Invalid split config: dynamic ratio contract address is not a contract"
                );
            }

            splitConfigMapping[splitConfig.token][splitConfig.outputAccount] = splitConfig;
        }

        // checking if sum of all ratios is 1 and conflicting types are not provided
        for (uint256 i = 0; i < splits.length; i++) {
            SplitConfig memory splitConfig = splits[i];

            if (splitConfig.splitType == SplitType.FixedAmount) {
                require(
                    tokenRatioSplitSum[splitConfig.token] == 0,
                    "Invalid split config: cannot combine different split types for same token."
                );
            } else if (splitConfig.splitType == SplitType.FixedRatio) {
                unchecked {
                    uint256 sum = 10 ** DECIMALS - tokenRatioSplitSum[splitConfig.token];
                    require(sum <= 1, "Invalid split config: sum of ratios is not equal to 1.");
                }
                require(
                    tokenAmountSplitSum[splitConfig.token] == 0,
                    "Invalid split config: cannot combine different split types for same token."
                );
            } else {
                require(
                    tokenAmountSplitSum[splitConfig.token] == 0 && tokenRatioSplitSum[splitConfig.token] == 0,
                    "Invalid split config: cannot combine different split types for same token."
                );
            }
        }
    }

    /**
     * @notice Checks if any split for a given token uses dynamic ratio
     * @param splits The array of splits to check
     * @param token The token to check for
     * @return true if any split for the token uses dynamic ratio
     */
    function hasDynamicRatioForToken(SplitConfig[] memory splits, address token) internal pure returns (bool) {
        for (uint256 i = 0; i < splits.length; i++) {
            if (splits[i].token == token && splits[i].splitType == SplitType.DynamicRatio) {
                return true;
            }
        }
        return false;
    }

    /**
     * @notice deletes the existing splits in state
     * @dev Useful to be used before updating config
     */
    function deleteSplitsInState() internal {
        for (uint256 i = 0; i < config.splits.length; i++) {
            SplitConfig memory splitConfig = config.splits[i];

            delete tokenRatioSplitSum[splitConfig.token];
            delete tokenAmountSplitSum[splitConfig.token];
            delete splitConfigMapping[splitConfig.token][splitConfig.outputAccount];
        }
    }

    /**
     * @dev Internal initialization function called during construction
     * @param _config New configuration
     */
    function _initConfig(bytes memory _config) internal override {
        config = validateConfig(_config);
    }

    /**
     * @dev Updates the Splitter configuration.
     * Only the contract owner is authorized to call this function.
     * @param _config New encoded configuration parameters.
     */
    function updateConfig(bytes memory _config) public override onlyOwner {
        config = validateConfig(_config);
    }

    /**
     * @notice Executes the split operation based on the configured splits
     * @dev Splits funds from the input account to output accounts according to configured ratios
     * Only the processor can call this function
     */
    function split() external onlyProcessor {
        SplitterConfig memory currentConfig = config;
        TransferData[] memory transfers = new TransferData[](currentConfig.splits.length);

        for (uint256 i = 0; i < currentConfig.splits.length; i++) {
            SplitConfig memory splitConfig = currentConfig.splits[i];
            address token = splitConfig.token;
            uint256 balance;

            if (address(token) == address(0)) {
                balance = address(currentConfig.inputAccount).balance;
            } else {
                balance = IERC20(token).balanceOf(address(currentConfig.inputAccount));
            }
            
            // Process all splits for this token
            transfers[i] = prepareSplit(splitConfig, balance);
        }

        for (uint256 i = 0; i < transfers.length; i++) {
            TransferData memory transfer = transfers[i];
            if (transfer.amount == 0) {
                continue;
            }
            transferFunds(currentConfig.inputAccount, transfer.outputAccount, transfer.token, transfer.amount);
        }
    }

    function prepareSplit(SplitConfig memory splitConfig, uint256 totalBalance)
        internal
        view
        returns (TransferData memory)
    {
        uint256 amount = calculateSplitAmount(splitConfig, totalBalance);
        return TransferData({token: splitConfig.token, outputAccount: splitConfig.outputAccount, amount: amount});
    }

    /**
     * @notice Calculates the split amount based on the split configuration
     * @param splitConfig The split configuration
     * @param totalBalance The total balance available for splitting
     * @return The calculated split amount
     */
    function calculateSplitAmount(SplitConfig memory splitConfig, uint256 totalBalance)
        internal
        view
        returns (uint256)
    {
        if (splitConfig.splitType == SplitType.FixedAmount) {
            return abi.decode(splitConfig.splitData, (uint256));
        } else if (splitConfig.splitType == SplitType.FixedRatio) {
            uint256 ratio = abi.decode(splitConfig.splitData, (uint256));
            // Using multiply_ratio equivalent: (balance * numerator) / denominator
            return (totalBalance * ratio) / (10 ** DECIMALS);
        } else if (splitConfig.splitType == SplitType.DynamicRatio) {
            DynamicRatioAmount memory dynamicRatioAmount = abi.decode(splitConfig.splitData, (DynamicRatioAmount));
            // Get dynamic ratio from oracle contract
            uint256 ratio =
                queryDynamicRatio(IERC20(splitConfig.token), dynamicRatioAmount.contractAddress, dynamicRatioAmount.params);
            return (totalBalance * ratio) / (10 ** DECIMALS);
        } else {
            revert("Invalid split type");
        }
    }

    /**
     * @notice Queries dynamic ratio from external contract
     * @param contractAddr The external contract address
     * @param token The token address
     * @param params The calldata to be passed with query
     * @return The dynamic ratio from oracle
     */
    function queryDynamicRatio(IERC20 token, address contractAddr, bytes memory params)
        internal
        view
        returns (uint256)
    {
        uint256 ratio = IDynamicRatioOracle(contractAddr).queryDynamicRatio(token, params);
        require(ratio <= 10 ** DECIMALS, "Dynamic ratio exceeds maximum (1.0)");
        return ratio;
    }

    /**
     * @notice Transfers funds from input account to output account
     * @param from The input account
     * @param to The output account
     * @param token The token to transfer (address(0) for ETH)
     * @param amount The amount to transfer
     */
    function transferFunds(Account from, Account to, address token, uint256 amount) internal {
        if (token == address(0)) {
            bytes memory data = "";
            from.execute(address(to), amount, data);
        } else {
            bytes memory transferData = abi.encodeWithSelector(IERC20.transfer.selector, address(to), amount);
            from.execute(token, 0, transferData);
        }
    }
}
