use std::collections::HashSet;

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
    /// latest registered obligation id.
    /// `None` indicates that no obligations have been
    /// registered yet (and expects id=0 for next).
    pub latest_id: Option<Uint64>,
    /// mars settlement ratio
    pub mars_settlement_ratio: Decimal,
    /// supervaults settlement information
    pub supervaults_settlement_info: Vec<ValidatedSupervaultSettlementInfo>,
}

#[cw_serde]
pub struct SupervaultSettlementInfo {
    /// supervaults address
    pub supervault_addr: String,
    /// supervault provider address
    pub supervault_sender: String,
    /// settlement ratio
    pub settlement_ratio: Decimal,
}

#[cw_serde]
pub struct ValidatedSupervaultSettlementInfo {
    /// supervaults address
    pub supervault_addr: Addr,
    /// supervault provider address
    pub supervault_sender: Addr,
    /// settlement ratio
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
    pub latest_id: Option<Uint64>,
    /// mars settlement ratio
    pub mars_settlement_ratio: Decimal,
    /// supervaults settlement information
    pub supervaults_settlement_info: Vec<SupervaultSettlementInfo>,
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
        mars_settlement_ratio: Decimal,
        supervaults_settlement_info: Vec<SupervaultSettlementInfo>,
    ) -> Self {
        LibraryConfig {
            settlement_acc_addr: settlement_acc_addr.into(),
            denom,
            latest_id,
            mars_settlement_ratio,
            supervaults_settlement_info,
        }
    }

    #[allow(clippy::type_complexity)]
    fn do_validate(
        &self,
        api: &dyn cosmwasm_std::Api,
    ) -> Result<
        (
            Addr,
            String,
            Option<Uint64>,
            Decimal,
            Vec<ValidatedSupervaultSettlementInfo>,
        ),
        LibraryError,
    > {
        // validate the input account
        let settlement_acc_addr = self.settlement_acc_addr.to_addr(api)?;

        ensure!(
            !self.denom.is_empty(),
            LibraryError::ConfigurationError("input denom cannot be empty".to_string())
        );

        // validate the mars settlement ratio
        DecimalRange::new(Decimal::zero(), Decimal::one()).contains(self.mars_settlement_ratio)?;

        // check that the supervaults settlement information is not empty
        ensure!(
            !self.supervaults_settlement_info.is_empty(),
            LibraryError::ConfigurationError(
                "supervaults settlement information cannot be empty".to_string()
            )
        );

        // validate supervaults settlement information
        let supervaults_info =
            validate_supervaults_settlement_info(&self.supervaults_settlement_info, api)?;

        Ok((
            settlement_acc_addr,
            self.denom.clone(),
            self.latest_id,
            self.mars_settlement_ratio,
            supervaults_info,
        ))
    }
}

/// validate supervaults settlement information
/// 1. Addresses are valid
/// 2. Settlement ratios are between 0 and 1
/// 3. Settlement ratios sum to 1
/// 4. No duplicate supervault addresses
fn validate_supervaults_settlement_info(
    supervaults_settlement_info: &[SupervaultSettlementInfo],
    api: &dyn cosmwasm_std::Api,
) -> Result<Vec<ValidatedSupervaultSettlementInfo>, LibraryError> {
    let mut total_supervaults_ratio = Decimal::zero();
    let mut supervaults_addrs = vec![];
    let mut check_duplicated = HashSet::new();

    for info in supervaults_settlement_info {
        let supervault_addr = api.addr_validate(&info.supervault_addr)?;
        let supervault_sender = api.addr_validate(&info.supervault_sender)?;
        DecimalRange::new(Decimal::zero(), Decimal::one()).contains(info.settlement_ratio)?;
        ensure!(
            check_duplicated.insert(supervault_addr.to_string()),
            LibraryError::ConfigurationError(format!(
                "Duplicate supervault address: {}",
                supervault_addr
            ))
        );
        total_supervaults_ratio += info.settlement_ratio;
        supervaults_addrs.push(ValidatedSupervaultSettlementInfo {
            supervault_addr,
            supervault_sender,
            settlement_ratio: info.settlement_ratio,
        });
    }

    ensure!(
        total_supervaults_ratio == Decimal::one(),
        LibraryError::ConfigurationError(format!(
            "Total supervaults settlement ratio must be 1, got {}",
            total_supervaults_ratio
        ))
    );

    Ok(supervaults_addrs)
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
            mars_settlement_ratio,
            supervaults_settlement_info,
        ) = self.do_validate(deps.api)?;

        Ok(Config {
            settlement_acc_addr,
            denom,
            latest_id,
            mars_settlement_ratio,
            supervaults_settlement_info,
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

        if let OptionUpdate::Set(latest_id) = self.latest_id {
            config.latest_id = latest_id;
        }

        if let Some(mars_settlement_ratio) = self.mars_settlement_ratio {
            DecimalRange::new(Decimal::zero(), Decimal::one()).contains(mars_settlement_ratio)?;
            config.mars_settlement_ratio = mars_settlement_ratio;
        }

        if let Some(supervaults_settlement_info) = self.supervaults_settlement_info {
            ensure!(
                !supervaults_settlement_info.is_empty(),
                LibraryError::ConfigurationError(
                    "supervaults settlement information cannot be empty".to_string()
                )
            );

            // validate supervaults settlement information
            let supervaults_info =
                validate_supervaults_settlement_info(&supervaults_settlement_info, deps.api)?;
            config.supervaults_settlement_info = supervaults_info;
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
