use std::{env, error::Error};

use localic_std::modules::cosmwasm::contract_instantiate;
use localic_utils::{
    utils::test_context::TestContext, DEFAULT_KEY, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME,
};
use log::info;

/// Sets up the authorization contract with its processor on a domain
pub fn set_up_authorization_and_processor(
    test_ctx: &mut TestContext,
    salt: String,
) -> Result<(String, String), Box<dyn Error>> {
    let mut uploader = test_ctx.build_tx_upload_contracts();

    // Upload the authorization contract to Neutron and the processor to both Neutron and Juno
    let current_dir = env::current_dir()?;

    let authorization_contract_path = format!(
        "{}/artifacts/valence_authorization.wasm",
        current_dir.display()
    );

    info!("{}", authorization_contract_path);

    let processor_contract_path =
        format!("{}/artifacts/valence_processor.wasm", current_dir.display());
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&authorization_contract_path)?;
    uploader.send_single_contract(&processor_contract_path)?;

    let predicted_authorization_contract_address = test_ctx
        .get_built_contract_address()
        .src(NEUTRON_CHAIN_NAME)
        .creator(NEUTRON_CHAIN_ADMIN_ADDR)
        .contract("valence_authorization")
        .salt_hex_encoded(&salt)
        .get();

    // Now we can instantiate the processor
    let processor_code_id_on_neutron = test_ctx
        .get_contract()
        .contract("valence_processor")
        .get_cw()
        .code_id
        .unwrap();

    let processor_instantiate_msg = valence_processor_utils::msg::InstantiateMsg {
        authorization_contract: predicted_authorization_contract_address.clone(),
        polytone_contracts: None,
    };

    let processor_on_main_domain = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        processor_code_id_on_neutron,
        &serde_json::to_string(&processor_instantiate_msg).unwrap(),
        "processor",
        None,
        "",
    )
    .unwrap();

    info!(
        "Processor on main domain: {}",
        processor_on_main_domain.address.clone()
    );

    // Instantiate the authorization contract now, we will add the external domains later
    let authorization_code_id = test_ctx
        .get_contract()
        .contract("valence_authorization")
        .get_cw()
        .code_id
        .unwrap();

    let authorization_instantiate_msg = valence_authorization_utils::msg::InstantiateMsg {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        sub_owners: vec![],
        processor: processor_on_main_domain.address.clone(),
    };

    test_ctx
        .build_tx_instantiate2()
        .with_label("authorization")
        .with_code_id(authorization_code_id)
        .with_salt_hex_encoded(&salt)
        .with_msg(serde_json::to_value(authorization_instantiate_msg).unwrap())
        .send()
        .unwrap();

    info!(
        "Authorization contract address: {}",
        predicted_authorization_contract_address.clone()
    );

    Ok((
        predicted_authorization_contract_address,
        processor_on_main_domain.address,
    ))
}
