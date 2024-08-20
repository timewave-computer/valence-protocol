use cosmwasm_std::Addr;
use neutron_test_tube::{NeutronTestApp, SigningAccount, Wasm};
use valence_authorization_utils::domain::ExternalDomain;

use crate::msg::InstantiateMsg;

pub fn store_and_instantiate_authorization_contract(
    wasm: &Wasm<'_, NeutronTestApp>,
    signer: &SigningAccount,
    owner: Option<Addr>,
    sub_owners: Vec<Addr>,
    processor: Addr,
    external_domains: Vec<ExternalDomain>,
) -> String {
    let wasm_byte_code = std::fs::read("../../artifacts/valence_authorization.wasm").unwrap();
    let code_id = wasm
        .store_code(&wasm_byte_code, None, signer)
        .unwrap()
        .data
        .code_id;
    wasm.instantiate(
        code_id,
        &InstantiateMsg {
            owner,
            sub_owners,
            processor,
            external_domains,
        },
        None,
        "authorization".into(),
        &[],
        signer,
    )
    .unwrap()
    .data
    .address
}
