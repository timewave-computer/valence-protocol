pub mod account;
pub mod bridge;
pub mod config;
pub mod connectors;
pub mod domain;
pub mod error;
pub mod helpers;
pub mod macros;
pub mod service;
pub mod tests;
pub mod workflow_config;
pub mod workflow_update;

use connectors::Connectors;
use error::ManagerResult;
use workflow_config::WorkflowConfig;

// Main chain name
const NEUTRON_CHAIN: &str = "neutron";
// // Main domain
// const MAIN_DOMAIN: Domain = Domain::CosmosCosmwasm(MAIN_CHAIN);
// // Neutron domain
// const NEUTRON_DOMAIN: Domain = Domain::CosmosCosmwasm("neutron");

pub async fn init_workflow(workflow_config: &mut WorkflowConfig) -> ManagerResult<()> {
    let connectors = Connectors::default();

    // TODO: We probably want to register the error we got, with the config in question so we can know when it failed and why
    workflow_config.init(&connectors).await
}

// pub fn update_workflow(mut workflow_config: WorkflowConfig, mut old_workflow_config: WorkflowConfig) {
//     let ctx = None;

//     workflow_config.update(ctx);

//     println!("{:#?}", workflow_config);
// }
