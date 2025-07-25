use valence_authorization_utils::authorization::AuthorizationInfo;
use valence_library_utils::{library_account_type::GetId, LibraryAccountType};

use crate::{
    account::AccountInfo,
    library::LibraryInfo,
    program_config::{Link, ProgramConfig},
};

#[derive(Default)]
pub struct ProgramConfigBuilder {
    account_id: u64,
    library_id: u64,
    link_id: u64,
    program_config: ProgramConfig,
}

impl ProgramConfigBuilder {
    pub fn new(name: &str, owner: &str) -> Self {
        ProgramConfigBuilder {
            program_config: ProgramConfig {
                owner: owner.to_string(),
                name: name.to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn set_owner(&mut self, owner: String) {
        self.program_config.owner = owner;
    }

    pub fn add_account(&mut self, info: AccountInfo) -> LibraryAccountType {
        let id = self.account_id;
        self.account_id += 1;

        if self.program_config.accounts.insert(id, info).is_some() {
            panic!("Account with id {id} already exists");
        }

        LibraryAccountType::AccountId(id)
    }

    pub fn add_library(&mut self, info: LibraryInfo) -> LibraryAccountType {
        let id = self.library_id;
        self.library_id += 1;

        if self.program_config.libraries.insert(id, info).is_some() {
            panic!("Library with id {id} already exists");
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

        self.program_config.links.insert(
            id,
            Link {
                input_accounts_id: inputs.into_iter().map(|id| id.get_account_id()).collect(),
                output_accounts_id: outputs.into_iter().map(|id| id.get_account_id()).collect(),
                library_id: library_id.get_library_id(),
            },
        );
    }

    pub fn add_authorization(&mut self, authorization: AuthorizationInfo) {
        self.program_config.authorizations.push(authorization);
    }

    pub fn build(self) -> ProgramConfig {
        self.program_config
    }
}
