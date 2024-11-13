use std::collections::VecDeque;

use cosmwasm_schema::schemars::JsonSchema;
use cosmwasm_std::{to_json_binary, Coin, CosmosMsg, WasmMsg};

use serde::{Deserialize, Serialize};
use valence_authorization_utils::authorization::AuthorizationInfo;
use valence_library_utils::{GetId, Id, LibraryAccountType};

use crate::{
    connectors::Connectors,
    domain::Domain,
    error::{ManagerError, ManagerResult},
    init_program,
    program_config::ProgramConfig,
    NEUTRON_CHAIN,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, JsonSchema)]
#[schemars(crate = "cosmwasm_schema::schemars")]
pub struct FundsTransfer {
    from: String,
    to: LibraryAccountType,
    domain: Domain,
    funds: Coin,
}

/// We allow to migrate an existing program to a new one
/// This is done by creating a new program and transfering funds from the old program accounts to the new accounts
/// Because the new program can be any configuration, we can the user to tell us
/// what funds to move from where to what accounts
/// Note: We assume funds are moved FROM accounts to another contract, on the same domain
/// At least for V1
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[schemars(crate = "cosmwasm_schema::schemars")]
pub struct ProgramConfigMigrate {
    pub old_id: Id,
    /// The new program we instantiate
    pub new_program: ProgramConfig,
    /// Transfer funds details
    pub transfer_funds: Vec<FundsTransfer>,
}

#[derive(Clone, Debug)]
pub struct MigrateResponse {
    pub instructions: Vec<CosmosMsg>,
    pub new_config: ProgramConfig,
}

impl ProgramConfigMigrate {
    /// Migrate old program to new program
    /// We first verify the migration data is correct
    /// We then create a new program
    /// We then create authorizations to transfer funds and insert the messages
    /// Then we build the messages to stop all processors.
    /// NOTE: stopping all processor must happen after transfer of funds was completed, else
    /// the transfer message will be stuck in the processor.
    pub async fn migrate(&mut self, connectors: &Connectors) -> ManagerResult<MigrateResponse> {
        let neutron_domain = Domain::CosmosCosmwasm(NEUTRON_CHAIN.to_string());

        // Get the old program config from registry
        let mut neutron_connector = connectors.get_or_create_connector(&neutron_domain).await?;

        let old_config = neutron_connector.get_program_config(self.old_id).await?;

        // Verify the migration config
        self.verify_config_migration(&old_config)?;

        // After we verified the migration config is correct, we can start the migration

        // Create the new program
        init_program(&mut self.new_program).await?;

        let mut instructions: VecDeque<CosmosMsg> = VecDeque::new();
        let mut new_authorizations: Vec<AuthorizationInfo> = vec![];

        for (i, transfer_funds) in self.transfer_funds.iter().enumerate() {
            // Transfer funds from old program to new program
            instructions.push_front(
                WasmMsg::Execute {
                    contract_addr: transfer_funds.from.clone(),
                    msg: to_json_binary(
                        &valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
                            valence_authorization_utils::msg::PermissionedMsg::TransferFunds {
                                to: transfer_funds
                                    .to
                                    .to_string()
                                    .map_err(|e| ManagerError::generic_err(e.to_string()))?,
                                amount: transfer_funds.funds.clone(),
                            },
                        ),
                    )
                    .unwrap(),
                    funds: vec![transfer_funds.funds.clone()],
                }
                .into(),
            );
        }

        // Add all new authorizations
        instructions.push_front(
            WasmMsg::Execute {
                contract_addr: old_config.authorization_data.authorization_addr.clone(),
                msg: to_json_binary(
                    &valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
                        valence_authorization_utils::msg::PermissionedMsg::CreateAuthorizations {
                            authorizations: new_authorizations,
                        },
                    ),
                )
                .unwrap(),
                funds: vec![],
            }
            .into(),
        );

        // Save the updated config to the registry
        neutron_connector
            .save_program_config(self.new_program.clone())
            .await?;

        Ok(MigrateResponse {
            instructions: instructions.into(),
            new_config: self.new_program.clone(),
        })
    }

    /// Verify that the migration data is correct
    /// Make sure old program id is not 0
    /// For each funds transfer make sure the amount is not zero, the account we send funds from
    /// exists on the old program and that id we send funds to exists in the new program config
    fn verify_config_migration(&self, old_config: &ProgramConfig) -> ManagerResult<()> {
        if self.old_id == 0 {
            return Err(ManagerError::InvalidProgramId);
        }

        for funds_transfer in self.transfer_funds.iter() {
            // We make sure the amount is not zero
            if funds_transfer.funds.amount.is_zero() {
                return Err(ManagerError::FundsTransferAmountZero(
                    funds_transfer.from.clone(),
                    funds_transfer
                        .to
                        .to_string()
                        .map_err(|e| ManagerError::generic_err(e.to_string()))?,
                ));
            }

            // We make sure the account to send funds from exists in the old config
            if !old_config
                .accounts
                .iter()
                .any(|(_, acc_info)| acc_info.addr.clone().unwrap() == funds_transfer.from)
            {
                return Err(ManagerError::AccountNotFoundInOldProgram(
                    funds_transfer.from.clone(),
                ));
            }

            // Make sure the account we sent to exists in the new program config
            let funds_transfer_to_id = funds_transfer.to.get_id();

            if !self
                .new_program
                .accounts
                .iter()
                .any(|(id, _)| *id == funds_transfer_to_id)
            {
                return Err(ManagerError::AccountIdWasNotFound(funds_transfer_to_id));
            }
        }

        Ok(())
    }
}

fn verify_authorization_not_exists(
    authorizations: &[AuthorizationInfo],
    label: String,
) -> ManagerResult<()> {
    if authorizations.iter().any(|auth| auth.label == label) {
        return Err(ManagerError::AuthorizationLabelExists(label));
    }

    Ok(())
}

fn verify_authorization_exists(
    authorizations: &[AuthorizationInfo],
    label: String,
) -> ManagerResult<()> {
    if !authorizations.iter().any(|auth| auth.label == label) {
        return Err(ManagerError::AuthorizationLabelNotFound(label));
    }

    Ok(())
}
