use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, CustomQuery, Deps, DepsMut, QuerierWrapper, Uint128, Uint256, Uint64};
use cw_ownable::cw_ownable_query;
use valence_library_utils::{
    error::LibraryError, msg::LibraryConfigValidation, LibraryAccountType,
};
use valence_macros::{valence_library_query, ValenceLibraryInterface};

#[cw_serde]
pub enum FunctionMsgs {
    /// If quote amount is provided, it will override the quote amount in the config.
    Transfer { quote_amount: Option<Uint256> },
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
    pub denom: UncheckedUnionDenomConfig,
    pub amount: TransferAmount,
    // Information about the asset to be transferred.
    pub input_asset_name: String,
    pub input_asset_symbol: String,
    pub input_asset_decimals: u8,
    pub input_asset_token_path: Uint256,
    // Information about the asset to be received.
    pub quote_token: String,
    pub quote_amount: Uint256,
    // Information about the remote chain.
    pub channel_id: String,
    pub transfer_timeout: Option<Uint64>, // If not provided, a default 3 days will be used (259200 seconds).
    // Information about the protocol
    pub protocol_version: u8,
}

#[cw_serde]
pub enum UncheckedUnionDenomConfig {
    /// A native (bank module) asset.
    Native(String),
    /// A cw20 asset along with the token minter address that needs to be approved for spending during transfers.
    Cw20(UncheckedUnionCw20Config),
}

#[cw_serde]
pub struct UncheckedUnionCw20Config {
    pub token: String,
    pub minter: String,
}

impl UncheckedUnionDenomConfig {
    pub fn into_checked(self, deps: Deps) -> StdResult<CheckedUnionDenomConfig> {
        match self {
            Self::Native(denom) => Ok(CheckedUnionDenomConfig::Native(denom)),
            Self::Cw20(unchecked_config) => {
                let addr_token = deps.api.addr_validate(&unchecked_config.token)?;
                let addr_minter = deps.api.addr_validate(&unchecked_config.minter)?;
                let _info: cw20::TokenInfoResponse = deps
                    .querier
                    .query_wasm_smart(addr_token.clone(), &cw20::Cw20QueryMsg::TokenInfo {})?;
                Ok(CheckedUnionDenomConfig::Cw20(CheckedUnionCw20Config {
                    token: addr_token,
                    minter: addr_minter,
                }))
            }
        }
    }
}

#[cw_serde]
pub enum CheckedUnionDenomConfig {
    /// A native (bank module) asset.
    Native(String),
    /// A cw20 asset along with the token minter address that needs to be approved for spending during transfers.
    Cw20(CheckedUnionCw20Config),
}

impl CheckedUnionDenomConfig {
    pub fn to_string(&self) -> String {
        match self {
            Self::Native(denom) => denom.clone(),
            Self::Cw20(config) => config.token.to_string(),
        }
    }

    pub fn query_balance<C: CustomQuery>(
        &self,
        querier: &QuerierWrapper<C>,
        who: &Addr,
    ) -> StdResult<Uint128> {
        match self {
            Self::Native(denom) => Ok(querier.query_balance(who, denom)?.amount),
            Self::Cw20(config) => {
                let balance: cw20::BalanceResponse = querier.query_wasm_smart(
                    config.token.clone(),
                    &cw20::Cw20QueryMsg::Balance {
                        address: who.to_string(),
                    },
                )?;
                Ok(balance.balance)
            }
        }
    }
}

#[cw_serde]
pub struct CheckedUnionCw20Config {
    pub token: Addr,
    pub minter: Addr,
}

#[cw_serde]
pub enum TransferAmount {
    FullAmount,
    FixedAmount(Uint128),
}

impl LibraryConfig {
    pub fn new(
        input_addr: LibraryAccountType,
        output_addr: LibraryAccountType,
        denom: UncheckedUnionDenomConfig,
        amount: TransferAmount,
        input_asset_name: String,
        input_asset_symbol: String,
        input_asset_decimals: u8,
        input_asset_token_path: Uint256,
        quote_token: String,
        quote_amount: Uint256,
        channel_id: String,
        transfer_timeout: Option<Uint64>,
        protocol_version: u8,
    ) -> Self {
        Self {
            input_addr,
            output_addr,
            denom,
            amount,
            input_asset_name,
            input_asset_symbol,
            input_asset_decimals,
            input_asset_token_path,
            quote_token,
            quote_amount,
            channel_id,
            transfer_timeout,
            protocol_version,
        }
    }

    fn do_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<Addr, LibraryError> {
        let input_addr = self.input_addr.to_addr(api)?;

        match self.amount {
            TransferAmount::FullAmount => {}
            TransferAmount::FixedAmount(amount) => {
                if amount.is_zero() {
                    return Err(LibraryError::ConfigurationError(
                        "Invalid Union transfer config: amount cannot be zero.".to_string(),
                    ));
                }
            }
        }

        if self.channel_id.is_empty() {
            return Err(LibraryError::ConfigurationError(
                "Invalid Union transfer config: channel_id cannot be empty.".to_string(),
            ));
        }

        if let Some(timeout) = self.transfer_timeout {
            if timeout.is_zero() {
                return Err(LibraryError::ConfigurationError(
                    "Invalid Union transfer config: transfer_timeout cannot be zero.".to_string(),
                ));
            }
        }

        Ok(input_addr)
    }
}

impl LibraryConfigValidation<Config> for LibraryConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), LibraryError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, LibraryError> {
        let input_addr = self.do_validate(deps.api)?;
        Ok(Config {
            input_addr,
            // Can't validate output address as it's on another chain
            output_addr: self.output_addr.to_string()?,
            denom: self
                .denom
                .clone()
                .into_checked(deps)
                .map_err(|err| LibraryError::ConfigurationError(err.to_string()))?,
            amount: self.amount.clone(),
            input_asset_name: self.input_asset_name.clone(),
            input_asset_symbol: self.input_asset_symbol.clone(),
            input_asset_decimals: self.input_asset_decimals,
            input_asset_token_path: self.input_asset_token_path,
            quote_token: self.quote_token.clone(),
            quote_amount: self.quote_amount,
            channel_id: self.channel_id.clone(),
            transfer_timeout: self.transfer_timeout,
            protocol_version: self.protocol_version,
        })
    }
}

impl LibraryConfigUpdate {
    pub fn update_config<T>(self, deps: DepsMut<T>) -> Result<(), LibraryError>
    where
        T: CustomQuery,
    {
        let mut config: Config = valence_library_base::load_config(deps.storage)?;

        if let Some(input_addr) = self.input_addr {
            config.input_addr = input_addr.to_addr(deps.api)?;
        }

        if let Some(output_addr) = self.output_addr {
            config.output_addr = output_addr.to_string()?;
        }

        if let Some(denom) = self.denom {
            config.denom = denom
                .clone()
                .into_checked(deps.as_ref().into_empty())
                .map_err(|err| LibraryError::ConfigurationError(err.to_string()))?;
        }

        if let Some(amount) = self.amount {
            if let TransferAmount::FixedAmount(amount) = &amount {
                if amount.is_zero() {
                    return Err(LibraryError::ConfigurationError(
                        "Invalid Union transfer config: amount cannot be zero.".to_string(),
                    ));
                }
            }
            config.amount = amount;
        }

        if let Some(input_asset_name) = self.input_asset_name {
            config.input_asset_name = input_asset_name;
        }

        if let Some(input_asset_symbol) = self.input_asset_symbol {
            config.input_asset_symbol = input_asset_symbol;
        }

        if let Some(input_asset_decimals) = self.input_asset_decimals {
            config.input_asset_decimals = input_asset_decimals;
        }

        if let Some(input_asset_token_path) = self.input_asset_token_path {
            config.input_asset_token_path = input_asset_token_path;
        }

        if let Some(quote_token) = self.quote_token {
            config.quote_token = quote_token;
        }

        if let Some(quote_amount) = self.quote_amount {
            config.quote_amount = quote_amount;
        }

        if let Some(channel_id) = self.channel_id {
            config.channel_id = channel_id;
        }

        if let OptionUpdate::Set(transfer_timeout) = self.transfer_timeout {
            if let Some(timeout) = transfer_timeout {
                if timeout.is_zero() {
                    return Err(LibraryError::ConfigurationError(
                        "Invalid Union transfer config: transfer_timeout cannot be zero."
                            .to_string(),
                    ));
                }
            }
            config.transfer_timeout = transfer_timeout;
        }

        if let Some(protocol_version) = self.protocol_version {
            config.protocol_version = protocol_version;
        }

        valence_library_base::save_config(deps.storage, &config)?;

        Ok(())
    }
}

#[cw_serde]
pub struct Config {
    pub input_addr: Addr,
    pub output_addr: String,
    pub denom: CheckedUnionDenomConfig,
    pub amount: TransferAmount,
    pub input_asset_name: String,
    pub input_asset_symbol: String,
    pub input_asset_decimals: u8,
    pub input_asset_token_path: Uint256,
    pub quote_token: String,
    pub quote_amount: Uint256,
    pub channel_id: String,
    pub transfer_timeout: Option<Uint64>,
    pub protocol_version: u8,
}

impl Config {
    pub fn new(
        input_addr: Addr,
        output_addr: String,
        denom: CheckedUnionDenomConfig,
        amount: TransferAmount,
        input_asset_name: String,
        input_asset_symbol: String,
        input_asset_decimals: u8,
        input_asset_token_path: Uint256,
        quote_token: String,
        quote_amount: Uint256,
        channel_id: String,
        transfer_timeout: Option<Uint64>,
        protocol_version: u8,
    ) -> Self {
        Config {
            input_addr,
            output_addr,
            denom,
            amount,
            input_asset_name,
            input_asset_symbol,
            input_asset_decimals,
            input_asset_token_path,
            quote_token,
            quote_amount,
            channel_id,
            transfer_timeout,
            protocol_version,
        }
    }
}
