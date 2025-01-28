pub mod account;
pub mod bridge;
pub mod config;
pub mod connectors;
pub mod domain;
pub mod error;
pub mod helpers;
pub mod library;
pub mod macros;
pub mod mock_api;
pub mod program_config;
pub mod program_config_builder;
pub mod program_migration;
pub mod program_update;
pub mod tests;

use connectors::Connectors;
use error::ManagerResult;
use program_config::ProgramConfig;
use program_migration::{MigrateResponse, ProgramConfigMigrate};
use program_update::{ProgramConfigUpdate, UpdateResponse};

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

pub async fn update_program(
    mut program_config: ProgramConfigUpdate,
) -> ManagerResult<UpdateResponse> {
    let connectors = Connectors::default();

    program_config.update(&connectors).await
}

pub async fn migrate_program(
    mut program_config: ProgramConfigMigrate,
) -> ManagerResult<MigrateResponse> {
    let connectors = Connectors::default();

    program_config.migrate(&connectors).await
}
