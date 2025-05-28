use cosmwasm_std::Uint128;
use serde::{Deserialize, Serialize};
use valence_e2e::utils::worker::ValenceWorkerTomlSerde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeutronStrategyConfig {
    /// grpc node url
    pub grpc_url: String,
    /// grpc node port
    pub grpc_port: String,
    /// neutron chain id
    pub chain_id: String,
    /// strategist mnemonic
    pub mnemonic: String,
    /// total amount of untrn required to initiate an ibc transfer from neutron
    pub min_ibc_fee: Uint128,

    /// Mars protocol wbtc contract
    pub mars_pool: String,
    /// Supervaults vault address
    pub supervault: String,

    /// authorizations module
    pub authorizations: String,
    /// processor coupled with the authorizations
    pub processor: String,

    /// all denoms relevant to the neutron-side of strategy
    pub denoms: NeutronDenoms,
    /// all accounts relevant to the neutron-side of strategy
    pub accounts: NeutronAccounts,
    /// all libraries relevant to the neutron-side of strategy
    pub libraries: NeutronLibraries,
}

// default impl serde trait to enable toml config file parsing
impl ValenceWorkerTomlSerde for NeutronStrategyConfig {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeutronDenoms {
    /// WBTC (ibc'd in from Cosmos hub)
    pub wbtc: String,
    /// gas fee denom
    pub ntrn: String,
    /// supervaults LP share denom
    pub supervault_lp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeutronAccounts {
    /// deposit account where funds will arrive from cosmos hub
    pub deposit: String,
    /// input account from which funds will be deposited into Mars
    pub mars: String,
    /// input account from which funds will be deposited into Supervault
    pub supervault: String,
    /// settlement account to settle user withdraw obligations
    pub settlement: String,
    /// interchain account to route funds from cosmos hub to neutron
    pub gaia_ica: IcaAccount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcaAccount {
    /// Valence Interchain Account contract addr
    pub library_account: String,
    /// ICA opened by the library account
    pub remote_addr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeutronLibraries {
    /// clearing queue library (settlement engine)
    pub clearing: String,
    /// library to interact with Mars lending protocol
    pub mars_lending: String,
    /// library to perform deposits into Supervaults
    pub supervaults_depositor: String,
    /// Valence forwarder which routes funds from the deposit
    /// account to Mars or Supervaults depositor, depending on
    /// the phase
    pub deposit_forwarder: String,
    /// ICA ibc transfer library
    pub ica_ibc_transfer: String,
}
