use cosmwasm_std::Uint128;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeutronStrategyConfig {
    pub grpc_url: String,
    pub grpc_port: String,
    pub chain_id: String,
    pub mnemonic: String,
    pub mars_pool: String,
    pub denoms: NeutronDenoms,
    pub accounts: NeutronAccounts,
    pub libraries: NeutronLibraries,
    // total amount of untrn required to initiate an ibc transfer from neutron
    pub min_ibc_fee: Uint128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeutronDenoms {
    pub wbtc: String,
    pub ntrn: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeutronAccounts {
    pub deposit: String,
    pub position: String,
    pub withdraw: String,
    pub liquidation: String,
    pub noble_inbound_ica: IcaAccount,
    pub noble_outbound_ica: IcaAccount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcaAccount {
    pub library_account: String,
    pub remote_addr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeutronLibraries {
    pub neutron_ibc_transfer: String,
    pub noble_inbound_transfer: String,
    pub noble_cctp_transfer: String,
    pub astroport_lper: String,
    pub astroport_lwer: String,
    pub liquidation_forwarder: String,
    pub authorizations: String,
    pub processor: String,
}
