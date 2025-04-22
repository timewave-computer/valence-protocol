#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use valence_library_utils::{
    error::LibraryError,
    msg::{ExecuteMsg, InstantiateMsg},
};

use crate::{
    msg::{Config, FunctionMsgs, LibraryConfig, LibraryConfigUpdate, QueryMsg},
    state::COUNTER,
};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_TIMEOUT_SECONDS: u64 = 259200; // 3 days
const DEFAULT_BATCH_INSTRUCTION_VERSION: u8 = 0x00;
const DEFAULT_TRANSFER_INSTRUCTION_VERSION: u8 = 0x01;
const BATCH_OP_CODE: u8 = 0x02; // OP_CODE for batch
const TRANSFER_OP_CODE: u8 = 0x03; // OP_CODE for transfer

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg<LibraryConfig>,
) -> Result<Response, LibraryError> {
    // Initialize the counter
    COUNTER.save(deps.storage, &0)?;

    valence_library_base::instantiate(deps, CONTRACT_NAME, CONTRACT_VERSION, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<FunctionMsgs, LibraryConfigUpdate>,
) -> Result<Response, LibraryError> {
    valence_library_base::execute(
        deps,
        env,
        info,
        msg,
        functions::process_function,
        execute::update_config,
    )
}

mod functions {
    use alloy_primitives::{
        hex::{self, FromHex},
        Bytes, U256,
    };
    use alloy_sol_types::SolValue;
    use cosmwasm_std::{
        to_json_binary, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdResult, Uint64, WasmMsg,
    };
    use cw20::Cw20ExecuteMsg;
    use sha2::{Digest, Sha256};
    use valence_library_utils::{error::LibraryError, execute_on_behalf_of};

    use crate::{
        msg::{CheckedUnionDenomConfig, Config, FunctionMsgs, TransferAmount},
        state::COUNTER,
        union::{self, Batch, FungibleAssetOrder, Instruction},
    };

    use super::{
        BATCH_OP_CODE, DEFAULT_BATCH_INSTRUCTION_VERSION, DEFAULT_TIMEOUT_SECONDS,
        DEFAULT_TRANSFER_INSTRUCTION_VERSION, TRANSFER_OP_CODE,
    };

    pub fn process_function(
        deps: DepsMut,
        env: Env,
        _info: MessageInfo,
        msg: FunctionMsgs,
        cfg: Config,
    ) -> Result<Response, LibraryError> {
        match msg {
            FunctionMsgs::Transfer { quote_amount } => {
                let balance = cfg.denom.query_balance(&deps.querier, &cfg.input_addr)?;

                let amount = match cfg.amount {
                    TransferAmount::FullAmount => balance,
                    TransferAmount::FixedAmount(amount) => {
                        if balance < amount {
                            return Err(LibraryError::ExecutionError(format!(
                                "Insufficient balance for denom '{}' in config (required: {}, available: {}).",
                                cfg.denom, amount, balance,
                            )));
                        }
                        amount
                    }
                };

                // Messages to be used for the transfer
                let mut msgs = vec![];

                // If the token we are sending is Cw20, we first need to approve the token minter to spend the tokens
                // This is how the union transfer works for Cw20 tokens
                if let CheckedUnionDenomConfig::Cw20(ref checked_union_cw20_config) = cfg.denom {
                    let allowance_msg = Cw20ExecuteMsg::IncreaseAllowance {
                        spender: checked_union_cw20_config.minter.to_string(),
                        amount,
                        expires: None,
                    };

                    let cosmos_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: checked_union_cw20_config.token.to_string(),
                        msg: to_json_binary(&allowance_msg)?,
                        funds: vec![],
                    });

                    msgs.push(cosmos_msg);
                }

                // If we are passing the quote_amount in the message, we will use that one, otherwise the one in the config
                let quote_amount = if let Some(quote_amount) = quote_amount {
                    U256::from_be_bytes(quote_amount.to_be_bytes())
                } else {
                    U256::from_be_bytes(cfg.quote_amount.to_be_bytes())
                };

                // Let's create the transfer instruction that will go inside the batch
                let fungible_asset_order = FungibleAssetOrder {
                    sender: Bytes::from(cfg.input_addr.to_string().into_bytes()), // The sender needs to be the bytes of the address
                    receiver: Bytes::from_hex(&cfg.output_addr).map_err(|_| {
                        LibraryError::ExecutionError(
                            "The receiver address is not a valid EVM address.".to_string(),
                        )
                    })?, // The receiver is already in hex format
                    baseToken: Bytes::from(cfg.denom.to_string().into_bytes()), // The base token is the denom we are sending
                    baseAmount: U256::from(amount.u128()), // The base amount is the amount we are sending
                    baseTokenSymbol: cfg.input_asset_symbol,
                    baseTokenName: cfg.input_asset_name,
                    baseTokenDecimals: cfg.input_asset_decimals,
                    baseTokenPath: U256::from_be_bytes(cfg.input_asset_token_path.to_be_bytes()),
                    quoteToken: Bytes::from_hex(cfg.quote_token).map_err(|_| {
                        LibraryError::ExecutionError(
                            "The quote token is not a valid EVM address.".to_string(),
                        )
                    })?, // The quote token is the output asset token path
                    quoteAmount: quote_amount,
                };

                let transfer_instruction = Instruction {
                    version: cfg
                        .transfer_instruction_version
                        .unwrap_or(DEFAULT_TRANSFER_INSTRUCTION_VERSION),
                    opcode: TRANSFER_OP_CODE,
                    operand: fungible_asset_order.abi_encode_params().into(),
                };

                // Now we create the batch instruction that will contain this one
                let batch = Batch {
                    instructions: vec![transfer_instruction],
                };
                let batch_instruction = Instruction {
                    version: cfg
                        .batch_instruction_version
                        .unwrap_or(DEFAULT_BATCH_INSTRUCTION_VERSION),
                    opcode: BATCH_OP_CODE,
                    operand: batch.abi_encode_params().into(),
                };
                let bytes_instruction: Bytes = batch_instruction.abi_encode_params().into();

                // Let's generate a unique salt for the transaction
                let counter = COUNTER.update(deps.storage, |mut counter| -> StdResult<_> {
                    counter += 1;
                    Ok(counter)
                })?;
                let salt = Sha256::new()
                    .chain_update(cfg.input_addr.to_string().as_bytes())
                    .chain_update(env.block.time.seconds().to_be_bytes())
                    .chain_update(counter.to_be_bytes())
                    .finalize();

                // Create the send message
                let send_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cfg.zkgm_contract.to_string(),
                    msg: to_json_binary(&union::ExecuteMsg::Send {
                        channel_id: cfg.channel_id,
                        timeout_height: Uint64::zero(),
                        timeout_timestamp: Uint64::from(
                            env.block
                                .time
                                .plus_seconds(
                                    cfg.transfer_timeout.unwrap_or(DEFAULT_TIMEOUT_SECONDS),
                                )
                                .nanos(),
                        ),
                        salt: Bytes::from_hex(hex::encode(salt))
                            .map_err(|_| {
                                LibraryError::ExecutionError("Can't encode the salt.".to_string())
                            })?
                            .to_string(),
                        instruction: bytes_instruction.to_string(),
                    })?,
                    funds: vec![],
                });
                msgs.push(send_msg);

                let input_account_msgs = execute_on_behalf_of(msgs, &cfg.input_addr)?;

                Ok(Response::new()
                    .add_attribute("method", "union-transfer")
                    .add_message(input_account_msgs))
            }
        }
    }
}

mod execute {
    use cosmwasm_std::{DepsMut, Env, MessageInfo};
    use valence_library_utils::error::LibraryError;

    use crate::msg::LibraryConfigUpdate;

    pub fn update_config(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        new_config: LibraryConfigUpdate,
    ) -> Result<(), LibraryError> {
        new_config.update_config(deps)
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => {
            to_json_binary(&valence_library_base::get_ownership(deps.storage)?)
        }
        QueryMsg::GetProcessor {} => {
            to_json_binary(&valence_library_base::get_processor(deps.storage)?)
        }
        QueryMsg::GetLibraryConfig {} => {
            let config: Config = valence_library_base::load_config(deps.storage)?;
            to_json_binary(&config)
        }
        QueryMsg::GetRawLibraryConfig {} => {
            let raw_config: LibraryConfig =
                valence_library_utils::raw_config::query_raw_library_config(deps.storage)?;
            to_json_binary(&raw_config)
        }
    }
}
