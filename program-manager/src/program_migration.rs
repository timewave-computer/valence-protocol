use std::collections::{BTreeMap, VecDeque};

use anyhow::Context;
use cosmwasm_schema::schemars::JsonSchema;
use cosmwasm_std::{to_json_binary, Coin, CosmosMsg, WasmMsg};
use cw_ownable::Expiration;
use serde::{Deserialize, Serialize};
use valence_authorization_utils::{
    authorization::{AuthorizationInfo, AuthorizationModeInfo, Priority},
    authorization_message::{Message, MessageDetails, MessageType},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
    msg::ProcessorMessage,
};
use valence_library_utils::{Id, LibraryAccountType};

use crate::{
    connectors::Connectors, domain::Domain, error::{ManagerError, ManagerResult}, library::LibraryConfigUpdate, program_config::ProgramConfig, NEUTRON_CHAIN
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct FundsTransfer {
    from: String,
    to: String,
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
    /// The new program we instantiate
    pub new_program: ProgramConfig,
    /// Transfer funds details
    pub transfer_funds: Vec<FundsTransfer>,
    pub authorizations: Vec<AuthorizationInfoUpdate>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[schemars(crate = "cosmwasm_schema::schemars")]
pub enum AuthorizationInfoUpdate {
    Add(AuthorizationInfo),
    Modify {
        label: String,
        not_before: Option<Expiration>,
        expiration: Option<Expiration>,
        max_concurrent_executions: Option<u64>,
        priority: Option<Priority>,
    },
    /// Disable by label
    Disable(String),
    /// Disable by label
    Enable(String),
}

#[derive(Clone, Debug)]
pub struct UpdateResponse {
    pub instructions: Vec<CosmosMsg>,
}

impl ProgramConfigUpdate {
    /// Modify an existing config with a new config
    pub async fn update(&mut self, connectors: &Connectors) -> ManagerResult<UpdateResponse> {
        let neutron_domain = Domain::CosmosCosmwasm(NEUTRON_CHAIN.to_string());

        // get the old program config from registry
        let mut neutron_connector = connectors.get_or_create_connector(&neutron_domain).await?;

        if self.id == 0 {
            return Err(ManagerError::IdIsZero);
        }

        let mut config = neutron_connector.get_program_config(self.id).await?;

        let mut instructions: VecDeque<CosmosMsg> = VecDeque::new();
        let mut new_authorizations: Vec<AuthorizationInfo> = vec![];

        if let Some(new_owner) = self.owner.clone() {
            config.owner = new_owner.clone();

            // Create instruction to change owner
            instructions.push_back(
                WasmMsg::Execute {
                    contract_addr: config.authorization_data.authorization_addr.clone(),
                    msg: to_json_binary(&cw_ownable::Action::TransferOwnership {
                        new_owner,
                        expiry: None,
                    })
                    .context("Failed binary parsing TransferOwnership")?,
                    funds: vec![],
                }
                .into(),
            );
        }

        for (id, library_update) in self.libraries.iter() {
            // Verify that the library id exists in the config and get it
            let library = config
                .libraries
                .get(id)
                .context(ManagerError::LibraryIdIsMissing(*id).to_string())?;

            // Add authorization to update the library
            let label = format!("update_library_{}_{}", library.name, id);

            // Create authorization if we don't already have one
            if !config.authorizations.iter().any(|auth| auth.label == label) {
                let library_domain = if library.domain == neutron_domain {
                    valence_authorization_utils::domain::Domain::Main
                } else {
                    valence_authorization_utils::domain::Domain::External(
                        library.domain.to_string(),
                    )
                };
                let actions_config = AtomicSubroutineBuilder::new()
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_domain(library_domain)
                            .with_contract_address(LibraryAccountType::Addr(
                                library.addr.clone().unwrap(),
                            ))
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "update_config".to_string(),
                                    params_restrictions: None,
                                },
                            })
                            .build(),
                    )
                    .build();
                let authorization_builder = AuthorizationBuilder::new()
                    .with_label(&label)
                    .with_mode(AuthorizationModeInfo::Permissioned(
                        valence_authorization_utils::authorization::PermissionTypeInfo::WithoutCallLimit(vec![config.owner.clone()]),
                    ))
                    .with_priority(Priority::High)
                    .with_subroutine(actions_config);

                new_authorizations.push(authorization_builder.build());
            }

            // execute insert message on the authorization
            let update_config_msg = library_update
                .clone()
                .get_update_msg()
                .context("Failed binary parsing get_update_msg")?;

            instructions.push_back(
                WasmMsg::Execute {
                    contract_addr: config.authorization_data.authorization_addr.clone(),
                    msg: to_json_binary(
                        &valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
                            valence_authorization_utils::msg::PermissionedMsg::InsertMsgs {
                                label,
                                queue_position: 0,
                                priority: Priority::High,
                                messages: vec![ProcessorMessage::CosmwasmExecuteMsg {
                                    msg: update_config_msg,
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

        // Generate authorization update instructions
        for authorization in self.authorizations.clone().into_iter() {
            match authorization {
                AuthorizationInfoUpdate::Add(authorization_info) => {
                    verify_authorization_not_exists(
                        &config.authorizations,
                        authorization_info.label.clone(),
                    )?;

                    // Create instruction for adding authorization
                    new_authorizations.push(authorization_info.clone());

                    // Add new authorizations to our config saved in registry
                    config.authorizations.push(authorization_info);
                }
                AuthorizationInfoUpdate::Modify {
                    label,
                    not_before,
                    expiration,
                    max_concurrent_executions,
                    priority,
                } => {
                    verify_authorization_exists(&config.authorizations, label.clone())?;

                    // Create instruction for modifying authorization
                    instructions.push_back(WasmMsg::Execute {
                        contract_addr: config.authorization_data.authorization_addr.clone(),
                        msg: to_json_binary(&valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
                            valence_authorization_utils::msg::PermissionedMsg::ModifyAuthorization { label: label.clone(), not_before, expiration, max_concurrent_executions, priority: priority.clone() }
                        )).context("Failed binary parsing AuthorizationInfoUpdate::Modify")?,
                        funds: vec![]
                    }.into());

                    // Modify saved config with the new modified authorizations
                    let auth = config
                        .authorizations
                        .iter_mut()
                        .find(|a| a.label == label)
                        .context(format!("Failed to find authorization {}", label))?;

                    if let Some(not_before) = not_before {
                        auth.not_before = not_before;
                    }

                    auth.priority = priority;
                    auth.max_concurrent_executions = max_concurrent_executions;
                }
                AuthorizationInfoUpdate::Disable(label) => {
                    verify_authorization_exists(&config.authorizations, label.clone())?;

                    // Create instruction for disabling authorization
                    instructions.push_back(WasmMsg::Execute {
                        contract_addr: config.authorization_data.authorization_addr.clone(),
                        msg: to_json_binary(&valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
                            valence_authorization_utils::msg::PermissionedMsg::DisableAuthorization { label }
                        )).unwrap(),
                        funds: vec![]
                    }.into());
                }
                AuthorizationInfoUpdate::Enable(label) => {
                    verify_authorization_exists(&config.authorizations, label.clone())?;

                    // Create instruction for enabling authorization
                    instructions.push_back(WasmMsg::Execute {
                        contract_addr: config.authorization_data.authorization_addr.clone(),
                        msg: to_json_binary(&valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
                            valence_authorization_utils::msg::PermissionedMsg::EnableAuthorization { label }
                        )).unwrap(),
                        funds: vec![]
                    }.into());
                }
            }
        }

        // Add all new authorizations
        instructions.push_front(
            WasmMsg::Execute {
                contract_addr: config.authorization_data.authorization_addr.clone(),
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
        neutron_connector.update_program_config(config).await?;

        Ok(UpdateResponse {
            instructions: instructions.into(),
        })
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
