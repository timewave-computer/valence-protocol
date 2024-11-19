use std::{collections::HashMap, error::Error, time::Duration};

use cosmwasm_std_old::Uint64;
use local_interchaintest::utils::{
    manager::{
        get_global_config, setup_manager, use_manager_init, OSMOSIS_GAMM_LPER_NAME,
        OSMOSIS_GAMM_LWER_NAME, POLYTONE_NOTE_NAME, POLYTONE_PROXY_NAME, POLYTONE_VOICE_NAME,
    },
    LOGS_FILE_PATH, NEUTRON_OSMO_CONFIG_FILE, VALENCE_ARTIFACTS_PATH,
};

use localic_std::{
    modules::{bank, cosmwasm::CosmWasm},
    relayer::Relayer,
};
use localic_utils::{
    utils::test_context::TestContext, ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY,
    GAIA_CHAIN_NAME, LOCAL_IC_API_URL, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM,
    NEUTRON_CHAIN_ID, NEUTRON_CHAIN_NAME, OSMOSIS_CHAIN_ADMIN_ADDR, OSMOSIS_CHAIN_DENOM,
    OSMOSIS_CHAIN_ID, OSMOSIS_CHAIN_NAME,
};
use log::info;
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
};
use valence_program_manager::{
    account::{AccountInfo, AccountType},
    bridge::{Bridge, PolytoneSingleChainInfo},
    library::{LibraryConfig, LibraryInfo},
    program_config_builder::ProgramConfigBuilder,
};

fn setup_gamm_pool(
    test_ctx: &mut TestContext,
    denom_1: &str,
    denom_2: &str,
) -> Result<u64, Box<dyn Error>> {
    info!("transferring 1000 neutron tokens to osmo admin addr for pool creation...");
    test_ctx
        .build_tx_transfer()
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .with_amount(1_000_000_000u128)
        .with_recipient(OSMOSIS_CHAIN_ADMIN_ADDR)
        .with_denom(NEUTRON_CHAIN_DENOM)
        .send()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    let token_balances = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        OSMOSIS_CHAIN_ADMIN_ADDR,
    );
    info!("osmosis chain admin addr balances: {:?}", token_balances);

    test_ctx
        .build_tx_create_osmo_pool()
        .with_weight(denom_1, 1)
        .with_weight(denom_2, 1)
        .with_initial_deposit(denom_1, 100_000_000)
        .with_initial_deposit(denom_2, 100_000_000)
        .send()?;

    // Get its id
    let pool_id = test_ctx
        .get_osmo_pool()
        .denoms(denom_1.into(), denom_2.to_string())
        .get_u64();

    info!("Gamm pool id: {:?}", pool_id);

    Ok(pool_id)
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_chain(ConfigChainBuilder::default_osmosis().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, OSMOSIS_CHAIN_NAME)
        .build()?;

    setup_manager(
        &mut test_ctx,
        NEUTRON_OSMO_CONFIG_FILE,
        vec![GAIA_CHAIN_NAME],
        vec![
            OSMOSIS_GAMM_LPER_NAME,
            OSMOSIS_GAMM_LWER_NAME,
            POLYTONE_NOTE_NAME,
            POLYTONE_VOICE_NAME,
            POLYTONE_PROXY_NAME,
        ],
    )?;

    let mut builder = ProgramConfigBuilder::new(NEUTRON_CHAIN_ADMIN_ADDR.to_string());
    let _neutron_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm(NEUTRON_CHAIN_NAME.to_string());
    let osmo_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm(OSMOSIS_CHAIN_NAME.to_string());

    let gamm_input_acc = builder.add_account(AccountInfo::new(
        "gamm_input".to_string(),
        &osmo_domain,
        AccountType::default(),
    ));
    let gamm_output_acc = builder.add_account(AccountInfo::new(
        "gamm_output".to_string(),
        &osmo_domain,
        AccountType::default(),
    ));

    let ntrn_on_osmo_denom = test_ctx
        .get_ibc_denom()
        .base_denom(NEUTRON_CHAIN_DENOM.to_owned())
        .src(NEUTRON_CHAIN_NAME)
        .dest(OSMOSIS_CHAIN_NAME)
        .get();

    let pool_id = setup_gamm_pool(&mut test_ctx, OSMOSIS_CHAIN_DENOM, &ntrn_on_osmo_denom)?;

    let gamm_lper_config = valence_osmosis_gamm_lper::msg::LibraryConfig {
        input_addr: gamm_input_acc.clone(),
        output_addr: gamm_output_acc.clone(),
        lp_config: valence_osmosis_gamm_lper::msg::LiquidityProviderConfig {
            pool_id,
            pool_asset_1: OSMOSIS_CHAIN_DENOM.to_string(),
            pool_asset_2: ntrn_on_osmo_denom.to_string(),
        },
    };

    let gamm_lper_library = builder.add_library(LibraryInfo::new(
        "test_gamm_lp".to_string(),
        &osmo_domain,
        LibraryConfig::ValenceOsmosisGammLper(gamm_lper_config.clone()),
    ));

    builder.add_link(
        &gamm_lper_library,
        vec![&gamm_input_acc],
        vec![&gamm_output_acc],
    );

    let gamm_lper_function = AtomicFunctionBuilder::new()
        .with_contract_address(gamm_lper_library.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "provide_liquidity".to_string(),
                params_restrictions: None,
            },
        })
        .build();

    let gamm_lper_subroutine = AtomicSubroutineBuilder::new()
        .with_function(gamm_lper_function)
        .build();

    let provide_liquidity_authorization = AuthorizationBuilder::new()
        .with_subroutine(gamm_lper_subroutine)
        .build();

    builder.add_authorization(provide_liquidity_authorization);

    let mut program_config = builder.build();

    setup_polytone(&mut test_ctx)?;

    use_manager_init(&mut program_config)?;

    Ok(())
}

fn setup_polytone(test_ctx: &mut TestContext) -> Result<(), Box<dyn Error>> {
    // Before setting up the external domains and the processor on the external domain, we are going to set up polytone and predict the proxy addresses on both sides
    let mut polytone_note_on_neutron = test_ctx
        .get_contract()
        .contract(POLYTONE_NOTE_NAME)
        .get_cw();

    let mut polytone_voice_on_neutron = test_ctx
        .get_contract()
        .contract(POLYTONE_VOICE_NAME)
        .get_cw();

    let polytone_proxy_on_neutron = test_ctx
        .get_contract()
        .contract(POLYTONE_PROXY_NAME)
        .get_cw();

    let mut polytone_note_on_osmosis = test_ctx
        .get_contract()
        .src(OSMOSIS_CHAIN_NAME)
        .contract(POLYTONE_NOTE_NAME)
        .get_cw();

    let mut polytone_voice_on_osmosis = test_ctx
        .get_contract()
        .src(OSMOSIS_CHAIN_NAME)
        .contract(POLYTONE_VOICE_NAME)
        .get_cw();

    let polytone_proxy_on_osmosis = test_ctx
        .get_contract()
        .src(OSMOSIS_CHAIN_NAME)
        .contract(POLYTONE_PROXY_NAME)
        .get_cw();

    let polytone_note_instantiate_msg = polytone_note::msg::InstantiateMsg {
        pair: None,
        block_max_gas: Uint64::new(3010000),
    };

    let neutron_polytone_voice_instantiate_msg = polytone_voice::msg::InstantiateMsg {
        proxy_code_id: Uint64::new(polytone_proxy_on_neutron.code_id.unwrap()),
        block_max_gas: Uint64::new(3010000),
        contract_addr_len: None,
    };

    let osmo_polytone_voice_instantiate_msg = polytone_voice::msg::InstantiateMsg {
        proxy_code_id: Uint64::new(polytone_proxy_on_osmosis.code_id.unwrap()),
        block_max_gas: Uint64::new(3010000),
        contract_addr_len: None,
    };

    info!("Instantiating polytone contracts on both domains");
    let polytone_note_on_neutron_address = polytone_note_on_neutron
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&polytone_note_instantiate_msg).unwrap(),
            "polytone-note-neutron",
            None,
            "",
        )
        .unwrap()
        .address;

    info!("Polytone Note on Neutron: {polytone_note_on_neutron_address}");
    std::thread::sleep(std::time::Duration::from_secs(2));

    let polytone_voice_on_neutron_address = polytone_voice_on_neutron
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&neutron_polytone_voice_instantiate_msg).unwrap(),
            "polytone-voice-neutron",
            None,
            "",
        )
        .unwrap()
        .address;

    info!("Polytone voice on neutron: {polytone_voice_on_neutron_address}",);
    std::thread::sleep(std::time::Duration::from_secs(2));

    let polytone_note_on_osmo_address = polytone_note_on_osmosis
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&polytone_note_instantiate_msg).unwrap(),
            "polytone-note-osmosis",
            None,
            &format!("--fees {}{}", 5000, OSMOSIS_CHAIN_DENOM),
        )
        .unwrap()
        .address;

    info!("polytone note on osmosis: {polytone_note_on_osmo_address}");
    std::thread::sleep(std::time::Duration::from_secs(2));

    let polytone_voice_on_osmo_address = polytone_voice_on_osmosis
        .instantiate(
            DEFAULT_KEY,
            &serde_json::to_string(&osmo_polytone_voice_instantiate_msg).unwrap(),
            "polytone-voice-osmosis",
            None,
            &format!("--fees {}{}", 5000, OSMOSIS_CHAIN_DENOM),
        )
        .unwrap()
        .address;
    info!("Polytone Voice on osmo: {polytone_voice_on_osmo_address}");

    std::thread::sleep(std::time::Duration::from_secs(2));
    info!("Creating WASM connections...");

    let relayer = Relayer::new(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
    );

    polytone_note_on_neutron
        .create_wasm_connection(
            &relayer,
            "neutron-osmosis",
            &CosmWasm::new_from_existing(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(OSMOSIS_CHAIN_NAME),
                None,
                None,
                Some(polytone_voice_on_osmo_address.clone()),
            ),
            "unordered",
            "polytone-1",
        )
        .unwrap();

    polytone_voice_on_neutron
        .create_wasm_connection(
            &relayer,
            "neutron-osmosis",
            &CosmWasm::new_from_existing(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(OSMOSIS_CHAIN_NAME),
                None,
                None,
                Some(polytone_note_on_osmo_address.clone()),
            ),
            "unordered",
            "polytone-1",
        )
        .unwrap();

    // Give some time to make sure the channels are open
    std::thread::sleep(Duration::from_secs(15));

    // Get the connection ids so that we can predict the proxy addresses
    let neutron_channels = relayer.get_channels(NEUTRON_CHAIN_ID).unwrap();

    let neutron_to_osmo_polytone_channel = neutron_channels
        .iter()
        .find_map(|neutron_channel| {
            if neutron_channel.port_id
                == format!("wasm.{}", polytone_note_on_neutron_address.clone())
            {
                Some(neutron_channel.clone())
            } else {
                None
            }
        })
        .unwrap();

    let osmo_channels = relayer.get_channels(OSMOSIS_CHAIN_ID).unwrap();

    let osmo_to_neutron_polytone_channel = osmo_channels
        .iter()
        .find_map(|osmo_channel| {
            if osmo_channel.port_id == format!("wasm.{}", polytone_note_on_osmo_address.clone()) {
                Some(osmo_channel.clone())
            } else {
                None
            }
        })
        .unwrap();

    let osmo_polytone_info = PolytoneSingleChainInfo {
        voice_addr: polytone_voice_on_osmo_address,
        note_addr: polytone_note_on_osmo_address,
        other_note_port: neutron_to_osmo_polytone_channel.port_id,
        connection_id: osmo_to_neutron_polytone_channel
            .connection_hops
            .first()
            .cloned()
            .unwrap(),
        channel_id: osmo_to_neutron_polytone_channel.channel_id,
    };
    let neutron_polytone_info = PolytoneSingleChainInfo {
        voice_addr: polytone_voice_on_neutron_address,
        note_addr: polytone_note_on_neutron_address,
        other_note_port: osmo_to_neutron_polytone_channel.port_id,
        connection_id: neutron_to_osmo_polytone_channel
            .connection_hops
            .first()
            .cloned()
            .unwrap(),
        channel_id: neutron_to_osmo_polytone_channel.channel_id,
    };

    let osmo_to_neutron_polytone_bridge_info: HashMap<String, PolytoneSingleChainInfo> =
        HashMap::from([
            (NEUTRON_CHAIN_NAME.to_string(), neutron_polytone_info),
            (OSMOSIS_CHAIN_NAME.to_string(), osmo_polytone_info),
        ]);

    let mut neutron_bridge_map: HashMap<String, Bridge> = HashMap::new();
    neutron_bridge_map.insert(
        OSMOSIS_CHAIN_NAME.to_string(),
        Bridge::Polytone(osmo_to_neutron_polytone_bridge_info),
    );

    let mut gc = get_global_config();

    gc.bridges
        .insert(NEUTRON_CHAIN_NAME.to_string(), neutron_bridge_map);

    Ok(())
}
