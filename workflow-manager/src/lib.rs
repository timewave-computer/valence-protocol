pub mod account;
pub mod config;
pub mod context;
pub mod domain;
pub mod service;
pub mod tests;
pub mod workflow_config;

use context::Context;
use workflow_config::WorkflowConfig;

pub async fn init_workflow(mut workflow_config: WorkflowConfig) {
    let ctx = Context::default();

    workflow_config.init(ctx.get_clone()).await;

    println!("{:#?}", workflow_config);
    println!("{:#?}", ctx.get_domain_infos_len().await);
}

// pub fn update_workflow(mut workflow_config: WorkflowConfig, mut old_workflow_config: WorkflowConfig) {
//     let ctx = None;

//     workflow_config.update(ctx);

//     println!("{:#?}", workflow_config);
// }
