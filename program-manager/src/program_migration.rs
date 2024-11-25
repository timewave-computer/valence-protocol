use std::collections::VecDeque;

use anyhow::Context;
use cosmwasm_schema::schemars::JsonSchema;
use cosmwasm_std::{to_json_binary, Coin, CosmosMsg, WasmMsg};

use serde::{Deserialize, Serialize};
use valence_authorization_utils::{
    authorization::{AuthorizationInfo, AuthorizationModeInfo, Priority},
    authorization_message::{Message, MessageDetails, MessageType},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
    msg::ProcessorMessage,
};
use valence_library_utils::{GetId, Id, LibraryAccountType};

use crate::{
    connectors::Connectors,
    domain::Domain,
    error::{ManagerError, ManagerResult},
    program_config::ProgramConfig,
    NEUTRON_CHAIN,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, JsonSchema)]
#[schemars(crate = "cosmwasm_schema::schemars")]
pub struct FundsTransfer {
    pub from: String,
    pub to: LibraryAccountType,
    pub domain: Domain,
    pub funds: Coin,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MigrateResponse {
    pub instructions: Vec<CosmosMsg>,
    pub pause_processor_messages: Vec<CosmosMsg>,
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

        let mut old_config = neutron_connector.get_program_config(self.old_id).await?;

        // Verify the migration config
        self.verify_config_migration(&old_config)?;

        // After we verified the migration config is correct, we can start the migration

        // We drop the connector here to free it for the init functionlity.
        drop(neutron_connector);

        // Create the new program
        self.new_program.init(connectors).await?;

        let mut instructions: VecDeque<CosmosMsg> = VecDeque::new();
        let mut new_authorizations: Vec<AuthorizationInfo> = vec![];

        for transfer_funds in self.transfer_funds.iter() {
            // Get the account id we are sending funds from
            // We unwrap because we already verified this account exists
            let account_id = old_config
                .accounts
                .iter()
                .find(|(_, acc_info)| acc_info.addr.clone().unwrap() == transfer_funds.from)
                .map(|(id, _)| *id)
                .unwrap();

            // We set no restrictions on this authorization, so we can have a generic "open" authorization on the account
            // This authorization can be only executed by the owner, so its fine.
            let label = format!("account_id_{}", account_id);

            // We skip creating this authorization because we already have it
            if !old_config
                .authorizations
                .iter()
                .any(|auth| auth.label == label)
            {
                // get what domain this can be executed on
                let domain = if transfer_funds.domain == neutron_domain {
                    valence_authorization_utils::domain::Domain::Main
                } else {
                    valence_authorization_utils::domain::Domain::External(
                        transfer_funds.domain.to_string(),
                    )
                };

                // Build the authorization
                let subroutine = AtomicSubroutineBuilder::new()
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_domain(domain)
                            .with_contract_address(LibraryAccountType::Addr(
                                transfer_funds.from.clone(),
                            ))
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "execute_msg".to_string(),
                                    params_restrictions: None,
                                },
                            })
                            .build(),
                    )
                    .build();

                let authorization_builder = AuthorizationBuilder::new()
                .with_label(&label)
                .with_mode(AuthorizationModeInfo::Permissioned(
                    valence_authorization_utils::authorization::PermissionTypeInfo::WithoutCallLimit(vec![old_config.owner.clone()]),
                ))
                .with_priority(Priority::High)
                .with_subroutine(subroutine);

                let authorization_info = authorization_builder.build();

                new_authorizations.push(authorization_info.clone());
                old_config.authorizations.push(authorization_info);
            }

            // Build the messages of the funds transfer
            // execute insert message on the authorization to push this message to processor
            let send_to_addr = self
                .new_program
                .get_account(transfer_funds.to.get_account_id())?
                .addr
                .clone()
                .context(format!(
                    "Account id: {} doesn't have address in new config",
                    transfer_funds.to.get_account_id()
                ))?;

            let transfer_msg = cosmwasm_std::BankMsg::Send {
                to_address: send_to_addr,
                amount: vec![transfer_funds.funds.clone()],
            };

            let account_execute_msg =
                to_json_binary(&valence_account_utils::msg::ExecuteMsg::ExecuteMsg {
                    msgs: vec![transfer_msg.into()],
                })
                .context("Migrate: failed to parse to binary Account::ExecuteMsg")?;

            instructions.push_back(
                WasmMsg::Execute {
                    contract_addr: old_config.authorization_data.authorization_addr.clone(),
                    msg: to_json_binary(
                        &valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
                            valence_authorization_utils::msg::PermissionedMsg::InsertMsgs {
                                label,
                                queue_position: 0,
                                priority: Priority::High,
                                messages: vec![ProcessorMessage::CosmwasmExecuteMsg {
                                    msg: account_execute_msg,
                                }],
                            },
                        ),
                    )
                    .context("Failed binary parsing InsertMsgs")?,
                    funds: vec![],
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

        // Add all processor halt messages
        let mut pause_processor_messages: Vec<CosmosMsg> = vec![];

        for (domain, _) in old_config.authorization_data.processor_addrs.clone() {
            let domain = if Domain::from_string(domain.clone())? == neutron_domain {
                valence_authorization_utils::domain::Domain::Main
            } else {
                valence_authorization_utils::domain::Domain::External(domain.clone())
            };

            pause_processor_messages.push(
                WasmMsg::Execute {
                    contract_addr: old_config.authorization_data.authorization_addr.clone(),
                    msg: to_json_binary(
                        &valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
                            valence_authorization_utils::msg::PermissionedMsg::PauseProcessor {
                                domain,
                            },
                        ),
                    )
                    .unwrap(),
                    funds: vec![],
                }
                .into(),
            )
        }

        // Save the updated config to the registry
        let mut neutron_connector = connectors.get_or_create_connector(&neutron_domain).await?;

        neutron_connector
            .update_program_config(old_config.clone())
            .await?;

        Ok(MigrateResponse {
            instructions: instructions.into(),
            new_config: self.new_program.clone(),
            pause_processor_messages,
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
            let funds_transfer_to_id = funds_transfer.to.get_account_id();

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
