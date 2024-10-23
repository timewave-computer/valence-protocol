pub mod astroport_cw20_lp_token;
pub mod astroport_native_lp_token;
#[cfg(feature = "testing")]
pub mod suite;

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
