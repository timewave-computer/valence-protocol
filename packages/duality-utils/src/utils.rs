use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct PoolConfig {
    pub lp_denom: String,
    pub pair_data: PairData,
}

#[cw_serde]
pub struct PairData {
    pub token_0: TokenData,
    pub token_1: TokenData,
}

#[cw_serde]
pub struct TokenData {
    pub denom: String,
}
