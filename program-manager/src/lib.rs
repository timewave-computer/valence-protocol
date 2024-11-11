pub mod account;
pub mod bridge;
pub mod config;
pub mod connectors;
pub mod domain;
pub mod error;
pub mod helpers;
pub mod macros;
pub mod program_config;
pub mod program_config_builder;
pub mod service;
pub mod tests;

use connectors::Connectors;
use error::ManagerResult;
use program_config::ProgramConfig;

// Main chain name
const NEUTRON_CHAIN: &str = "neutron";
// // Main domain
// const MAIN_DOMAIN: Domain = Domain::CosmosCosmwasm(MAIN_CHAIN);
// // Neutron domain
// const NEUTRON_DOMAIN: Domain = Domain::CosmosCosmwasm("neutron");

pub async fn init_program(program_config: &mut ProgramConfig) -> ManagerResult<()> {
    let connectors = Connectors::default();

    // TODO: We probably want to register the error we got, with the config in question so we can know when it failed and why
    program_config.init(&connectors).await
}

// pub fn update_program(mut program_config: ProgramConfig, mut old_program_config: ProgramConfig) {
//     let ctx = None;

//     program_config.update(ctx);

//     println!("{:#?}", program_config);
// }
