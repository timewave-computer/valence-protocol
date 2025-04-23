use alloy_sol_types::sol;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint64;

// We make a more Cosmwasm friendly version of the original msg
// Since the bytes with a Hex prefix are needed, we are not using HexBinary but instead a String that we will build correctly in the Union Transfer library
#[cw_serde]
pub enum ExecuteMsg {
    Send {
        channel_id: u64,
        timeout_height: Uint64,
        timeout_timestamp: Uint64,
        salt: String,
        instruction: String,
    },
}

// Types that are used for the Instruction
sol! {
    struct Instruction {
        uint8 version;
        uint8 opcode;
        bytes operand;
    }

    struct Batch {
        Instruction[] instructions;
    }

    struct FungibleAssetOrder {
        bytes sender;
        bytes receiver;
        bytes baseToken;
        uint256 baseAmount;
        string baseTokenSymbol;
        string baseTokenName;
        uint8 baseTokenDecimals;
        uint256 baseTokenPath;
        bytes quoteToken;
        uint256 quoteAmount;
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{hex::FromHex, Bytes, U256};
    use alloy_sol_types::SolValue;
    use cosmwasm_std::{to_json_string, Addr, Uint128, Uint256};

    use crate::msg::{CheckedUnionDenomConfig, Config, TransferAmount};

    use super::*;

    #[test]
    fn test_serialize_execute_msg() {
        let bytes_salt = Bytes::from(&[0xde, 0xad, 0xbe, 0xef]);
        let bytes_instruction = Bytes::from(&[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]);

        // Create an ExecuteMsg::Send instance
        let msg = ExecuteMsg::Send {
            channel_id: 5,
            timeout_height: Uint64::new(100),
            timeout_timestamp: Uint64::new(1634567890),
            salt: bytes_salt.to_string(),
            instruction: bytes_instruction.to_string(),
        };

        // Serialize to JSON and check
        let serialized = to_json_string(&msg).unwrap();
        assert_eq!(
            serialized,
            r#"{"send":{"channel_id":5,"timeout_height":"100","timeout_timestamp":"1634567890","salt":"0xdeadbeef","instruction":"0x0123456789abcdef"}}"#
        );
    }

    #[test]
    fn test_decode_real_instruction() {
        // Real instruction example taken from a real transaction
        let instruction_hex = "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000003c000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000002e0000000000000000000000000000000000000000000000000000000000000014000000000000000000000000000000000000000000000000000000000000001a000000000000000000000000000000000000000000000000000000000000001e00000000000000000000000000000000000000000000000000000000002217153000000000000000000000000000000000000000000000000000000000000022000000000000000000000000000000000000000000000000000000000000002600000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002a00000000000000000000000000000000000000000000000000000000000ff8693000000000000000000000000000000000000000000000000000000000000002a62626e31657032756d6a366b6e3334673274746a616c73633572397738707437737634786a7537333472000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000014e7c952d457121ba8f02df1b1d85b26de80a6f1ac00000000000000000000000000000000000000000000000000000000000000000000000000000000000000047562626e0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000047562626e0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000047562626e000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000014e53dCec07d16D88e386AE0710E86d9a400f83c31000000000000000000000000";

        // Convert hex string to bytes - Bytes should handle the 0x prefix correctly
        let instruction_bytes =
            Bytes::from_hex(instruction_hex).expect("Failed to parse hex string");

        // Decode the instruction
        let instruction = Instruction::abi_decode_params(&instruction_bytes, true)
            .expect("Failed to decode instruction");

        // Verify instruction fields
        assert_eq!(instruction.version, 0);
        assert_eq!(instruction.opcode, 2);

        // Print decoded instruction details for debugging
        println!("Instruction successfully decoded:");
        println!("  Version: {}", instruction.version);
        println!("  Opcode: {}", instruction.opcode);
        println!("  Operand length: {} bytes", instruction.operand.len());

        // Decode the operand as a Batch
        let batch = Batch::abi_decode_params(&instruction.operand, true)
            .expect("Failed to decode Batch from operand");

        // Verify Batch fields
        assert_eq!(batch.instructions.len(), 1);
        assert_eq!(batch.instructions[0].version, 1);
        assert_eq!(batch.instructions[0].opcode, 3);

        // Decode the first instruction as a FungibleAssetOrder
        let fungible_asset_order =
            FungibleAssetOrder::abi_decode_params(&batch.instructions[0].operand, true)
                .expect("Failed to decode FungibleAssetOrder from operand");

        // Verify FungibleAssetOrder fields
        assert_eq!(fungible_asset_order.baseTokenSymbol, "ubbn");
        assert_eq!(fungible_asset_order.baseTokenName, "ubbn");
        assert_eq!(fungible_asset_order.baseTokenDecimals, 6);

        // Extract and print important fields for verification
        println!("\nFungibleAssetOrder details:");
        println!("  Sender: {}", fungible_asset_order.sender);
        println!("  Receiver: {}", fungible_asset_order.receiver);
        println!("  Base token: {}", fungible_asset_order.baseToken);
        println!("  Base amount: {}", fungible_asset_order.baseAmount);
        println!(
            "  Base token symbol: {}",
            fungible_asset_order.baseTokenSymbol
        );
        println!("  Base token name: {}", fungible_asset_order.baseTokenName);
        println!(
            "  Base token decimals: {}",
            fungible_asset_order.baseTokenDecimals
        );
        println!("  Quote token: {}", fungible_asset_order.quoteToken);
        println!("  Quote amount: {}", fungible_asset_order.quoteAmount);
        println!("  Base token path: {}", fungible_asset_order.baseTokenPath);
    }

    #[test]
    fn correct_instruction_from_config() {
        // Create an example Config object and verify that all fields are transformed correctly
        let cfg = Config::new(
            Addr::unchecked(
                "bbn1rvp882qs76sawd6ejydst28272t4au3n3hl79cwt8xsc7t3kp8rs26uc7z".to_string(),
            ),
            "0xe7c952d457121ba8f02df1b1d85b26de80a6f1ac".to_string(),
            CheckedUnionDenomConfig::Native("ubbn".to_string()),
            TransferAmount::FixedAmount(Uint128::from(100000000u128)),
            "ubbn".to_string(),
            "ubbn".to_string(),
            6,
            Uint256::zero(),
            "0xe53dcec07d16d88e386ae0710e86d9a400f83c31".to_string(),
            Uint256::from(100000000u128),
            3,
            None,
            Addr::unchecked(
                "bbn1336jj8ertl8h7rdvnz4dh5rqahd09cy0x43guhsxx6xyrztx292q77945h".to_string(),
            ),
            None,
            None,
        );

        let fungible_asset_order = FungibleAssetOrder {
            sender: Bytes::from(cfg.input_addr.to_string().into_bytes()),
            receiver: Bytes::from_hex(&cfg.output_addr).unwrap(),
            baseToken: Bytes::from(cfg.denom.to_string().into_bytes()),
            baseAmount: U256::from(100000000u128),
            baseTokenSymbol: cfg.input_asset_symbol,
            baseTokenName: cfg.input_asset_name,
            baseTokenDecimals: cfg.input_asset_decimals,
            baseTokenPath: U256::from_be_bytes(cfg.input_asset_token_path.to_be_bytes()),
            quoteToken: Bytes::from_hex(cfg.quote_token).unwrap(),
            quoteAmount: U256::from(100000000u128),
        };

        // Verify the fields
        assert_eq!(fungible_asset_order.sender.to_string(), "0x62626e31727670383832717337367361776436656a79647374323832373274346175336e33686c3739637774387873633774336b7038727332367563377a".to_string());
        assert_eq!(
            fungible_asset_order.receiver.to_string(),
            "0xe7c952d457121ba8f02df1b1d85b26de80a6f1ac".to_string()
        );
        assert_eq!(
            fungible_asset_order.baseToken.to_string(),
            "0x7562626e".to_string() // "ubbn"
        );

        assert_eq!(fungible_asset_order.baseTokenPath, U256::from(0));
        assert_eq!(
            fungible_asset_order.quoteToken.to_string(),
            "0xe53dcec07d16d88e386ae0710e86d9a400f83c31".to_string()
        );
    }
}
