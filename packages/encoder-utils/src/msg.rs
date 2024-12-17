use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(bool)]
    IsValidEncodingInfo { library: String, function: String },
    #[returns(Binary)]
    Encode {
        library: String,
        function: String,
        msg: Binary,
    },
}
