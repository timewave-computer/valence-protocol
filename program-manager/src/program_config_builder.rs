use valence_authorization_utils::authorization::AuthorizationInfo;
use valence_service_utils::{GetId, ServiceAccountType};

use crate::{
    account::AccountInfo,
    program_config::{Link, ProgramConfig},
    service::ServiceInfo,
};

#[derive(Default)]
pub struct ProgramConfigBuilder {
    account_id: u64,
    service_id: u64,
    link_id: u64,
    program_config: ProgramConfig,
}

impl ProgramConfigBuilder {
    pub fn new(owner: String) -> Self {
        ProgramConfigBuilder {
            program_config: ProgramConfig {
                owner,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn set_owner(&mut self, owner: String) {
        self.program_config.owner = owner;
    }

    pub fn add_account(&mut self, info: AccountInfo) -> ServiceAccountType {
        let id = self.account_id;
        self.account_id += 1;

        if self.program_config.accounts.insert(id, info).is_some() {
            panic!("Account with id {} already exists", id);
        }

        ServiceAccountType::AccountId(id)
    }

    pub fn add_service(&mut self, info: ServiceInfo) -> ServiceAccountType {
        let id = self.service_id;
        self.service_id += 1;

        if self.program_config.services.insert(id, info).is_some() {
            panic!("Service with id {} already exists", id);
        }

        ServiceAccountType::ServiceId(id)
    }

    pub fn add_link(
        &mut self,
        service_id: &impl GetId,
        inputs: Vec<&impl GetId>,
        outputs: Vec<&impl GetId>,
    ) {
        let id = self.link_id;
        self.link_id += 1;

        self.program_config.links.insert(
            id,
            Link {
                input_accounts_id: inputs.into_iter().map(|id| id.get_id()).collect(),
                output_accounts_id: outputs.into_iter().map(|id| id.get_id()).collect(),
                service_id: service_id.get_id(),
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
