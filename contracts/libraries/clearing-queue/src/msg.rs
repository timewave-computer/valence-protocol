use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Addr, Deps, DepsMut, Uint128, Uint64};
use cw_ownable::cw_ownable_query;
use valence_library_utils::{
    error::LibraryError, msg::LibraryConfigValidation, LibraryAccountType,
};
use valence_macros::{valence_library_query, ValenceLibraryInterface};

use crate::state::WithdrawalObligation;

#[cw_serde]
/// Validated library configuration
pub struct Config {
    /// settlement input account which we tap into in order
    /// to settle the obligations
    pub settlement_acc_addr: Addr,
    /// obligation base denom
    pub denom: String,
}

#[cw_serde]
#[derive(ValenceLibraryInterface)]
pub struct LibraryConfig {
    /// settlement input account which we tap into in order
    /// to settle the obligations
    pub settlement_acc_addr: LibraryAccountType,
    /// obligation base denom
    pub denom: String,
}

#[cw_serde]
pub enum FunctionMsgs {
    /// validates and enqueues a new withdrawal obligation
    RegisterObligation {
        /// where the payout is to be routed
        recipient: String,
        /// amount of the config denom owed to the recipient
        payout_amount: Uint128,
        /// some unique identifier for the request
        id: Uint64,
    },
    /// settles the oldest withdrawal obligation
    SettleNextObligation {},
}

impl LibraryConfig {
    pub fn new(settlement_acc_addr: impl Into<LibraryAccountType>, denom: String) -> Self {
        LibraryConfig {
            settlement_acc_addr: settlement_acc_addr.into(),
            denom,
        }
    }

    fn do_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(Addr, String), LibraryError> {
        // validate the input account
        let settlement_acc_addr = self.settlement_acc_addr.to_addr(api)?;

        ensure!(
            !self.denom.is_empty(),
            LibraryError::ConfigurationError("input denom cannot be empty".to_string())
        );

        Ok((settlement_acc_addr, self.denom.to_string()))
    }
}

impl LibraryConfigValidation<Config> for LibraryConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), LibraryError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, LibraryError> {
        let (settlement_acc_addr, denom) = self.do_validate(deps.api)?;
        Ok(Config {
            settlement_acc_addr,
            denom,
        })
    }
}

impl LibraryConfigUpdate {
    pub fn update_config(self, deps: DepsMut) -> Result<(), LibraryError> {
        let mut config: Config = valence_library_base::load_config(deps.storage)?;

        if let Some(settlement_acc_addr) = self.settlement_acc_addr {
            config.settlement_acc_addr = settlement_acc_addr.to_addr(deps.api)?;
        }

        if let Some(denom) = self.denom {
            ensure!(
                !denom.is_empty(),
                LibraryError::ConfigurationError("clearing denom cannot be empty".to_string())
            );
            config.denom = denom;
        }

        valence_library_base::save_config(deps.storage, &config)?;

        Ok(())
    }
}

#[valence_library_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// returns the total number of obligations in the queue
    #[returns(QueueInfoResponse)]
    QueueInfo {},
    /// returns a list of obligations in the queue starting from the given index
    #[returns(ObligationsResponse)]
    PendingObligations {
        /// starting index
        from: Option<u64>,
        /// end index
        to: Option<u64>,
    },
    /// constant time status check for a specific obligation.
    /// if status of more than one obligations will be relevant,
    /// this information can be inferred from the `Obligations` query
    /// (if obligation is in the queue then it is not yet settled).
    #[returns(ObligationStatusResponse)]
    ObligationStatus { id: u64 },
}

#[cw_serde]
pub struct QueueInfoResponse {
    /// total number of obligations in the queue
    pub len: u64,
}

#[cw_serde]
pub struct ObligationStatusResponse {
    /// boolean status of a given obligation where
    /// `false` indicates that the obligation is registered,
    /// and `true` indicates that the obligation is settled.
    pub settled: bool,
}

#[cw_serde]
pub struct ObligationsResponse {
    pub obligations: Vec<WithdrawalObligation>,
}
