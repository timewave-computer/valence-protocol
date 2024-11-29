use std::str::FromStr;

use cosmwasm_schema::cw_serde;

use crate::error::ContractError;

macro_rules! define_osmosis_types {
    ($(($variant:ident, $type:ty)),* $(,)?) => {
        #[allow(clippy::large_enum_variant)]
        #[cw_serde]
        pub enum OsmosisTypes {
            $(
                $variant($type),
            )*
        }

        impl FromStr for OsmosisTypes {
            type Err = ContractError;

            fn from_str(type_url: &str) -> Result<Self, Self::Err> {
                match type_url {
                    $(
                        <$type>::TYPE_URL => Ok(OsmosisTypes::$variant(<$type>::default())),
                    )*
                    _ => Err(ContractError::UnknownTypeUrl(type_url.to_string())),
                }
            }
        }
    };
}

define_osmosis_types! {
    (GammV1Beta1Pool, osmosis_std::types::osmosis::gamm::v1beta1::Pool),
    (GammV1Beta1ParamsResponse, osmosis_std::types::osmosis::gamm::v1beta1::ParamsResponse),
    (GammV1Beta1QueryCalcExitPoolCoinsFromSharesRequest, osmosis_std::types::osmosis::gamm::v1beta1::QueryCalcExitPoolCoinsFromSharesRequest),
    (BankV1Beta1BalanceResponse, osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceResponse)
}
