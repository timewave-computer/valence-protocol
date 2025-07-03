use clap::{arg, command, Parser};
use std::{error::Error, fs, path::Path};

use localic_utils::{
    types::config::ConfigChain, utils::test_context::TestContext, TestContextBuilder,
    GAIA_CHAIN_NAME, LOCAL_IC_API_URL, NEUTRON_CHAIN_NAME,
};
use valence_e2e::utils::{
    manager::{
        get_global_config, setup_manager, ASTROPORT_LPER_NAME, ASTROPORT_WITHDRAWER_NAME,
        FORWARDER_NAME, GENERIC_IBC_TRANSFER_NAME, LOG_FILE_PATH, NEUTRON_IBC_TRANSFER_NAME,
        OSMOSIS_CL_LPER_NAME, OSMOSIS_CL_LWER_NAME, OSMOSIS_GAMM_LPER_NAME, OSMOSIS_GAMM_LWER_NAME,
        POLYTONE_NOTE_NAME, POLYTONE_PROXY_NAME, POLYTONE_VOICE_NAME, REVERSE_SPLITTER_NAME,
        SPLITTER_NAME,
    },
    polytone::setup_polytone,
    NEUTRON_CONFIG_FILE,
};

pub const VALENCE_ARTIFACTS_PATH: &str = "artifacts";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Enviroment config to use
    #[arg(short, long)]
    chain_config: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let chain_config_path = args.chain_config.unwrap_or(NEUTRON_CONFIG_FILE.to_string());

    let mut test_ctx_builder = TestContextBuilder::default();
    test_ctx_builder
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_log_file_path(LOG_FILE_PATH);

    // Set chains in context based on chain config
    set_chains_in_context(&mut test_ctx_builder, chain_config_path.as_str());

    let mut test_ctx = test_ctx_builder.build()?;

    // Setup local environment
    setup_manager(
        &mut test_ctx,
        chain_config_path.as_str(),
        vec![GAIA_CHAIN_NAME],
        vec![
            POLYTONE_NOTE_NAME,
            POLYTONE_VOICE_NAME,
            POLYTONE_PROXY_NAME,
            SPLITTER_NAME,
            REVERSE_SPLITTER_NAME,
            FORWARDER_NAME,
            GENERIC_IBC_TRANSFER_NAME,
            NEUTRON_IBC_TRANSFER_NAME,
            ASTROPORT_LPER_NAME,
            ASTROPORT_WITHDRAWER_NAME,
            OSMOSIS_GAMM_LPER_NAME,
            OSMOSIS_GAMM_LWER_NAME,
            OSMOSIS_CL_LPER_NAME,
            OSMOSIS_CL_LWER_NAME,
        ],
    )?;

    setup_bridges(&mut test_ctx, vec![GAIA_CHAIN_NAME]);

    // Export manager config to file
    export_manager_config()?;

    Ok(())
}

fn set_chains_in_context(test_ctx_builder: &mut TestContextBuilder, chain_config_path: &str) {
    let chain_file_path = format!("e2e/chains/{chain_config_path}");
    let chain_file = fs::File::open(chain_file_path).expect("Couldn't open chain config file");
    let chain_json: serde_json::Value =
        serde_json::from_reader(chain_file).expect("Chain config file is not a valid JSON");

    let chain_json = chain_json
        .get("chains")
        .expect("file should have chains")
        .as_array()
        .expect("Chains should be an array");

    chain_json.iter().for_each(|chain_data| {
        let chain_name = chain_data
            .get("name")
            .expect("Chain data must have name")
            .as_str()
            .expect("name must be a string");
        let chain_id = chain_data
            .get("chain_id")
            .expect("Chain data must have chain_id")
            .as_str()
            .expect("chain_id must be a string");

        let chain_prefix = chain_data
            .get("bech32_prefix")
            .expect("Chain data must have bech32_prefix")
            .as_str()
            .expect("bech32_prefix must be a string");

        let chain_denom = chain_data
            .get("denom")
            .expect("Chain data must have denom")
            .as_str()
            .expect("denom must be a string");

        // TODO: This might change for ETH based on the chain config
        let genesis = chain_data
            .get("genesis")
            .expect("Chain data must have genesis")
            .as_object()
            .expect("genesis must be an object");
        let accounts = genesis
            .get("accounts")
            .expect("Genesis must have accounts")
            .as_array()
            .expect("accounts must be an array");
        let admin_addr = accounts[0]
            .as_object()
            .expect("Account myust be an object")
            .get("address")
            .expect("Account must have address")
            .as_str()
            .expect("Address must be a string");

        test_ctx_builder.with_chain(ConfigChain {
            denom: chain_denom.to_string(),
            debugging: true,
            chain_id: chain_id.to_string(),
            chain_name: chain_name.to_string(),
            chain_prefix: chain_prefix.to_string(),
            admin_addr: admin_addr.to_string(),
        });
    })
}

fn setup_bridges(test_ctx: &mut TestContext, ignore_chain: Vec<&str>) {
    let chain_names = test_ctx.chains.keys().cloned().collect::<Vec<String>>();

    let neutron_chain_id = test_ctx.get_chain(NEUTRON_CHAIN_NAME).rb.chain_id.clone();
    let neutron_chain_denom = test_ctx.get_chain(NEUTRON_CHAIN_NAME).native_denom.clone();

    for chain_name in chain_names.clone() {
        if ignore_chain.contains(&chain_name.as_str()) || chain_name == NEUTRON_CHAIN_NAME {
            continue;
        }

        let other_chain_id = test_ctx.get_chain(chain_name.as_str()).rb.chain_id.clone();
        let other_chain_denom = test_ctx.get_chain(chain_name.as_str()).native_denom.clone();

        setup_polytone(
            test_ctx,
            NEUTRON_CHAIN_NAME,
            chain_name.as_str(),
            neutron_chain_id.as_str(),
            other_chain_id.as_str(),
            neutron_chain_denom.as_str(),
            other_chain_denom.as_str(),
        )
        .unwrap();
    }
}

fn export_manager_config() -> Result<(), Box<dyn Error>> {
    let gc = get_global_config();

    let path_name = "deployment/configs/local/";
    let path = Path::new(path_name);

    if !path.exists() {
        fs::create_dir_all(path_name)?;
    }

    let file_path = path.join("config.toml");

    let t = toml::to_string(&*gc).unwrap();
    fs::write(file_path, t)?;
    Ok(())
}
