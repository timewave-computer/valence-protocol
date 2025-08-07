use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
use cosmwasm_std::{from_json, Addr, Binary};
use cw_ownable::Ownership;
use std::collections::HashMap;

// Helper function to create a default instantiate message
fn default_instantiate_msg(owner: Addr, verifier1: Addr) -> InstantiateMsg {
    let mut initial_routes = HashMap::new();

    initial_routes.insert("route1".to_string(), verifier1.to_string());

    InstantiateMsg {
        owner: owner.to_string(),
        initial_routes,
    }
}

#[test]
fn test_instantiate_success() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&deps.api.addr_make("creator"), &[]);
    let owner = deps.api.addr_make("owner");
    let verifier1 = deps.api.addr_make("verifier1");
    let msg = default_instantiate_msg(owner, verifier1.clone());

    instantiate(deps.as_mut(), env, info, msg.clone()).unwrap();
    // Verify initial routes were saved
    let route1 = from_json::<Addr>(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetRoute {
                name: "route1".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(route1, verifier1);
}

#[test]
fn test_instantiate_empty_routes() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&deps.api.addr_make("creator"), &[]);
    let msg = InstantiateMsg {
        owner: deps.api.addr_make("owner").to_string(),
        initial_routes: HashMap::new(),
    };

    instantiate(deps.as_mut(), env, info, msg).unwrap();

    // Query routes should return empty list
    let routes = from_json::<Vec<(String, Addr)>>(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetRoutes {
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(routes.len(), 0);
}

#[test]
fn test_add_route_success() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let owner = deps.api.addr_make("owner");
    let info = message_info(&deps.api.addr_make("creator"), &[]);

    // Instantiate contract
    let instantiate_msg = InstantiateMsg {
        owner: owner.to_string(),
        initial_routes: HashMap::new(),
    };
    instantiate(deps.as_mut(), env.clone(), info, instantiate_msg).unwrap();

    // Add a new route as owner
    let owner_info = message_info(&owner, &[]);
    let new_verifier = deps.api.addr_make("new_verifier");
    let add_msg = ExecuteMsg::AddRoute {
        name: "new_route".to_string(),
        address: new_verifier.to_string(),
    };

    execute(deps.as_mut(), env, owner_info, add_msg).unwrap();

    // Verify route was added
    let route = from_json::<Addr>(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetRoute {
                name: "new_route".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(route, new_verifier);
}

#[test]
fn test_add_route_unauthorized() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&deps.api.addr_make("creator"), &[]);

    // Instantiate contract
    let instantiate_msg = InstantiateMsg {
        owner: deps.api.addr_make("owner").to_string(),
        initial_routes: HashMap::new(),
    };
    instantiate(deps.as_mut(), env.clone(), info, instantiate_msg).unwrap();

    // Try to add route as non-owner
    let non_owner_info = message_info(&deps.api.addr_make("non_owner"), &[]);
    let add_msg = ExecuteMsg::AddRoute {
        name: "new_route".to_string(),
        address: deps.api.addr_make("new_verifier").to_string(),
    };

    let err = execute(deps.as_mut(), env, non_owner_info, add_msg).unwrap_err();

    // Should get ownership error
    assert!(matches!(err, ContractError::Ownership(_)));
}

#[test]
fn test_add_route_already_exists() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&deps.api.addr_make("creator"), &[]);
    let owner = deps.api.addr_make("owner");

    // Instantiate contract with existing route
    let mut initial_routes = HashMap::new();
    let verifier1 = deps.api.addr_make("verifier1");
    initial_routes.insert("existing_route".to_string(), verifier1.to_string());

    let instantiate_msg = InstantiateMsg {
        owner: owner.to_string(),
        initial_routes,
    };
    instantiate(deps.as_mut(), env.clone(), info, instantiate_msg).unwrap();

    // Try to add route with same name
    let owner_info = message_info(&owner, &[]);
    let add_msg = ExecuteMsg::AddRoute {
        name: "existing_route".to_string(),
        address: deps.api.addr_make("new_verifier").to_string(),
    };

    let err = execute(deps.as_mut(), env, owner_info, add_msg).unwrap_err();
    assert!(matches!(err, ContractError::RouteAlreadyExists {}));
}

#[test]
fn test_get_route_not_found() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&deps.api.addr_make("creator"), &[]);

    // Instantiate contract with no routes
    let instantiate_msg = InstantiateMsg {
        owner: deps.api.addr_make("owner").to_string(),
        initial_routes: HashMap::new(),
    };
    instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

    // Query non-existent route
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetRoute {
            name: "nonexistent".to_string(),
        },
    );

    // Should return error
    assert!(res.is_err());
}

#[test]
fn test_get_routes_pagination() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&deps.api.addr_make("creator"), &[]);

    // Create multiple initial routes
    let mut initial_routes = HashMap::new();
    for i in 0..5 {
        let verifier = deps.api.addr_make(&format!("verifier{i}"));
        initial_routes.insert(format!("route{i}"), verifier.to_string());
    }

    let instantiate_msg = InstantiateMsg {
        owner: deps.api.addr_make("owner").to_string(),
        initial_routes,
    };
    instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

    // Test getting all routes
    let all_routes = from_json::<Vec<(String, Addr)>>(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetRoutes {
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(all_routes.len(), 5);

    // Test pagination with limit
    let limited_routes = from_json::<Vec<(String, Addr)>>(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetRoutes {
                start_after: None,
                limit: Some(2),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(limited_routes.len(), 2);

    // Test pagination with start_after
    let paginated_routes = from_json::<Vec<(String, Addr)>>(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetRoutes {
                start_after: Some("route1".to_string()),
                limit: Some(2),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(paginated_routes.len(), 2);
}

#[test]
fn test_ownership_query() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&deps.api.addr_make("creator"), &[]);
    let test_owner = deps.api.addr_make("test_owner");

    let instantiate_msg = InstantiateMsg {
        owner: test_owner.to_string(),
        initial_routes: HashMap::new(),
    };
    instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

    let ownership: Ownership<Addr> =
        from_json(query(deps.as_ref(), mock_env(), QueryMsg::Ownership {}).unwrap()).unwrap();

    assert_eq!(ownership.owner, Some(test_owner));
    assert_eq!(ownership.pending_owner, None);
    assert_eq!(ownership.pending_expiry, None);
}

#[test]
fn test_update_ownership() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&deps.api.addr_make("creator"), &[]);

    let owner = deps.api.addr_make("owner");
    let instantiate_msg = InstantiateMsg {
        owner: owner.to_string(),
        initial_routes: HashMap::new(),
    };
    instantiate(deps.as_mut(), env.clone(), info, instantiate_msg).unwrap();

    // Transfer ownership
    let owner_info = message_info(&owner, &[]);
    let new_owner = deps.api.addr_make("new_owner");
    let transfer_msg = ExecuteMsg::UpdateOwnership(cw_ownable::Action::TransferOwnership {
        new_owner: new_owner.to_string(),
        expiry: None,
    });

    let res = execute(deps.as_mut(), env, owner_info, transfer_msg).unwrap();
    assert!(!res.attributes.is_empty());

    // Check that ownership is now pending
    let ownership: Ownership<Addr> =
        from_json(query(deps.as_ref(), mock_env(), QueryMsg::Ownership {}).unwrap()).unwrap();

    assert_eq!(ownership.owner, Some(owner));
    assert_eq!(ownership.pending_owner, Some(new_owner));
}

#[test]
fn test_verify_query_api() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&deps.api.addr_make("creator"), &[]);

    // Instantiate contract with no routes
    let instantiate_msg = InstantiateMsg {
        owner: deps.api.addr_make("owner").to_string(),
        initial_routes: HashMap::new(),
    };
    instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

    // Try to verify, which should fail because route is not set
    query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Verify {
            route: "route66".to_string(),
            vk: Binary::default(),
            inputs: Binary::default(),
            proof: Binary::default(),
            payload: Binary::default(),
        },
    )
    .unwrap_err();
}
