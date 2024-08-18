pub mod tests;
pub mod types;
pub mod domain;

use types::WorkflowConfig;

pub fn init_workflow(mut workflow_config: WorkflowConfig) {
    workflow_config.init();

    println!("{:#?}", workflow_config);
}