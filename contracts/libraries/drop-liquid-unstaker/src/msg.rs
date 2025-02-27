use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps, DepsMut};
use cw_ownable::cw_ownable_query;
use valence_library_utils::LibraryAccountType;
use valence_library_utils::{error::LibraryError, msg::LibraryConfigValidation};
use valence_macros::{valence_library_query, ValenceLibraryInterface};

#[cw_serde]
pub enum FunctionMsgs {
    Unstake {},
    Withdraw { token_id: String }, // Token ID of the NFT Voucher to withdraw
}

#[valence_library_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
/// Enum representing the different query messages that can be sent.
pub enum QueryMsg {}

#[cw_serde]
#[derive(ValenceLibraryInterface)]
pub struct LibraryConfig {
    pub input_addr: LibraryAccountType,
    pub output_addr: LibraryAccountType,
    // Address of the liquid unstaker contract (drop core contract)
    pub liquid_unstaker_addr: String,
    // Address of the withdrawal_manager_addr (drop withdrawal manager)
    pub withdrawal_manager_addr: String,
    // Address of the voucher NFT contract that we get after unstaking and we use for the withdraw
    pub voucher_addr: String,
    // Denom of the asset we are going to unstake
    pub denom: String,
}

impl LibraryConfig {
    pub fn new(
        input_addr: impl Into<LibraryAccountType>,
        output_addr: impl Into<LibraryAccountType>,
        liquid_unstaker_addr: String,
        withdrawal_manager_addr: String,
        voucher_addr: String,
        denom: String,
    ) -> Self {
        LibraryConfig {
            input_addr: input_addr.into(),
            output_addr: output_addr.into(),
            liquid_unstaker_addr,
            withdrawal_manager_addr,
            voucher_addr,
            denom,
        }
    }

    fn do_validate(
        &self,
        api: &dyn cosmwasm_std::Api,
    ) -> Result<(Addr, Addr, Addr, Addr, Addr), LibraryError> {
        let input_addr = self.input_addr.to_addr(api)?;
        let output_addr = self.output_addr.to_addr(api)?;
        let liquid_unstaker_addr = api.addr_validate(&self.liquid_unstaker_addr)?;
        let voucher_addr = api.addr_validate(&self.voucher_addr)?;
        let withdrawal_manager_addr = api.addr_validate(&self.withdrawal_manager_addr)?;

        Ok((
            input_addr,
            output_addr,
            liquid_unstaker_addr,
            voucher_addr,
            withdrawal_manager_addr,
        ))
    }
}

impl LibraryConfigValidation<Config> for LibraryConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), LibraryError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, LibraryError> {
        let (input_addr, output_addr, liquid_unstaker_addr, voucher_addr, withdrawal_manager_addr) =
            self.do_validate(deps.api)?;

        Ok(Config {
            input_addr,
            output_addr,
            liquid_unstaker_addr,
            voucher_addr,
            withdrawal_manager_addr,
            denom: self.denom.clone(),
        })
    }
}

impl LibraryConfigUpdate {
    pub fn update_config(self, deps: DepsMut) -> Result<(), LibraryError> {
        let mut config: Config = valence_library_base::load_config(deps.storage)?;

        // First update input_addr (if needed)
        if let Some(input_addr) = self.input_addr {
            config.input_addr = input_addr.to_addr(deps.api)?;
        }

        // Next update output_addr (if needed)
        if let Some(output_addr) = self.output_addr {
            config.output_addr = output_addr.to_addr(deps.api)?;
        }

        // Next update liquid_unstaker_addr (if needed)
        if let Some(liquid_unstaker_addr) = self.liquid_unstaker_addr {
            config.liquid_unstaker_addr = deps.api.addr_validate(&liquid_unstaker_addr)?;
        }

        // Next update withdrawal_manager_addr (if needed)
        if let Some(withdrawal_manager_addr) = self.withdrawal_manager_addr {
            config.withdrawal_manager_addr = deps.api.addr_validate(&withdrawal_manager_addr)?;
        }

        // Next update denom (if needed)
        if let Some(denom) = self.denom {
            config.denom = denom;
        }

        valence_library_base::save_config(deps.storage, &config)?;
        Ok(())
    }
}

#[cw_serde]
pub struct Config {
    pub input_addr: Addr,
    pub output_addr: Addr,
    pub liquid_unstaker_addr: Addr,
    pub voucher_addr: Addr,
    pub withdrawal_manager_addr: Addr,
    pub denom: String,
}

impl Config {
    pub fn new(
        input_addr: Addr,
        output_addr: Addr,
        liquid_unstaker_addr: Addr,
        voucher_addr: Addr,
        withdrawal_manager_addr: Addr,
        denom: String,
    ) -> Self {
        Config {
            input_addr,
            output_addr,
            liquid_unstaker_addr,
            voucher_addr,
            withdrawal_manager_addr,
            denom,
        }
    }
}
