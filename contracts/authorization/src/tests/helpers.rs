use neutron_test_tube::{Account, NeutronTestApp, SigningAccount, Wasm};
use valence_authorization_utils::domain::ExternalDomain;

use crate::msg::InstantiateMsg;
use valence_processor::msg::{
    ExecuteMsg as ProcessorExecuteMsg, InstantiateMsg as ProcessorInstantiateMsg, OwnerMsg,
};

pub fn store_and_instantiate_authorization_with_processor_contract(
    wasm: &Wasm<'_, NeutronTestApp>,
    signer: &SigningAccount,
    owner: String,
    sub_owners: Vec<String>,
    external_domains: Vec<ExternalDomain>,
) -> (String, String) {
    let wasm_byte_code_authorization =
        std::fs::read("../../artifacts/valence_authorization.wasm").unwrap();
    let wasm_byte_code_processor = std::fs::read("../../artifacts/valence_processor.wasm").unwrap();

    let code_id_authorization = wasm
        .store_code(&wasm_byte_code_authorization, None, signer)
        .unwrap()
        .data
        .code_id;
    let code_id_processor = wasm
        .store_code(&wasm_byte_code_processor, None, signer)
        .unwrap()
        .data
        .code_id;

    let processor_address = wasm
        .instantiate(
            code_id_processor,
            &ProcessorInstantiateMsg {
                owner: signer.address().to_string(),
                authorization_contract: owner.clone(),
                polytone_contracts: None,
            },
            None,
            "processor".into(),
            &[],
            signer,
        )
        .unwrap()
        .data
        .address;

    let authorization_address = wasm
        .instantiate(
            code_id_authorization,
            &InstantiateMsg {
                owner,
                sub_owners,
                processor: processor_address.clone(),
                external_domains,
            },
            None,
            "authorization".into(),
            &[],
            signer,
        )
        .unwrap()
        .data
        .address;

    wasm.execute::<ProcessorExecuteMsg>(
        &processor_address,
        &ProcessorExecuteMsg::OwnerAction(OwnerMsg::UpdateConfig {
            authorization_contract: Some(authorization_address.clone()),
            polytone_contracts: None,
        }),
        &[],
        signer,
    )
    .unwrap();

    (authorization_address, processor_address)
}

pub fn wait_for_height(app: &NeutronTestApp, height: u64) {
    while (app.get_block_height() as u64) < height {
        // We can't increase blocks directly so we do it this way
        app.increase_time(1);
    }
}
