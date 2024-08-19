use workflow_config::WorkflowConfig;

pub mod tests;
pub mod domain;
pub mod account;
pub mod service;
pub mod workflow_config;

pub fn init_workflow(mut workflow_config: WorkflowConfig) {
    workflow_config.init();

    println!("{:#?}", workflow_config);
}