// Purpose: JIT account contract with controller-bound execution
//
// JIT (Just-In-Time) accounts are lightweight account implementations designed
// for factory-created accounts. They provide controller-bound execution where
// only the designated controller or approved libraries can perform operations.
//
// Key Features:
// - Controller-bound execution (only controller can approve libraries)
// - Library approval system (libraries can execute once approved)
// - Full capabilities (both token custody and data storage)
// - Minimal state storage for gas efficiency
// - Message forwarding to authorized libraries

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response,
    StdError, StdResult,
};
use cw2::set_contract_version;
use thiserror::Error;

use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{APPROVED_LIBRARIES, CONTROLLER};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Contract-specific errors for JIT account operations
#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("Unauthorized")]
    Unauthorized {},
}

/// Initialize a new JIT account with controller binding
///
/// Sets up the account with an immutable controller address.
/// The controller is the only entity that can approve/remove libraries and
/// execute messages directly. All accounts have full capabilities for both
/// token operations and data storage.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    // Set contract version for migration tracking
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Store controller address (immutable after instantiation)
    let controller = deps.api.addr_validate(&msg.controller)?;
    CONTROLLER.save(deps.storage, &controller)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("controller", controller))
}

/// Execute operations on the JIT account
///
/// Handles library approval/removal (controller only) and message execution
/// (controller or approved libraries). All operations include authorization
/// checks to ensure only permitted entities can perform actions.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // Approve a library for executing messages on behalf of this account
        ExecuteMsg::ApproveLibrary { library } => execute::approve_library(deps, info, library),

        // Remove approval for a library
        ExecuteMsg::RemoveLibrary { library } => execute::remove_library(deps, info, library),

        // Execute one or more CosmosMsg through this account
        ExecuteMsg::Execute { msgs } => execute::execute_msgs(deps, info, msgs),
    }
}

/// Internal execution module containing authorization logic
mod execute {
    use super::*;

    /// Approve a library to execute messages on behalf of this account
    ///
    /// Only the controller can approve libraries. Once approved, a library
    /// can call the Execute method to perform operations using this account's
    /// permissions and capabilities.
    pub fn approve_library(
        deps: DepsMut,
        info: MessageInfo,
        library: String,
    ) -> Result<Response, ContractError> {
        // Load controller address and verify sender authorization
        let controller = CONTROLLER.load(deps.storage)?;
        ensure!(info.sender == controller, ContractError::Unauthorized {});

        // Validate and store library address
        let library_addr = deps.api.addr_validate(&library)?;
        APPROVED_LIBRARIES.save(deps.storage, library_addr.clone(), &Empty {})?;

        Ok(Response::new()
            .add_attribute("method", "approve_library")
            .add_attribute("library", library_addr))
    }

    /// Remove approval for a library
    ///
    /// Only the controller can remove library approvals. Once removed,
    /// the library can no longer execute messages through this account.
    pub fn remove_library(
        deps: DepsMut,
        info: MessageInfo,
        library: String,
    ) -> Result<Response, ContractError> {
        // Verify only controller can remove library approvals
        let controller = CONTROLLER.load(deps.storage)?;
        ensure!(info.sender == controller, ContractError::Unauthorized {});

        // Remove library from approved list
        let library_addr = deps.api.addr_validate(&library)?;
        APPROVED_LIBRARIES.remove(deps.storage, library_addr.clone());

        Ok(Response::new()
            .add_attribute("method", "remove_library")
            .add_attribute("library", library_addr))
    }

    /// Execute messages through this account
    ///
    /// Can be called by either the controller directly or by any approved library.
    /// This is the primary mechanism for performing operations (token transfers,
    /// data storage, etc.) using this account's identity and permissions.
    ///
    /// All accounts have full capabilities for both token and data operations.
    pub fn execute_msgs(
        deps: DepsMut,
        info: MessageInfo,
        msgs: Vec<CosmosMsg>,
    ) -> Result<Response, ContractError> {
        let controller = CONTROLLER.load(deps.storage)?;

        // Authorization check: allow controller or approved libraries to execute
        ensure!(
            info.sender == controller || APPROVED_LIBRARIES.has(deps.storage, info.sender.clone()),
            ContractError::Unauthorized {}
        );

        // Forward all provided messages for execution
        // All accounts have full capabilities for token and data operations
        Ok(Response::new()
            .add_messages(msgs)
            .add_attribute("method", "execute_msgs")
            .add_attribute("sender", info.sender))
    }
}

/// Query account state and configuration
///
/// Provides read-only access to account information including controller
/// and library approvals.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // Get the controller address for this account
        QueryMsg::GetController {} => {
            let controller = CONTROLLER.load(deps.storage)?;
            to_json_binary(&controller)
        }

        // Check if a specific library is approved to execute messages
        QueryMsg::IsLibraryApproved { library } => {
            let library_addr = deps.api.addr_validate(&library)?;
            let approved = APPROVED_LIBRARIES.has(deps.storage, library_addr);
            to_json_binary(&approved)
        }
    }
}
