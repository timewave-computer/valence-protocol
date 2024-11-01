use std::vec;

use crate::msg::{ComparisonOperator, QueryInstruction};
use cosmwasm_std::{
    coin, to_json_binary, Addr, Binary, Coin, CosmosMsg, Decimal, Empty, Uint128, WasmMsg,
};
use cw_multi_test::{error::AnyResult, App, AppResponse, ContractWrapper, Executor};
use getset::{Getters, Setters};
use valence_authorization_utils::{
    callback::ExecutionResult,
    msg::{ExecuteMsg, InternalAuthorizationMsg},
};

const NTRN: &str = "untrn";
const ONE_THOUSAND: u128 = 1_000_000_000_u128;
const ONE_MILLION: u128 = 1_000_000_000_000_u128;

#[derive(Getters, Setters)]
pub struct TestSuite {
    #[getset(get, get)]
    app: App,
    #[getset(get)]
    owner: Addr,
    #[getset(get)]
    code_id: u64,
}

impl TestSuite {
    pub fn new() -> Self {
        let mut app = App::default();
        let owner = app.api().addr_make("owner");
        let code = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        let code_id = app.store_code(Box::new(code));
        Self {
            app,
            owner,
            code_id,
        }
    }

    pub fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }

    fn init_balance(&mut self, addr: &Addr, amounts: Vec<Coin>) {
        self.app_mut().init_modules(|router, _, store| {
            router.bank.init_balance(store, addr, amounts).unwrap();
        });
    }

    pub fn instantiate(&mut self) -> Addr {
        let msg = crate::msg::InstantiateMsg {
            owner: self.owner.to_string(),
        };

        self.app
            .instantiate_contract(
                self.code_id,
                self.owner.clone(),
                &msg,
                &[],
                "condition-branch",
                Some(self.owner.to_string()),
            )
            .unwrap()
    }

    pub fn dyn_ratio_contract_init(&mut self, denom: &str, ratio: Decimal) -> Addr {
        let dyn_ratio_code = ContractWrapper::new(
            valence_test_dynamic_ratio::contract::execute,
            valence_test_dynamic_ratio::contract::instantiate,
            valence_test_dynamic_ratio::contract::query,
        );

        let dyn_ratio_code_id = self.app_mut().store_code(Box::new(dyn_ratio_code));

        let init_msg = valence_test_dynamic_ratio::msg::InstantiateMsg {
            denom_ratios: [(denom.to_string(), ratio)].into(),
        };

        self.app
            .instantiate_contract(
                dyn_ratio_code_id,
                self.owner.clone(),
                &init_msg,
                &[],
                "dynamic-ratio",
                Some(self.owner.to_string()),
            )
            .unwrap()
    }

    pub fn execute(
        &mut self,
        addr: &Addr,
        query: QueryInstruction,
        operator: ComparisonOperator,
        rhs_operand: Binary,
        true_branch: Option<Binary>,
        false_branch: Option<Binary>,
    ) -> AnyResult<AppResponse> {
        let msg = crate::msg::ExecuteMsg::CompareAndBranch {
            query,
            operator,
            rhs_operand,
            true_branch,
            false_branch,
        };
        let res = self
            .app
            .execute_contract(self.owner().clone(), addr.clone(), &msg, &[])?;
        Ok(res)
    }
}

#[test]
fn instantiate_works() {
    let mut suite = TestSuite::new();
    suite.instantiate();
}

#[test]
fn balance_checks_succeed() {
    let mut suite = TestSuite::new();
    let addr = suite.instantiate();

    let authz = suite.app().api().addr_make("authorization");

    let input = suite.app().api().addr_make("input");
    suite.init_balance(&input, vec![coin(ONE_MILLION, NTRN)]);

    let query = QueryInstruction::BalanceQuery {
        address: input.to_string(),
        denom: NTRN.to_owned(),
    };

    let cb_msg: CosmosMsg<Empty> = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: authz.to_string(),
        msg: to_json_binary(&ExecuteMsg::InternalAuthorizationAction(
            InternalAuthorizationMsg::ProcessorCallback {
                execution_id: 1234u64,
                execution_result: ExecutionResult::Success,
            },
        ))
        .unwrap(),
        funds: vec![],
    });

    // Greater than 1 thousand
    suite
        .execute(
            &addr,
            query.clone(),
            ComparisonOperator::GreaterThan,
            to_json_binary(&Uint128::from(ONE_THOUSAND)).unwrap(),
            Some(to_json_binary(&cb_msg).unwrap()),
            None,
        )
        .unwrap();

    // Equal to 1 million
    suite
        .execute(
            &addr,
            query.clone(),
            ComparisonOperator::Equal,
            to_json_binary(&Uint128::from(ONE_MILLION)).unwrap(),
            None,
            None,
        )
        .unwrap();

    // Not equal to 1 thousand
    suite
        .execute(
            &addr,
            query.clone(),
            ComparisonOperator::NotEqual,
            to_json_binary(&Uint128::from(ONE_THOUSAND)).unwrap(),
            None,
            None,
        )
        .unwrap();

    // Greater than or equal to 1 million
    suite
        .execute(
            &addr,
            query.clone(),
            ComparisonOperator::GreaterThanOrEqual,
            to_json_binary(&Uint128::from(ONE_MILLION)).unwrap(),
            None,
            None,
        )
        .unwrap();

    // Less than or equal to 1 million
    suite
        .execute(
            &addr,
            query.clone(),
            ComparisonOperator::LessThanOrEqual,
            to_json_binary(&Uint128::from(ONE_MILLION)).unwrap(),
            None,
            None,
        )
        .unwrap();

    // Less than 2 millions
    suite
        .execute(
            &addr,
            query,
            ComparisonOperator::LessThan,
            to_json_binary(&Uint128::from(ONE_MILLION * 2)).unwrap(),
            None,
            None,
        )
        .unwrap();
}

#[test]
fn balance_checks_fail() {
    let mut suite = TestSuite::new();
    let addr = suite.instantiate();

    let input = suite.app().api().addr_make("input");
    suite.init_balance(&input, vec![coin(ONE_THOUSAND, NTRN)]);

    let query = QueryInstruction::BalanceQuery {
        address: input.to_string(),
        denom: NTRN.to_owned(),
    };

    // Greater than 1 thousand
    assert!(suite
        .execute(
            &addr,
            query.clone(),
            ComparisonOperator::GreaterThan,
            to_json_binary(&Uint128::from(ONE_THOUSAND)).unwrap(),
            None,
            None,
        )
        .is_err());

    // Equal to 1 million
    assert!(suite
        .execute(
            &addr,
            query.clone(),
            ComparisonOperator::Equal,
            to_json_binary(&Uint128::from(ONE_MILLION)).unwrap(),
            None,
            None,
        )
        .is_err());

    // Not equal to 1 thousand
    assert!(suite
        .execute(
            &addr,
            query.clone(),
            ComparisonOperator::NotEqual,
            to_json_binary(&Uint128::from(ONE_THOUSAND)).unwrap(),
            None,
            None,
        )
        .is_err());

    // Greater than or equal to 1 million
    assert!(suite
        .execute(
            &addr,
            query.clone(),
            ComparisonOperator::GreaterThanOrEqual,
            to_json_binary(&Uint128::from(ONE_MILLION)).unwrap(),
            None,
            None,
        )
        .is_err());

    // Less than or equal to 10
    assert!(suite
        .execute(
            &addr,
            query.clone(),
            ComparisonOperator::LessThanOrEqual,
            to_json_binary(&Uint128::from(10u128)).unwrap(),
            None,
            None,
        )
        .is_err());

    // Less than 1 thousand
    assert!(suite
        .execute(
            &addr,
            query,
            ComparisonOperator::LessThan,
            to_json_binary(&Uint128::from(ONE_THOUSAND)).unwrap(),
            None,
            None,
        )
        .is_err());
}

#[test]
fn wasm_queries_succeed() {
    let mut suite = TestSuite::new();
    let addr = suite.instantiate();

    let dyn_ratio_contract = suite.dyn_ratio_contract_init(NTRN, Decimal::percent(50));

    let input = suite.app().api().addr_make("input");
    suite.init_balance(&input, vec![coin(ONE_MILLION, NTRN)]);

    let query_msg = to_json_binary(
        &valence_service_utils::msg::DynamicRatioQueryMsg::DynamicRatio {
            denoms: vec![NTRN.to_owned()],
            params: "".to_string(),
        },
    )
    .unwrap();
    let wasm_query = QueryInstruction::WasmQuery {
        contract_addr: dyn_ratio_contract.to_string(),
        msg: query_msg,
        value_path: vec!["denom_ratios".to_string(), NTRN.to_string()],
    };

    // Greater than 20 percent
    suite
        .execute(
            &addr,
            wasm_query.clone(),
            ComparisonOperator::GreaterThan,
            to_json_binary(&Decimal::percent(20)).unwrap(),
            None,
            None,
        )
        .unwrap();

    // Equal to 50 percent
    suite
        .execute(
            &addr,
            wasm_query.clone(),
            ComparisonOperator::Equal,
            to_json_binary(&Decimal::percent(50)).unwrap(),
            None,
            None,
        )
        .unwrap();

    // Not equal to 80 percent
    suite
        .execute(
            &addr,
            wasm_query.clone(),
            ComparisonOperator::NotEqual,
            to_json_binary(&Decimal::percent(80)).unwrap(),
            None,
            None,
        )
        .unwrap();

    // Greater than or equal to 50 percent
    suite
        .execute(
            &addr,
            wasm_query.clone(),
            ComparisonOperator::GreaterThanOrEqual,
            to_json_binary(&Decimal::percent(50)).unwrap(),
            None,
            None,
        )
        .unwrap();

    // Less than or equal to 60 percent
    suite
        .execute(
            &addr,
            wasm_query.clone(),
            ComparisonOperator::LessThanOrEqual,
            to_json_binary(&Decimal::percent(60)).unwrap(),
            None,
            None,
        )
        .unwrap();

    // Less than 60 percent
    suite
        .execute(
            &addr,
            wasm_query,
            ComparisonOperator::LessThan,
            to_json_binary(&Decimal::percent(60)).unwrap(),
            None,
            None,
        )
        .unwrap();
}

#[test]
fn wasm_queries_fail() {
    let mut suite = TestSuite::new();
    let addr = suite.instantiate();

    let dyn_ratio_contract = suite.dyn_ratio_contract_init(NTRN, Decimal::percent(50));

    let input = suite.app().api().addr_make("input");
    suite.init_balance(&input, vec![coin(ONE_MILLION, NTRN)]);

    let query_msg = to_json_binary(
        &valence_service_utils::msg::DynamicRatioQueryMsg::DynamicRatio {
            denoms: vec![NTRN.to_owned()],
            params: "".to_string(),
        },
    )
    .unwrap();
    let wasm_query = QueryInstruction::WasmQuery {
        contract_addr: dyn_ratio_contract.to_string(),
        msg: query_msg,
        value_path: vec!["denom_ratios".to_string(), NTRN.to_string()],
    };

    // Greater than 50 percent
    assert!(suite
        .execute(
            &addr,
            wasm_query.clone(),
            ComparisonOperator::GreaterThan,
            to_json_binary(&Decimal::percent(50)).unwrap(),
            None,
            None,
        )
        .is_err());

    // Equal to 60 percent
    assert!(suite
        .execute(
            &addr,
            wasm_query.clone(),
            ComparisonOperator::Equal,
            to_json_binary(&Decimal::percent(60)).unwrap(),
            None,
            None,
        )
        .is_err());

    // Not equal to 50 percent
    assert!(suite
        .execute(
            &addr,
            wasm_query.clone(),
            ComparisonOperator::NotEqual,
            to_json_binary(&Decimal::percent(50)).unwrap(),
            None,
            None,
        )
        .is_err());

    // Greater than or equal to 60 percent
    assert!(suite
        .execute(
            &addr,
            wasm_query.clone(),
            ComparisonOperator::GreaterThanOrEqual,
            to_json_binary(&Decimal::percent(60)).unwrap(),
            None,
            None,
        )
        .is_err());

    // Less than or equal to 40 percent
    assert!(suite
        .execute(
            &addr,
            wasm_query.clone(),
            ComparisonOperator::LessThanOrEqual,
            to_json_binary(&Decimal::percent(40)).unwrap(),
            None,
            None,
        )
        .is_err());

    // Less than 50 percent
    assert!(suite
        .execute(
            &addr,
            wasm_query,
            ComparisonOperator::LessThan,
            to_json_binary(&Decimal::percent(50)).unwrap(),
            None,
            None,
        )
        .is_err());
}
