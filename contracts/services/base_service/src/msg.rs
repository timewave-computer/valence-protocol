use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, DepsMut};
use cw_ownable::cw_ownable_execute;
use valence_macros::OptionalStruct;

use crate::{state::CONFIG, ContractError};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
    pub processor: String,
    pub config: ServiceConfig,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    UpdateProcessor { processor: String },
    UpdateConfig { new_config: OptionalServiceConfig },
    Processor(ActionsMsgs),
}

/// TODO: (3) Implement the actions the service can do
#[cw_serde]
pub enum ActionsMsgs {
    Action1 {/* action-specific fields */},
    Action2 {/* action-specific fields */},
    // Additional service-specific actions
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    GetAdmin {},
    #[returns(ServiceConfig)]
    GetServiceConfig {},
}

/// TODO: (1) Change the config based on the service requirements
/// Add here the things you need on instantiate
/// OptionalStruct macro gives us the `OptionalServiceConfig` struct where every field of the ServiceConfig is optional
/// this allows us to easily update only the fields we want to change doing updateConfig functionality
/// You can avoid using OptionalStruct macro and simply impl OptionalServiceConfig struct on your own
#[cw_serde]
#[derive(OptionalStruct)]
pub struct ServiceConfig {
    // Service-specific configuration fields
    some_addr: String,
}

impl OptionalServiceConfig {
    /// TODO: (2) Implement the update_config function to update config
    /// Field list matches the fields in the ServiceConfig struct, but all of them are optional
    /// if a field is Some, it means we want to update it.
    /// You can return here anything the service needs
    pub fn update_config(self, deps: DepsMut, ) -> Result<(), ContractError> {
        if let Some(some_addr) = self.some_addr {
            CONFIG.save(deps.storage, &ServiceConfig { some_addr })?;
        }
        Ok(())
    }
}
