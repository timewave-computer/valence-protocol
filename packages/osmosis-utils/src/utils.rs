use cosmwasm_schema::cw_serde;

#[cw_serde]
pub enum OsmosisPoolType {
    // gamm, xyk, defined in x/gamm
    Balancer,
    /// cfmm stableswap curve, defined in x/gamm
    StableSwap,
    // CL pool, defined in x/concentrated-liquidity
    Concentrated,
}
