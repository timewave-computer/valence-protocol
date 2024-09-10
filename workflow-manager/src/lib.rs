pub mod account;
pub mod bridge;
pub mod config;
pub mod connectors;
pub mod domain;
pub mod error;
pub mod service;
pub mod tests;
pub mod workflow_config;

use domain::Domain;
use workflow_config::WorkflowConfig;

// Main chain name
const MAIN_CHAIN: &str = "neutron";
// Main domain
const MAIN_DOMAIN: Domain = Domain::CosmosCosmwasm(MAIN_CHAIN);

pub async fn init_workflow(mut workflow_config: WorkflowConfig) {
    workflow_config.init().await.unwrap();

    println!("{:#?}", workflow_config);
    // println!("{:#?}", ctx.get_domain_infos_len().await);
}

// pub fn update_workflow(mut workflow_config: WorkflowConfig, mut old_workflow_config: WorkflowConfig) {
//     let ctx = None;

//     workflow_config.update(ctx);

//     println!("{:#?}", workflow_config);
// }
