use std::error::Error;

use cosmwasm_std::Int64;
use local_interchaintest::utils::{
    manager::{
        setup_manager, OSMOSIS_CL_LPER_NAME, OSMOSIS_CL_LWER_NAME, POLYTONE_NOTE_NAME,
        POLYTONE_PROXY_NAME, POLYTONE_VOICE_NAME,
    },
    osmosis::concentrated_liquidity::setup_cl_pool,
    LOGS_FILE_PATH, NEUTRON_OSMO_CONFIG_FILE, VALENCE_ARTIFACTS_PATH,
};

use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, GAIA_CHAIN_NAME, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_NAME, OSMOSIS_CHAIN_DENOM,
    OSMOSIS_CHAIN_NAME,
};
use log::info;
use valence_osmosis_utils::utils::cl_utils::TickRange;
use valence_program_manager::{
    account::{AccountInfo, AccountType},
    program_config_builder::ProgramConfigBuilder,
};

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

    let ntrn_on_osmo_denom = test_ctx
        .get_ibc_denom()
        .base_denom(NEUTRON_CHAIN_DENOM.to_owned())
        .src(NEUTRON_CHAIN_NAME)
        .dest(OSMOSIS_CHAIN_NAME)
        .get();

    let pool_id = setup_cl_pool(&mut test_ctx, OSMOSIS_CHAIN_DENOM, &ntrn_on_osmo_denom)?;

    setup_manager(
        &mut test_ctx,
        NEUTRON_OSMO_CONFIG_FILE,
        vec![GAIA_CHAIN_NAME],
        vec![
            OSMOSIS_CL_LPER_NAME,
            OSMOSIS_CL_LWER_NAME,
            POLYTONE_NOTE_NAME,
            POLYTONE_VOICE_NAME,
            POLYTONE_PROXY_NAME,
        ],
    )?;

    let mut builder = ProgramConfigBuilder::new(NEUTRON_CHAIN_ADMIN_ADDR.to_string());
    let osmo_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm(OSMOSIS_CHAIN_NAME.to_string());
    let ntrn_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm(NEUTRON_CHAIN_NAME.to_string());

    let cl_input_acc_info =
        AccountInfo::new("cl_input".to_string(), &osmo_domain, AccountType::default());
    let cl_output_acc_info = AccountInfo::new(
        "cl_output".to_string(),
        &osmo_domain,
        AccountType::default(),
    );
    let final_output_acc_info = AccountInfo::new(
        "final_output".to_string(),
        &osmo_domain,
        AccountType::default(),
    );

    let cl_input_acc = builder.add_account(cl_input_acc_info);
    let cl_output_acc = builder.add_account(cl_output_acc_info);
    let final_output_acc = builder.add_account(final_output_acc_info);

    info!("cl input acc: {:?}", cl_input_acc);
    info!("cl output acc: {:?}", cl_output_acc);
    info!("final output acc: {:?}", final_output_acc);

    let cl_lper_config = valence_osmosis_cl_lper::msg::LibraryConfig {
        input_addr: cl_input_acc.clone(),
        output_addr: cl_output_acc.clone(),
        lp_config: valence_osmosis_cl_lper::msg::LiquidityProviderConfig {
            pool_id: pool_id.into(),
            pool_asset_2: OSMOSIS_CHAIN_DENOM.to_string(),
            pool_asset_1: ntrn_on_osmo_denom.to_string(),
            global_tick_range: TickRange {
                lower_tick: Int64::from(-1000),
                upper_tick: Int64::from(1000),
            },
        },
    };

    let cl_lwer_config = valence_osmosis_cl_withdrawer::msg::LibraryConfig {
        input_addr: cl_output_acc.clone(),
        output_addr: final_output_acc.clone(),
        pool_id: pool_id.into(),
    };

    Ok(())
}
