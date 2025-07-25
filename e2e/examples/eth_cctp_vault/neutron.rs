pub mod ica {
    use std::{error::Error, time::Duration};

    use cosmwasm_std::Uint64;
    use localic_std::modules::cosmwasm::{contract_execute, contract_instantiate, contract_query};
    use localic_utils::{
        utils::test_context::TestContext, DEFAULT_KEY, NEUTRON_CHAIN_ADMIN_ADDR,
        NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_NAME,
    };
    use log::info;
    use valence_account_utils::ica::{IcaState, RemoteDomainInfo};
    use valence_e2e::utils::{
        ibc::poll_for_ica_state, manager::INTERCHAIN_ACCOUNT_NAME, GAS_FLAGS, NOBLE_CHAIN_NAME,
    };

    pub fn instantiate_interchain_account_contract(
        test_ctx: &TestContext,
    ) -> Result<String, Box<dyn Error>> {
        let ica_account_code = *test_ctx
            .get_chain(NEUTRON_CHAIN_NAME)
            .contract_codes
            .get(INTERCHAIN_ACCOUNT_NAME)
            .unwrap();

        info!("Instantiating the ICA contract...");
        let timeout_seconds = 90;
        let ica_instantiate_msg = valence_account_utils::ica::InstantiateMsg {
            admin: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
            approved_libraries: vec![],
            remote_domain_information: RemoteDomainInfo {
                connection_id: test_ctx
                    .get_connections()
                    .src(NEUTRON_CHAIN_NAME)
                    .dest(NOBLE_CHAIN_NAME)
                    .get(),
                ica_timeout_seconds: Uint64::new(timeout_seconds),
            },
        };

        let valence_ica = contract_instantiate(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            DEFAULT_KEY,
            ica_account_code,
            &serde_json::to_string(&ica_instantiate_msg)?,
            "valence_ica",
            None,
            "",
        )?;
        info!(
            "ICA contract instantiated. Address: {}",
            valence_ica.address
        );

        Ok(valence_ica.address)
    }

    pub fn register_interchain_account(
        test_ctx: &mut TestContext,
        interchain_account_addr: &str,
    ) -> Result<String, Box<dyn Error>> {
        info!("Registering the ICA...");
        contract_execute(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            interchain_account_addr,
            DEFAULT_KEY,
            &serde_json::to_string(&valence_account_utils::ica::ExecuteMsg::RegisterIca {})
                .unwrap(),
            &format!("{GAS_FLAGS} --amount=100000000{NEUTRON_CHAIN_DENOM}"),
        )
        .unwrap();
        std::thread::sleep(Duration::from_secs(3));

        // We want to check that it's in state created
        poll_for_ica_state(test_ctx, interchain_account_addr, |state| {
            matches!(state, IcaState::Created(_))
        });

        // Get the remote address
        let ica_state: IcaState = serde_json::from_value(
            contract_query(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                interchain_account_addr,
                &serde_json::to_string(&valence_account_utils::ica::QueryMsg::IcaState {}).unwrap(),
            )["data"]
                .clone(),
        )
        .unwrap();

        let remote_address = match ica_state {
            IcaState::Created(ica_info) => ica_info.address,
            _ => {
                unreachable!("Expected IcaState::Created variant");
            }
        };
        info!("Remote address created: {}", remote_address);

        Ok(remote_address)
    }
}
