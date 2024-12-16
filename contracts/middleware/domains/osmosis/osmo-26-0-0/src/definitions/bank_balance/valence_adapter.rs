use cosmwasm_std::{coins, StdError};
use osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceResponse;
use osmosis_std::types::cosmos::base::v1beta1::Coin as ProtoCoin;
use valence_middleware_utils::{
    canonical_types::bank::balance::{ValenceBankBalance, ValenceBankBalanceAdapter},
    MiddlewareError,
};

use super::OsmosisBankBalance;

impl ValenceBankBalanceAdapter for OsmosisBankBalance {
    type External = QueryBalanceResponse;

    fn try_to_canonical(&self) -> Result<ValenceBankBalance, MiddlewareError> {
        match &self.0.balance {
            Some(coin) => {
                let amount_u128 = coin.amount.parse::<u128>()?;

                Ok(ValenceBankBalance {
                    assets: coins(amount_u128, coin.denom.to_string()),
                })
            }
            None => Err(MiddlewareError::Std(StdError::generic_err(
                "failed to find coin in QueryBalanceResponse",
            ))),
        }
    }

    fn try_from_canonical(
        canonical: ValenceBankBalance,
    ) -> Result<Self::External, MiddlewareError> {
        let balance: Option<ProtoCoin> = match canonical.assets.len() {
            0 => None,
            1 => {
                let coin = canonical.assets.into_iter().next().unwrap();
                Some(coin.into())
            }
            _ => {
                return Err(MiddlewareError::Std(StdError::generic_err(
                    "failed to convert multiple coins into QueryBalanceResponse",
                )))
            }
        };
        Ok(QueryBalanceResponse { balance })
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::coins;
    use osmosis_std::types::cosmos::{
        bank::v1beta1::QueryBalanceResponse, base::v1beta1::Coin as ProtoCoin,
    };
    use valence_middleware_utils::canonical_types::bank::balance::{
        ValenceBankBalance, ValenceBankBalanceAdapter,
    };

    use crate::definitions::bank_balance::OsmosisBankBalance;

    #[test]
    fn test_try_from_canonical() {
        let canonical = ValenceBankBalance {
            assets: coins(100, "uosmo"),
        };
        let result = OsmosisBankBalance::try_from_canonical(canonical).unwrap();
        let result_coin = result.balance.unwrap();

        assert_eq!(result_coin.amount, "100");
        assert_eq!(result_coin.denom, "uosmo");
    }

    #[test]
    fn test_try_to_canonical() {
        let osmosis_bank_balance = OsmosisBankBalance(QueryBalanceResponse {
            balance: Some(ProtoCoin {
                denom: "uosmo".to_string(),
                amount: "100".to_string(),
            }),
        });

        let result = osmosis_bank_balance.try_to_canonical().unwrap();

        assert_eq!(result.assets.len(), 1);
        assert_eq!(result.assets[0].amount.u128(), 100);
        assert_eq!(result.assets[0].denom, "uosmo");
    }
}
