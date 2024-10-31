use std::{collections::BTreeMap, error::Error};

use cosmwasm_std::{Decimal, Uint128};
use local_interchaintest::utils::{
    manager::{
        setup_manager, ASTROPORT_LPER_NAME, ASTROPORT_WITHDRAWER_NAME, DETOKENIZER_NAME,
        FORWARDER_NAME, TOKENIZER_NAME,
    },
    LOGS_FILE_PATH, NEUTRON_CONFIG_FILE, VALENCE_ARTIFACTS_PATH,
};
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, GAIA_CHAIN_NAME, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME,
};
use valence_detokenizoooor_service::msg::DetokenizoooorConfig;
use valence_service_utils::denoms::UncheckedDenom;
use valence_splitter_service::msg::{UncheckedSplitAmount, UncheckedSplitConfig};
use valence_workflow_manager::{
    account::{AccountInfo, AccountType},
    service::{ServiceConfig, ServiceInfo},
    workflow_config_builder::WorkflowConfigBuilder,
};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .build()?;

    setup_manager(
        &mut test_ctx,
        NEUTRON_CONFIG_FILE,
        vec![GAIA_CHAIN_NAME],
        vec![
            TOKENIZER_NAME,
            DETOKENIZER_NAME,
            ASTROPORT_LPER_NAME,
            ASTROPORT_WITHDRAWER_NAME,
            FORWARDER_NAME,
        ],
    )?;

    let mut builder = WorkflowConfigBuilder::new(NEUTRON_CHAIN_ADMIN_ADDR.to_string());
    let neutron_domain =
        valence_workflow_manager::domain::Domain::CosmosCosmwasm(NEUTRON_CHAIN_NAME.to_string());

    let account_1 = builder.add_account(AccountInfo::new(
        "test_1".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let account_2 = builder.add_account(AccountInfo::new(
        "test_2".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let account_3 = builder.add_account(AccountInfo::new(
        "test_3".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let account_4 = builder.add_account(AccountInfo::new(
        "test_4".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let account_5 = builder.add_account(AccountInfo::new(
        "test_5".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));

    let mut price_map = BTreeMap::new();
    price_map.insert("untrn".to_string(), Uint128::one());
    let tokenizer_service = builder.add_service(ServiceInfo {
        name: "test_tokenizer".to_string(),
        domain: neutron_domain.clone(),
        config: ServiceConfig::ValenceTokenizer(valence_tokenizooor_service::msg::ServiceConfig {
            output_addr: account_1.clone(),
            input_denoms: price_map,
        }),
        addr: None,
    });

    let lper_service = builder.add_service(ServiceInfo {
        name: "test_lper".to_string(),
        domain: neutron_domain.clone(),
        config: ServiceConfig::ValenceAstroportLper(valence_astroport_lper::msg::ServiceConfig {
            input_addr: account_1,
            output_addr: account_2.clone(),
            pool_addr: todo!(),
            lp_config: todo!(),
        }),
        addr: None,
    });

    let withdrawer_service = builder.add_service(ServiceInfo {
        name: "test_withdrawer".to_string(),
        domain: neutron_domain.clone(),
        config: ServiceConfig::ValenceAstroportWithdrawer(
            valence_astroport_withdrawer::msg::ServiceConfig {
                input_addr: account_2,
                output_addr: account_3.clone(),
                pool_addr: todo!(),
                withdrawer_config: todo!(),
            },
        ),
        addr: None,
    });

    let splitter_service = builder.add_service(ServiceInfo {
        name: "test_splitter".to_string(),
        domain: neutron_domain.clone(),
        config: ServiceConfig::ValenceSplitterService(
            valence_splitter_service::msg::ServiceConfig {
                input_addr: account_3.clone(),
                splits: vec![
                    UncheckedSplitConfig {
                        denom: UncheckedDenom::Native("untrn".to_string()),
                        account: account_4.clone(),
                        amount: UncheckedSplitAmount::FixedRatio(Decimal::percent(95)),
                    },
                    UncheckedSplitConfig {
                        denom: UncheckedDenom::Native("shitcoin".to_string()),
                        account: account_5.clone(),
                        amount: UncheckedSplitAmount::FixedRatio(Decimal::percent(100)),
                    },
                    UncheckedSplitConfig {
                        denom: UncheckedDenom::Native("untrn".to_string()),
                        account: account_5.clone(),
                        amount: UncheckedSplitAmount::FixedRatio(Decimal::percent(5)),
                    },
                ],
            },
        ),
        addr: None,
    });

    let detokenizer_service = builder.add_service(ServiceInfo {
        name: "test_detokenizer".to_string(),
        domain: neutron_domain.clone(),
        config: ServiceConfig::ValenceDetokenizer(
            valence_detokenizoooor_service::msg::ServiceConfig {
                input_addr: account_5,
                voucher_denom: "dumdum".to_string(),
                detokenizoooor_config: DetokenizoooorConfig {
                    input_addr: todo!(),
                    voucher_denom: todo!(),
                    redeemable_denoms: todo!(),
                },
            },
        ),
        addr: None,
    });

    Ok(())
}
