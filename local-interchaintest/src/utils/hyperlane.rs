use localic_std::modules::cosmwasm::{contract_execute, contract_instantiate};
use localic_utils::{
    utils::test_context::TestContext, DEFAULT_KEY, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM,
    NEUTRON_CHAIN_NAME, NEUTRON_CHAIN_PREFIX,
};

use super::{
    ethereum::EthClient,
    solidity_contracts::{
        InterchainGasPaymaster, Mailbox, MerkleTreeHook, PausableIsm, ValidatorAnnounce,
    },
    GAS_FLAGS, HYPERLANE_COSMWASM_ARTIFACTS_PATH, LOCAL_CODE_ID_CACHE_PATH_NEUTRON,
};
pub struct HyperlaneContracts {
    pub mailbox: String,
    pub hook_merkle: String,
    pub igp: String,
    pub ism_pausable: String,
    pub validator_announce: String,
}

/// Converts a bech32 address to a hex address equivalent
pub fn bech32_to_hex_address(bech32_address: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Decode the bech32 address
    let (_, data) = bech32::decode(bech32_address)?;
    // Convert to hex and add 0x prefix
    let hex = format!("0x{}", hex::encode(data));
    Ok(hex)
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

    let merkle_hook_code_id = test_ctx
        .get_contract()
        .contract("hpl_hook_merkle")
        .get_cw()
        .code_id
        .unwrap();

    let hook_merkle_instantiate_msg = hpl_interface::hook::merkle::InstantiateMsg {
        mailbox: mailbox.clone(),
    };

    let hook_merkle = contract_instantiate(
        &test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        merkle_hook_code_id,
        &serde_json::to_string(&hook_merkle_instantiate_msg).unwrap(),
        "hook_merkle",
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

    let validator_announce_code_id = test_ctx
        .get_contract()
        .contract("hpl_validator_announce")
        .get_cw()
        .code_id
        .unwrap();

    let validator_announce_instantiate_msg = hpl_interface::core::va::InstantiateMsg {
        hrp: NEUTRON_CHAIN_PREFIX.to_string(),
        mailbox: mailbox.clone(),
    };

    let validator_announce = contract_instantiate(
        &test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        validator_announce_code_id,
        &serde_json::to_string(&validator_announce_instantiate_msg).unwrap(),
        "validator_announce",
        None,
        "",
    )
    .unwrap()
    .address;

    // Set hooks and ISM on mailbox
    let mailbox_set_default_hook_msg = hpl_interface::core::mailbox::ExecuteMsg::SetDefaultHook {
        hook: hook_merkle.clone(),
    };
    let mailbox_set_required_hook_msg = hpl_interface::core::mailbox::ExecuteMsg::SetRequiredHook {
        hook: hook_merkle.clone(),
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
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    Ok(HyperlaneContracts {
        mailbox,
        hook_merkle,
        igp,
        ism_pausable,
        validator_announce,
    })
}

pub fn set_up_eth_hyperlane_contracts(
    eth_client: &EthClient,
    domain_id: u32,
) -> Result<HyperlaneContracts, Box<dyn std::error::Error>> {
    let accounts = eth_client.get_accounts_addresses()?;

    let transaction = Mailbox::deploy_builder(&eth_client.provider, domain_id)
        .into_transaction_request()
        .from(accounts[0]);

    let mailbox = eth_client
        .send_transaction(transaction)?
        .contract_address
        .unwrap();

    let transaction = MerkleTreeHook::deploy_builder(&eth_client.provider, mailbox.clone())
        .into_transaction_request()
        .from(accounts[0]);

    let hook_merkle = eth_client
        .send_transaction(transaction)?
        .contract_address
        .unwrap();

    let transaction = InterchainGasPaymaster::deploy_builder(&eth_client.provider)
        .into_transaction_request()
        .from(accounts[0]);

    let igp = eth_client
        .send_transaction(transaction)?
        .contract_address
        .unwrap();

    let transaction = PausableIsm::deploy_builder(&eth_client.provider, accounts[0])
        .into_transaction_request()
        .from(accounts[0]);

    let ism_pausable = eth_client
        .send_transaction(transaction)?
        .contract_address
        .unwrap();

    let transaction = ValidatorAnnounce::deploy_builder(&eth_client.provider, mailbox.clone())
        .into_transaction_request()
        .from(accounts[0]);

    let validator_announce = eth_client
        .send_transaction(transaction)?
        .contract_address
        .unwrap();

    

    // Set hooks and ISM on mailbox
    let mailbox_contract = Mailbox::new(mailbox.clone(), &eth_client.provider);
    let tx = mailbox_contract
        .initialize(accounts[0], ism_pausable, hook_merkle, hook_merkle)
        .into_transaction_request()
        .from(accounts[0]);
    eth_client.send_transaction(tx)?;

    Ok(HyperlaneContracts {
        mailbox: mailbox.to_string(),
        hook_merkle: hook_merkle.to_string(),
        igp: igp.to_string(),
        ism_pausable: ism_pausable.to_string(),
        validator_announce: validator_announce.to_string(),
    })
}
