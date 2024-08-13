use std::collections::BTreeMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps, DepsMut, Uint128};
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

#[cw_serde]
pub enum ActionsMsgs {
    Split {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    GetAdmin {},
    #[returns(ServiceConfig)]
    GetServiceConfig {},
}

#[cw_serde]
#[derive(OptionalStruct)]
pub struct ServiceConfig {
    /// Address we pull funds from
    output_addr: String,
    splits: Splits,
}

impl ServiceConfig {
    pub fn validate(&self, deps: Deps) -> Result<Config, ContractError> {
        // TODO: Verify splits are valid
        Ok(Config {
            output_addr: deps.api.addr_validate(&self.output_addr)?,
            splits: self.splits.clone(),
        })
    }
}

impl OptionalServiceConfig {
    /// TODO: (2) Implement the update_config function to update config
    /// Field list matches the fields in the ServiceConfig struct, but all of them are optional
    /// if a field is Some, it means we want to update it.
    /// You can return here anything the service needs
    pub fn update_config(self, deps: DepsMut) -> Result<(), ContractError> {
        let mut config = CONFIG.load(deps.storage)?;

        if let Some(output_addr) = self.output_addr {
            config.output_addr = deps.api.addr_validate(&output_addr)?;
        }

        if let Some(splits) = self.splits {
            // TODO: Verify splits are valid
            config.splits = splits;
        }

        CONFIG.save(deps.storage, &config)?;
        Ok(())
    }
}

#[cw_serde]
pub struct Config {
    pub output_addr: Addr,
    pub splits: Splits,
}

type Splits = BTreeMap<String, BTreeMap<String, Uint128>>;
