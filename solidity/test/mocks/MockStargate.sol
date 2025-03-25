// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

import {IStargate, Ticket, StargateType} from "@stargatefinance/stg-evm-v2/src/interfaces/IStargate.sol";
import {
    MessagingFee,
    OFTReceipt,
    MessagingReceipt,
    SendParam,
    OFTLimit,
    OFTFeeDetail
} from "@layerzerolabs/lz-evm-oapp-v2/contracts/oft/interfaces/IOFT.sol";
import {IERC20} from "forge-std/src/interfaces/IERC20.sol";

// Mock IStargate implementation
contract MockStargate is IStargate {
    address private _token;
    uint256 private _nativeFee = 0.001 ether; // Default fee
    uint256 private _dustAmount = 0.0001 ether; // Small amount to simulate transfer costs
    uint256 private _receiptAmount; // The amount received after fees

    constructor(address token_) {
        _token = token_;
    }

    function send(SendParam calldata _sendParam, MessagingFee calldata _fee, address _refundAddress)
        external
        payable
        returns (MessagingReceipt memory, OFTReceipt memory)
    {
        (MessagingReceipt memory msgReceipt, OFTReceipt memory receipt,) = sendToken(_sendParam, _fee, _refundAddress);
        return (msgReceipt, receipt);
    }

    function sendToken(SendParam calldata sendParam, MessagingFee calldata fee, address refundAddress)
        public
        payable
        override
        returns (MessagingReceipt memory msgReceipt, OFTReceipt memory oftReceipt, Ticket memory ticket)
    {
        // Check that correct amount of native token was sent
        if (_token == address(0)) {
            // Native token case
            require(msg.value >= sendParam.amountLD + fee.nativeFee, "Insufficient native amount");
        } else {
            // ERC20 case
            require(msg.value >= fee.nativeFee, "Insufficient native fee");
            // Simulate transferring tokens from sender
            try IERC20(_token).transferFrom(msg.sender, address(this), sendParam.amountLD) {
                // Success
            } catch {
                revert("Token transfer failed");
            }
        }

        // Return dummy values
        msgReceipt = MessagingReceipt({guid: bytes32(0), nonce: 0, fee: fee});

        oftReceipt = OFTReceipt({amountSentLD: sendParam.amountLD, amountReceivedLD: sendParam.minAmountLD});

        ticket = Ticket({ticketId: 0, passengerBytes: ""});

        unchecked {
            refundAddress;
        }

        return (msgReceipt, oftReceipt, ticket);
    }

    function quoteOFT(SendParam calldata sendParam)
        external
        view
        override
        returns (OFTLimit memory, OFTFeeDetail[] memory oftFeeDetails, OFTReceipt memory)
    {
        uint256 amountReceived;

        if (_receiptAmount != 0) {
            // Use preset receipt amount
            amountReceived = _receiptAmount;
        } else {
            // Calculate a realistic amount after fees (e.g., 0.1% fee)
            amountReceived = sendParam.amountLD * 999 / 1000;
        }

        OFTReceipt memory receipt = OFTReceipt({amountSentLD: sendParam.amountLD, amountReceivedLD: amountReceived});

        unchecked {
            oftFeeDetails;
        }

        OFTLimit memory limit = OFTLimit({minAmountLD: amountReceived, maxAmountLD: amountReceived});

        return (limit, new OFTFeeDetail[](1), receipt);
    }

    function quoteSend(SendParam calldata sendParam, bool payInLzToken)
        external
        view
        override
        returns (MessagingFee memory)
    {
        MessagingFee memory fee = MessagingFee({nativeFee: _nativeFee, lzTokenFee: 0});

        unchecked {
            payInLzToken;
            sendParam;
        }

        return fee;
    }

    function setNativeFee(uint256 fee) external {
        _nativeFee = fee;
    }

    function setReceiptAmount(uint256 amount) external {
        _receiptAmount = amount;
    }

    function token() external view override returns (address) {
        return _token;
    }

    function oftVersion() external pure override returns (bytes4 interfaceId, uint64 version) {
        return (0x02e49c2c, 1);
    }

    function approvalRequired() external pure override returns (bool) {
        return false;
    }

    function sharedDecimals() external pure override returns (uint8) {
        return 18;
    }

    function stargateType() external pure override returns (StargateType) {
        return StargateType.OFT; // or StargateType.Pool depending on your use case
    }

    receive() external payable {}
}
