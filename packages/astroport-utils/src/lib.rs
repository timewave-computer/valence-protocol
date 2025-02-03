use cosmwasm_schema::cw_serde;
use cosmwasm_std::DepsMut;
use valence_library_utils::error::LibraryError;

pub mod astroport_cw20_lp_token;
pub mod astroport_native_lp_token;

#[cfg(feature = "testing")]
pub mod suite;

// Define a trait that both Asset types can implement
pub trait AssetTrait {
    fn as_coin(&self) -> Result<cosmwasm_std::Coin, LibraryError>;
}

// Implement the trait for both Asset types
impl AssetTrait for astroport_native_lp_token::Asset {
    fn as_coin(&self) -> Result<cosmwasm_std::Coin, LibraryError> {
        self.as_coin()
            .map_err(|error| LibraryError::ExecutionError(error.to_string()))
    }
}

impl AssetTrait for astroport_cw20_lp_token::Asset {
    fn as_coin(&self) -> Result<cosmwasm_std::Coin, LibraryError> {
        self.to_coin()
            .map_err(|error| LibraryError::ExecutionError(error.to_string()))
    }
}

#[cw_serde]
pub enum PoolType {
    NativeLpToken(astroport_native_lp_token::PairType),
    Cw20LpToken(astroport_cw20_lp_token::PairType),
}

pub fn query_pool(
    deps: &DepsMut,
    pool_addr: &str,
    pool_type: &PoolType,
) -> Result<Vec<Box<dyn AssetTrait>>, LibraryError> {
    match pool_type {
        PoolType::NativeLpToken(_) => {
            let assets = astroport_native_lp_token::query_pool(deps, pool_addr)?;
            Ok(assets
                .into_iter()
                .map(|asset| Box::new(asset) as Box<dyn AssetTrait>)
                .collect())
        }
        PoolType::Cw20LpToken(_) => {
            let assets = astroport_cw20_lp_token::query_pool(deps, pool_addr)?;
            Ok(assets
                .into_iter()
                .map(|asset| Box::new(asset) as Box<dyn AssetTrait>)
                .collect())
        }
    }
}

pub fn get_pool_asset_amounts(
    assets: Vec<Box<dyn AssetTrait>>,
    asset1_denom: &str,
    asset2_denom: &str,
) -> Result<(u128, u128), LibraryError> {
    let (mut asset1_balance, mut asset2_balance) = (0, 0);

    for asset in assets {
        let coin = asset
            .as_coin()
            .map_err(|error| LibraryError::ExecutionError(error.to_string()))?;

        if coin.denom == asset1_denom {
            asset1_balance = coin.amount.u128();
        } else if coin.denom == asset2_denom {
            asset2_balance = coin.amount.u128();
        }
    }

    if asset1_balance == 0 || asset2_balance == 0 {
        return Err(LibraryError::ExecutionError(
            "All pool assets must be non-zero".to_string(),
        ));
    }

    Ok((asset1_balance, asset2_balance))
}

// Implemented in the astroport crate for Decimal
pub mod decimal_checked_ops {
    use std::convert::TryInto;

    use cosmwasm_std::{Decimal, Fraction, OverflowError, Uint128, Uint256};

    // We define the helper for Decimals that is defined in the astroport crate
    pub trait DecimalCheckedOps {
        fn checked_mul_uint128(self, other: Uint128) -> Result<Uint128, OverflowError>;
    }

    impl DecimalCheckedOps for Decimal {
        fn checked_mul_uint128(self, other: Uint128) -> Result<Uint128, OverflowError> {
            if self.is_zero() || other.is_zero() {
                return Ok(Uint128::zero());
            }
            let multiply_ratio = other
                .full_mul(self.numerator())
                .checked_div(Uint256::from(self.denominator()))
                .expect("self denominator is not zero; qed");
            if multiply_ratio > Uint256::from(Uint128::MAX) {
                Err(OverflowError::new(cosmwasm_std::OverflowOperation::Mul))
            } else {
                Ok(multiply_ratio.try_into().unwrap())
            }
        }
    }
}
