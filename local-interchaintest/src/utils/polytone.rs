use std::{collections::HashMap, error::Error, time::Duration};

use cosmwasm_std_old::Uint64;
use localic_std::{modules::cosmwasm::CosmWasm, relayer::Relayer};
use localic_utils::{utils::test_context::TestContext, DEFAULT_KEY};
use log::info;
use sha2::{Digest, Sha512};
use valence_program_manager::bridge::{Bridge, PolytoneSingleChainInfo};

use crate::utils::manager::{
    get_global_config, POLYTONE_NOTE_NAME, POLYTONE_PROXY_NAME, POLYTONE_VOICE_NAME,
};

pub fn salt_for_proxy(
    connection_id: &str,
    counterparty_port: &str,
    remote_sender: &str,
) -> Vec<u8> {
    Sha512::default()
        .chain_update(connection_id.as_bytes())
        .chain_update(counterparty_port.as_bytes())
        .chain_update(remote_sender.as_bytes())
        .finalize()
        .to_vec()
}

/// performs the plumbing needed to establish a relayer polytone connection
/// between two domains
pub fn setup_polytone(
    test_ctx: &mut TestContext,
    domain_1: &str,
    domain_2: &str,
    domain_1_id: &str,
    domain_2_id: &str,
    domain_1_denom: &str,
    domain_2_denom: &str,
) -> Result<(), Box<dyn Error>> {
    let mut polytone_note_on_domain_1 = test_ctx
        .get_contract()
        .src(domain_1)
        .contract(POLYTONE_NOTE_NAME)
        .get_cw();

    let mut polytone_voice_on_domain_1 = test_ctx
        .get_contract()
        .src(domain_1)
        .contract(POLYTONE_VOICE_NAME)
        .get_cw();

    let polytone_proxy_on_domain_1 = test_ctx
        .get_contract()
        .src(domain_1)
        .contract(POLYTONE_PROXY_NAME)
        .get_cw();

    let mut polytone_note_on_domain_2 = test_ctx
        .get_contract()
        .src(domain_2)
        .contract(POLYTONE_NOTE_NAME)
        .get_cw();

    let mut polytone_voice_on_domain_2 = test_ctx
        .get_contract()
        .src(domain_2)
        .contract(POLYTONE_VOICE_NAME)
        .get_cw();

    let polytone_proxy_on_domain_2 = test_ctx
        .get_contract()
        .src(domain_2)
        .contract(POLYTONE_PROXY_NAME)
        .get_cw();

    let polytone_note_instantiate_msg = polytone_note::msg::InstantiateMsg {
        pair: None,
        block_max_gas: Uint64::new(3010000),
    };

    let domain_1_polytone_voice_instantiate_msg = polytone_voice::msg::InstantiateMsg {
        proxy_code_id: Uint64::new(polytone_proxy_on_domain_1.code_id.unwrap()),
        block_max_gas: Uint64::new(3010000),
        contract_addr_len: None,
    };

    let domain_2_polytone_voice_instantiate_msg = polytone_voice::msg::InstantiateMsg {
        proxy_code_id: Uint64::new(polytone_proxy_on_domain_2.code_id.unwrap()),
        block_max_gas: Uint64::new(3010000),
        contract_addr_len: None,
    };

    info!("Instantiating polytone contracts on both domains");
    let polytone_note_on_domain_1_address = polytone_note_on_domain_1
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&polytone_note_instantiate_msg)?,
            &format!("polytone-note-{domain_1}"),
            None,
            &format!("--fees {}{}", 5000, domain_1_denom),
        )?
        .address;

    info!("Polytone note on {domain_1}: {polytone_note_on_domain_1_address}");
    std::thread::sleep(std::time::Duration::from_secs(2));

    let polytone_voice_on_domain_1_address = polytone_voice_on_domain_1
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&domain_1_polytone_voice_instantiate_msg)?,
            &format!("polytone-voice-{domain_1}"),
            None,
            &format!("--fees {}{}", 5000, domain_1_denom),
        )?
        .address;

    info!("Polytone voice on {domain_1}: {polytone_voice_on_domain_1_address}",);
    std::thread::sleep(std::time::Duration::from_secs(2));

    let polytone_note_on_domain_2_address = polytone_note_on_domain_2
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&polytone_note_instantiate_msg)?,
            &format!("polytone-note-{domain_2}"),
            None,
            &format!("--fees {}{}", 5000, domain_2_denom),
        )?
        .address;

    info!("polytone note on {domain_2}: {polytone_note_on_domain_2_address}");
    std::thread::sleep(std::time::Duration::from_secs(2));

    let polytone_voice_on_domain_2_address = polytone_voice_on_domain_2
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&domain_2_polytone_voice_instantiate_msg)?,
            &format!("polytone-voice-{domain_2}"),
            None,
            &format!("--fees {}{}", 5000, domain_2_denom),
        )?
        .address;
    info!("Polytone voice on {domain_2}: {polytone_voice_on_domain_2_address}");

    std::thread::sleep(std::time::Duration::from_secs(2));
    info!("Creating WASM connections...");

    let relayer = Relayer::new(test_ctx.get_request_builder().get_request_builder(domain_1));

    polytone_note_on_domain_1.create_wasm_connection(
        &relayer,
        &format!("{domain_1}-{domain_2}"),
        &CosmWasm::new_from_existing(
            test_ctx.get_request_builder().get_request_builder(domain_2),
            None,
            None,
            Some(polytone_voice_on_domain_2_address.to_string()),
        ),
        "unordered",
        "polytone-1",
    )?;

    polytone_voice_on_domain_1.create_wasm_connection(
        &relayer,
        &format!("{domain_1}-{domain_2}"),
        &CosmWasm::new_from_existing(
            test_ctx.get_request_builder().get_request_builder(domain_2),
            None,
            None,
            Some(polytone_note_on_domain_2_address.to_string()),
        ),
        "unordered",
        "polytone-1",
    )?;

    // Give some time to make sure the channels are open
    std::thread::sleep(Duration::from_secs(15));

    // Get the connection ids so that we can predict the proxy addresses
    let domain_1_channels = relayer.get_channels(domain_1_id)?;

    let domain_1_to_domain_2_polytone_channel = domain_1_channels
        .iter()
        .find_map(|domain_1| {
            if domain_1.port_id == format!("wasm.{}", polytone_note_on_domain_1_address) {
                Some(domain_1.clone())
            } else {
                None
            }
        })
        .unwrap();

    let domain_2_channels = relayer.get_channels(domain_2_id)?;

    let domain_2_to_domain_1_polytone_channel = domain_2_channels
        .iter()
        .find_map(|domain_2_channel| {
            if domain_2_channel.port_id == format!("wasm.{}", polytone_note_on_domain_2_address) {
                Some(domain_2_channel.clone())
            } else {
                None
            }
        })
        .unwrap();

    let domain_2_polytone_info = PolytoneSingleChainInfo {
        voice_addr: polytone_voice_on_domain_2_address,
        note_addr: polytone_note_on_domain_2_address,
        other_note_port: domain_1_to_domain_2_polytone_channel.port_id,
        connection_id: domain_2_to_domain_1_polytone_channel
            .connection_hops
            .first()
            .cloned()
            .unwrap(),
        channel_id: domain_2_to_domain_1_polytone_channel.channel_id,
    };
    let domain_1_polytone_info = PolytoneSingleChainInfo {
        voice_addr: polytone_voice_on_domain_1_address,
        note_addr: polytone_note_on_domain_1_address,
        other_note_port: domain_2_to_domain_1_polytone_channel.port_id,
        connection_id: domain_1_to_domain_2_polytone_channel
            .connection_hops
            .first()
            .cloned()
            .unwrap(),
        channel_id: domain_1_to_domain_2_polytone_channel.channel_id,
    };

    let domain_2_to_domain_1_polytone_bridge_info: HashMap<String, PolytoneSingleChainInfo> =
        HashMap::from([
            (domain_1.to_string(), domain_1_polytone_info),
            (domain_2.to_string(), domain_2_polytone_info),
        ]);

    let mut domain_1_bridge_map: HashMap<String, Bridge> = HashMap::new();
    domain_1_bridge_map.insert(
        domain_2.to_string(),
        Bridge::Polytone(domain_2_to_domain_1_polytone_bridge_info),
    );

    let mut gc = get_global_config();

    gc.bridges.insert(domain_1.to_string(), domain_1_bridge_map);

    Ok(())
}
