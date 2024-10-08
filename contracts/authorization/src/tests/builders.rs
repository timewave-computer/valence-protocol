use cosmwasm_std::{coins, Addr};
use neutron_test_tube::{Account, NeutronTestApp, SigningAccount};

const FEE_DENOM: &str = "untrn";
pub struct NeutronTestAppBuilder {
    fee_denom: String,
    initial_balance: u128,
    num_accounts: u64,
}

impl NeutronTestAppBuilder {
    pub fn new() -> Self {
        NeutronTestAppBuilder {
            fee_denom: FEE_DENOM.to_string(),
            initial_balance: 100_000_000_000,
            num_accounts: 5,
        }
    }

    pub fn with_num_accounts(mut self, num_accounts: u64) -> Self {
        self.num_accounts = num_accounts;
        self
    }

    pub fn build(self) -> Result<NeutronTestAppSetup, &'static str> {
        let app = NeutronTestApp::new();

        if self.num_accounts < 3 {
            return Err("Number of accounts must be at least 3");
        }

        let accounts = app
            .init_accounts(
                &coins(self.initial_balance, &self.fee_denom),
                self.num_accounts,
            )
            .map_err(|_| "Failed to initialize accounts")?;

        let mut accounts_iter = accounts.into_iter();
        let owner_accounts: Vec<SigningAccount> = accounts_iter.by_ref().take(2).collect();
        let user_accounts: Vec<SigningAccount> = accounts_iter.collect();

        let owner = &owner_accounts[0];
        let subowner = &owner_accounts[1];

        let owner_addr = Addr::unchecked(owner.address());
        let subowner_addr = Addr::unchecked(subowner.address());

        Ok(NeutronTestAppSetup {
            app,
            owner_accounts,
            user_accounts,
            owner_addr,
            subowner_addr,
        })
    }
}

pub struct NeutronTestAppSetup {
    pub app: NeutronTestApp,
    pub owner_accounts: Vec<SigningAccount>,
    pub user_accounts: Vec<SigningAccount>,
    pub owner_addr: Addr,
    pub subowner_addr: Addr,
}
