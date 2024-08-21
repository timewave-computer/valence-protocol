use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

use crate::{
    config::Cfg,
    domain::{Domain, DomainInfo},
};

pub type Ctx = Arc<Mutex<ContextInner>>;

#[derive(Debug)]
pub struct Context(Arc<Mutex<ContextInner>>);

impl Default for Context {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(ContextInner {
            domain_infos: HashMap::new(),
            config: Cfg::default(),
        })))
    }
}

impl Context {
    pub fn get_clone(&self) -> Ctx {
        Arc::clone(&self.0)
    }

    pub async fn get_domain_infos_len(&self) -> usize {
        self.0.lock().await.domain_infos.len()
    }
}

#[derive(Debug, Default)]
pub struct ContextInner {
    pub domain_infos: HashMap<Domain, DomainInfo>,
    pub config: Cfg,
}

impl ContextInner {
    /// Get the domain from ctx if exists
    /// otherwise it gets a new domain info and save it ctx
    pub async fn get_or_create_domain_info(&mut self, domain: &Domain) -> &mut DomainInfo {
        if !self.domain_infos.contains_key(domain) {
            let domain_info = DomainInfo::from_domain(&self.config, domain).await;
            self.domain_infos.insert(domain.clone(), domain_info);
        }

        self.domain_infos.get_mut(domain).unwrap()
    }
}
