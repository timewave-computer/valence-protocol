use authorization_utils::domain::{CallBackProxy, Connector, ExternalDomain};
use cosmwasm_std::{coins, Addr};
use neutron_test_tube::{Account, NeutronTestApp, SigningAccount};

const FEE_DENOM: &str = "untrn";

pub struct NeutronTestAppBuilder {
    external_domain: String,
    fee_denom: String,
    initial_balance: u128,
    num_accounts: u64,
}

impl NeutronTestAppBuilder {
    pub fn new() -> Self {
        NeutronTestAppBuilder {
            external_domain: "osmosis".to_string(),
            fee_denom: FEE_DENOM.to_string(),
            initial_balance: 100_000_000_000,
            num_accounts: 6,
        }
    }

    pub fn with_num_accounts(mut self, num_accounts: u64) -> Self {
        self.num_accounts = num_accounts;
        self
    }

    pub fn build(self) -> Result<NeutronTestAppSetup, &'static str> {
        let app = NeutronTestApp::new();

        if self.num_accounts < 6 {
            return Err("Number of accounts must be at least 6");
        }

        let accounts = app
            .init_accounts(
                &coins(self.initial_balance, &self.fee_denom),
                self.num_accounts,
            )
            .map_err(|_| "Failed to initialize accounts")?;

        let owner = &accounts[0];
        let subowner = &accounts[1];
        let user = &accounts[2];
        let processor = &accounts[3];
        let connector = &accounts[4];
        let callback_proxy = &accounts[5];

        let owner_addr = Addr::unchecked(owner.address());
        let subowner_addr = Addr::unchecked(subowner.address());
        let processor_addr = Addr::unchecked(processor.address());
        let connector_addr = Addr::unchecked(connector.address());
        let callback_proxy_addr = Addr::unchecked(callback_proxy.address());
        let user_addr = Addr::unchecked(user.address());

        let external_domain = ExternalDomain {
            name: self.external_domain,
            connector: Connector::PolytoneNote(connector_addr),
            processor: "processor".to_string(),
            callback_proxy: CallBackProxy::PolytoneProxy(callback_proxy_addr),
        };

        Ok(NeutronTestAppSetup {
            app,
            accounts,
            external_domain,
            owner_addr,
            subowner_addr,
            user_addr,
            processor_addr,
        })
    }
}

pub struct NeutronTestAppSetup {
    pub app: NeutronTestApp,
    pub accounts: Vec<SigningAccount>,
    pub external_domain: ExternalDomain,
    pub owner_addr: Addr,
    pub subowner_addr: Addr,
    pub user_addr: Addr,
    pub processor_addr: Addr,
}
