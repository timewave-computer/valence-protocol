use valence_authorization_utils::authorization::AuthorizationInfo;
use valence_library_utils::{GetId, LibraryAccountType};

use crate::{
    account::AccountInfo,
    library::LibraryInfo,
    workflow_config::{Link, WorkflowConfig},
};

#[derive(Default)]
pub struct WorkflowConfigBuilder {
    account_id: u64,
    library_id: u64,
    link_id: u64,
    workflow_config: WorkflowConfig,
}

impl WorkflowConfigBuilder {
    pub fn new(owner: String) -> Self {
        WorkflowConfigBuilder {
            workflow_config: WorkflowConfig {
                owner,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn set_owner(&mut self, owner: String) {
        self.workflow_config.owner = owner;
    }

    pub fn add_account(&mut self, info: AccountInfo) -> LibraryAccountType {
        let id = self.account_id;
        self.account_id += 1;

        if self.workflow_config.accounts.insert(id, info).is_some() {
            panic!("Account with id {} already exists", id);
        }

        LibraryAccountType::AccountId(id)
    }

    pub fn add_library(&mut self, info: LibraryInfo) -> LibraryAccountType {
        let id = self.library_id;
        self.library_id += 1;

        if self.workflow_config.libraries.insert(id, info).is_some() {
            panic!("Library with id {} already exists", id);
        }

        LibraryAccountType::LibraryId(id)
    }

    pub fn add_link(
        &mut self,
        library_id: &impl GetId,
        inputs: Vec<&impl GetId>,
        outputs: Vec<&impl GetId>,
    ) {
        let id = self.link_id;
        self.link_id += 1;

        self.workflow_config.links.insert(
            id,
            Link {
                input_accounts_id: inputs.into_iter().map(|id| id.get_id()).collect(),
                output_accounts_id: outputs.into_iter().map(|id| id.get_id()).collect(),
                library_id: library_id.get_id(),
            },
        );
    }

    pub fn add_authorization(&mut self, authorization: AuthorizationInfo) {
        self.workflow_config.authorizations.push(authorization);
    }

    pub fn build(self) -> WorkflowConfig {
        self.workflow_config
    }
}
