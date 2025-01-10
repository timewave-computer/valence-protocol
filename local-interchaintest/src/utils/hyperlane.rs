use localic_std::modules::cosmwasm::{contract_execute, contract_instantiate};
use localic_utils::{
    utils::test_context::TestContext, DEFAULT_KEY, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM,
    NEUTRON_CHAIN_NAME, NEUTRON_CHAIN_PREFIX,
};

use super::{GAS_FLAGS, HYPERLANE_COSMWASM_ARTIFACTS_PATH, LOCAL_CODE_ID_CACHE_PATH_NEUTRON};

pub struct HyperlaneContracts {
    pub mailbox: String,
    pub hook_pausable: String,
    pub igp: String,
    pub ism_pausable: String,
}

pub fn set_up_cw_hyperlane_contracts(
    test_ctx: &mut TestContext,
) -> Result<HyperlaneContracts, Box<dyn std::error::Error>> {
    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .send_with_local_cache(
            HYPERLANE_COSMWASM_ARTIFACTS_PATH,
            LOCAL_CODE_ID_CACHE_PATH_NEUTRON,
        )
        .unwrap();

    let mailbox_code_id = test_ctx
        .get_contract()
        .contract("hpl_mailbox")
        .get_cw()
        .code_id
        .unwrap();

    let mailbox_instantiate_msg = hpl_interface::core::mailbox::InstantiateMsg {
        hrp: NEUTRON_CHAIN_PREFIX.to_string(),
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        domain: 1853125230, // Domain ID of Neutron for Hyperlane
    };

    let mailbox = contract_instantiate(
        &test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        mailbox_code_id,
        &serde_json::to_string(&mailbox_instantiate_msg).unwrap(),
        "mailbox",
        None,
        "",
    )
    .unwrap()
    .address;

    let pausable_hook_code_id = test_ctx
        .get_contract()
        .contract("hpl_hook_pausable")
        .get_cw()
        .code_id
        .unwrap();

    let hook_pausable_instantiate_msg = hpl_interface::hook::pausable::InstantiateMsg {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        paused: false,
    };

    let hook_pausable = contract_instantiate(
        &test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        pausable_hook_code_id,
        &serde_json::to_string(&hook_pausable_instantiate_msg).unwrap(),
        "hook_pausable",
        None,
        "",
    )
    .unwrap()
    .address;

    let igp_code_id = test_ctx
        .get_contract()
        .contract("hpl_igp")
        .get_cw()
        .code_id
        .unwrap();

    let igp_instantiate_msg = hpl_interface::igp::core::InstantiateMsg {
        hrp: NEUTRON_CHAIN_PREFIX.to_string(),
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        gas_token: NEUTRON_CHAIN_DENOM.to_string(),
        beneficiary: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        default_gas_usage: 0,
    };

    let igp = contract_instantiate(
        &test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        igp_code_id,
        // We are using an older version of serde_json_wasm that serializes u128 as a String instead of a number. Unfortunately
        // hyperlane used u128 (instead of Uint128) for default_gas_usage which the latest versions of serde_json and serde_json_wasm serialize as a number instead of a string.
        // Depending on what version of cosmwasm you are using, you need to pass a number or a string. In this case, we are using a string
        &serde_json_wasm::to_string(&igp_instantiate_msg).unwrap(),
        "igp",
        None,
        "",
    )
    .unwrap()
    .address;

    let ism_pausable_code_id = test_ctx
        .get_contract()
        .contract("hpl_ism_pausable")
        .get_cw()
        .code_id
        .unwrap();

    let ism_pausable_instantiate_msg = hpl_interface::ism::pausable::InstantiateMsg {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        paused: false,
    };

    let ism_pausable = contract_instantiate(
        &test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        ism_pausable_code_id,
        &serde_json::to_string(&ism_pausable_instantiate_msg).unwrap(),
        "ism_pausable",
        None,
        "",
    )
    .unwrap()
    .address;

    // Set hooks and ISM on mailbox
    let mailbox_set_default_hook_msg = hpl_interface::core::mailbox::ExecuteMsg::SetDefaultHook {
        hook: hook_pausable.clone(),
    };
    let mailbox_set_required_hook_msg = hpl_interface::core::mailbox::ExecuteMsg::SetRequiredHook {
        hook: hook_pausable.clone(),
    };
    let mailbox_set_default_ism_msg = hpl_interface::core::mailbox::ExecuteMsg::SetDefaultIsm {
        ism: ism_pausable.clone(),
    };

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &mailbox,
        DEFAULT_KEY,
        &serde_json::to_string(&mailbox_set_default_hook_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &mailbox,
        DEFAULT_KEY,
        &serde_json::to_string(&mailbox_set_required_hook_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &mailbox,
        DEFAULT_KEY,
        &serde_json::to_string(&mailbox_set_default_ism_msg).unwrap(),
        GAS_FLAGS,
    ).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    Ok(HyperlaneContracts {
        mailbox,
        hook_pausable,
        igp,
        ism_pausable,
    })
}
