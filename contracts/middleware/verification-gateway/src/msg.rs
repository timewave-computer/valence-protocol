use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;
use cw_ownable::cw_ownable_execute;

#[cw_serde]
pub struct InstantiateMsg {
    pub domain_vk: Binary,
    pub owner: String,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    UpdateDomainVk { domain_vk: Binary },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(bool)]
    VerifyProof {
        vk: Binary,
        proof: Binary,
        inputs: Binary,
    },
    #[returns(bool)]
    VerifyDomainProof {
        domain_proof: Binary,
        domain_inputs: Binary,
    },
}
