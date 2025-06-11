use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Addr, Decimal, Deps, DepsMut, Uint128, Uint64};
use cw_ownable::cw_ownable_query;
use valence_library_utils::{
    error::LibraryError, liquidity_utils::DecimalRange, msg::LibraryConfigValidation,
    LibraryAccountType,
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
    /// latest registered obligation id
    pub latest_id: Uint64,
    /// supervaults address
    pub supervault_addr: Addr,
    /// supervaults provider addr
    pub supervault_sender: Addr,
    /// settlement ratio (w.r.t. Mars position)
    pub settlement_ratio: Decimal,
}

#[cw_serde]
#[derive(ValenceLibraryInterface)]
pub struct LibraryConfig {
    /// settlement input account which we tap into in order
    /// to settle the obligations
    pub settlement_acc_addr: LibraryAccountType,
    /// obligation base denom
    pub denom: String,
    /// latest registered obligation id.
    /// if `None`, defaults to 0
    pub latest_id: Option<Uint64>,
    /// supervaults address
    pub supervault_addr: String,
    /// supervaults provider addr
    pub supervaults_sender: String,
    /// settlement ratio (w.r.t. Mars position)
    pub settlement_ratio: Decimal,
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
    pub fn new(
        settlement_acc_addr: impl Into<LibraryAccountType>,
        denom: String,
        latest_id: Option<Uint64>,
        supervault_addr: String,
        supervaults_sender: String,
        settlement_ratio: Decimal,
    ) -> Self {
        LibraryConfig {
            settlement_acc_addr: settlement_acc_addr.into(),
            denom,
            latest_id,
            supervault_addr,
            supervaults_sender,
            settlement_ratio,
        }
    }

    fn do_validate(
        &self,
        api: &dyn cosmwasm_std::Api,
    ) -> Result<(Addr, String, Uint64, Addr, Addr, Decimal), LibraryError> {
        // validate the input account
        let settlement_acc_addr = self.settlement_acc_addr.to_addr(api)?;

        ensure!(
            !self.denom.is_empty(),
            LibraryError::ConfigurationError("input denom cannot be empty".to_string())
        );

        // if id was not specified, we default to 0
        let id = self.latest_id.unwrap_or_default();

        let validated_supervault_addr = api.addr_validate(&self.supervault_addr)?;
        let validated_supervaults_sender = api.addr_validate(&self.supervaults_sender)?;

        // validate that the settlement ratio is between 0 and 1
        DecimalRange::new(Decimal::zero(), Decimal::one()).contains(self.settlement_ratio)?;

        Ok((
            settlement_acc_addr,
            self.denom.to_string(),
            id,
            validated_supervault_addr,
            validated_supervaults_sender,
            self.settlement_ratio,
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
        let (
            settlement_acc_addr,
            denom,
            latest_id,
            supervault_addr,
            supervault_sender,
            settlement_ratio,
        ) = self.do_validate(deps.api)?;
        Ok(Config {
            settlement_acc_addr,
            denom,
            latest_id,
            supervault_addr,
            supervault_sender,
            settlement_ratio,
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

        if let Some(addr) = self.supervault_addr {
            config.supervault_addr = deps.api.addr_validate(&addr)?;
        }

        if let Some(addr) = self.supervaults_sender {
            config.supervault_sender = deps.api.addr_validate(&addr)?;
        }

        if let Some(ratio) = self.settlement_ratio {
            // validate that the settlement ratio is between 0 and 1
            DecimalRange::new(Decimal::zero(), Decimal::one()).contains(ratio)?;
            config.settlement_ratio = ratio;
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
    #[returns(crate::state::ObligationStatus)]
    ObligationStatus { id: u64 },
}

#[cw_serde]
pub struct QueueInfoResponse {
    /// total number of obligations in the queue
    pub len: u64,
}

#[cw_serde]
pub struct ObligationsResponse {
    pub obligations: Vec<WithdrawalObligation>,
}
