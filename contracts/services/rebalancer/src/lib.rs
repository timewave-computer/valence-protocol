pub mod contract;
pub mod helpers;
pub mod msg;

#[cfg(test)]
mod tests;

pub(crate) const USDC_DENOM: &str =
    "ibc/B559A80D62249C8AA07A380E2A2BEA6E5CA9A6F079C912C3A9E9B494105E4F81";
pub(crate) const NEWT_DENOM: &str = "factory/neutron1p8d89wvxyjcnawmgw72klknr3lg9gwwl6ypxda/newt";
pub(crate) const NTRN_DENOM: &str = "untrn";
pub(crate) const ATOM_DENOM: &str =
    "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9";
