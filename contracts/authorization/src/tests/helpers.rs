use margined_neutron_std::types::{
    cosmos::base::v1beta1::Coin,
    cosmwasm::wasm::v1::{
        MsgInstantiateContract2, MsgInstantiateContract2Response, QueryBuildAddressRequest,
        QueryBuildAddressResponse,
    },
};
use neutron_test_tube::{
    Account, EncodeError, Module, NeutronTestApp, Runner, RunnerExecuteResult, RunnerResult,
    SigningAccount, Wasm,
};
use serde::Serialize;
use valence_authorization_utils::{domain::ExternalDomain, msg::InstantiateMsg};
use valence_processor_utils::msg::InstantiateMsg as ProcessorInstantiateMsg;
use valence_test_service::msg::InstantiateMsg as TestServiceInstantiateMsg;

pub const ARTIFACTS_DIR: &str = "../../artifacts";

pub struct ExtendedWasm<'a, R: Runner<'a>> {
    runner: &'a R,
}

impl<'a, R: Runner<'a>> Module<'a, R> for ExtendedWasm<'a, R> {
    fn new(runner: &'a R) -> Self {
        ExtendedWasm { runner }
    }
}

impl<'a, R> ExtendedWasm<'a, R>
where
    R: Runner<'a>,
{
    #[allow(clippy::too_many_arguments)]
    pub fn instantiate2<M>(
        &self,
        code_id: u64,
        msg: &M,
        admin: Option<&str>,
        label: Option<&str>,
        funds: &[Coin],
        salt: String,
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgInstantiateContract2Response>
    where
        M: ?Sized + Serialize,
    {
        self.runner.execute(
            MsgInstantiateContract2 {
                sender: signer.address(),
                admin: admin.unwrap_or_default().to_string(),
                code_id,
                label: label.unwrap_or(" ").to_string(), // empty string causes panic
                msg: serde_json::to_vec(msg).map_err(EncodeError::JsonEncodeError)?,
                funds: funds
                    .iter()
                    .map(|c| Coin {
                        denom: c.denom.parse().unwrap(),
                        amount: c.amount.to_string(),
                    })
                    .collect(),
                salt: hex::decode(salt).unwrap(),
                fix_msg: false,
            },
            "/cosmwasm.wasm.v1.MsgInstantiateContract2",
            signer,
        )
    }

    pub fn query_build_address(
        &self,
        code_hash: String,
        creator_address: String,
        salt: String,
    ) -> RunnerResult<String> {
        let res = self
            .runner
            .query::<QueryBuildAddressRequest, QueryBuildAddressResponse>(
                "/cosmwasm.wasm.v1.Query/BuildAddress",
                &QueryBuildAddressRequest {
                    code_hash,
                    creator_address,
                    salt,
                    init_args: vec![],
                },
            )?;

        Ok(res.address)
    }
}

pub fn store_and_instantiate_authorization_with_processor_contract(
    app: &NeutronTestApp,
    signer: &SigningAccount,
    owner: String,
    sub_owners: Vec<String>,
    external_domains: Vec<ExternalDomain>,
) -> (String, String) {
    let wasm = Wasm::new(app);
    let extended_wasm = ExtendedWasm::new(app);

    let wasm_byte_code_authorization =
        std::fs::read(format!("{}/valence_authorization.wasm", ARTIFACTS_DIR)).unwrap();
    let wasm_byte_code_processor =
        std::fs::read(format!("{}/valence_processor.wasm", ARTIFACTS_DIR)).unwrap();

    let code_response = wasm
        .store_code(&wasm_byte_code_authorization, None, signer)
        .unwrap()
        .data;
    let code_id_authorization = code_response.code_id;
    let checksum = code_response.checksum;

    let code_id_processor = wasm
        .store_code(&wasm_byte_code_processor, None, signer)
        .unwrap()
        .data
        .code_id;

    let salt = hex::encode("authorization");
    let predicted_address = extended_wasm
        .query_build_address(
            hex::encode(&checksum),
            signer.address().to_string(),
            salt.clone(),
        )
        .unwrap();

    let processor_address = wasm
        .instantiate(
            code_id_processor,
            &ProcessorInstantiateMsg {
                owner: signer.address().to_string(),
                authorization_contract: predicted_address.clone(),
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

    let authorization_address = extended_wasm
        .instantiate2(
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
            salt,
            signer,
        )
        .unwrap()
        .data
        .address;

    assert_eq!(predicted_address, authorization_address);

    (authorization_address, processor_address)
}

pub fn store_and_instantiate_test_service(
    wasm: &Wasm<'_, NeutronTestApp>,
    signer: &SigningAccount,
    admin: Option<&str>,
) -> String {
    let wasm_byte_code =
        std::fs::read(format!("{}/valence_test_service.wasm", ARTIFACTS_DIR)).unwrap();

    let code_id = wasm
        .store_code(&wasm_byte_code, None, signer)
        .unwrap()
        .data
        .code_id;

    wasm.instantiate(
        code_id,
        &TestServiceInstantiateMsg {},
        admin,
        "test_service".into(),
        &[],
        signer,
    )
    .unwrap()
    .data
    .address
}

pub fn wait_for_height(app: &NeutronTestApp, height: u64) {
    while (app.get_block_height() as u64) < height {
        // We can't increase blocks directly so we do it this way
        app.increase_time(1);
    }
}
