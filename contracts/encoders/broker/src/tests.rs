use std::collections::HashMap;

use cosmwasm_std::testing::{
    message_info, mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{from_json, Addr, OwnedDeps};
use cw_ownable::{Action, Ownership};

use crate::contract::{execute, instantiate, query};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

const OWNER: &str = "owner";
const ENCODER: &str = "encoder";
const NEW_OWNER: &str = "new_owner";

fn setup_contract() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies();
    let mut encoders = HashMap::new();
    let api = MockApi::default();
    let encoder_address = api.addr_make(ENCODER);
    let owner_address = api.addr_make(OWNER);

    encoders.insert("v1.0.0".to_string(), encoder_address.to_string());

    let msg = InstantiateMsg {
        owner: owner_address.to_string(),
        encoders,
    };
    let info = message_info(&owner_address, &[]);
    let env = mock_env();

    instantiate(deps.as_mut(), env, info, msg).unwrap();
    deps
}

#[test]
fn proper_instantiation() {
    let deps = setup_contract();
    let api = MockApi::default();

    // Test ownership
    let ownership: Ownership<Addr> =
        from_json(query(deps.as_ref(), mock_env(), QueryMsg::Ownership {}).unwrap()).unwrap();
    assert_eq!(ownership.owner.unwrap(), api.addr_make(OWNER));

    // Test encoder registration
    let encoders: Vec<(String, Addr)> =
        from_json(query(deps.as_ref(), mock_env(), QueryMsg::ListEncoders {}).unwrap()).unwrap();
    assert_eq!(encoders.len(), 1);
    assert_eq!(encoders[0].0, "v1.0.0");
    assert_eq!(encoders[0].1, MockApi::default().addr_make(ENCODER));
}

#[test]
fn register_encoder() {
    let mut deps = setup_contract();
    let api = MockApi::default();
    let new_encoder_address = api.addr_make("new_encoder");

    // Register new encoder
    let msg = ExecuteMsg::RegisterEncoder {
        version: "v2.0.0".to_string(),
        address: new_encoder_address.to_string(),
    };
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&api.addr_make(OWNER), &[]),
        msg,
    )
    .unwrap();
    assert_eq!(res.attributes.len(), 3);
    assert_eq!(res.attributes[0].key, "method");
    assert_eq!(res.attributes[0].value, "register_encoder");
    assert_eq!(res.attributes[1].key, "address");
    assert_eq!(res.attributes[1].value, new_encoder_address.to_string());
    assert_eq!(res.attributes[2].key, "version");
    assert_eq!(res.attributes[2].value, "v2.0.0");

    // Verify encoder was added
    let encoders: Vec<(String, Addr)> =
        from_json(query(deps.as_ref(), mock_env(), QueryMsg::ListEncoders {}).unwrap()).unwrap();
    assert_eq!(encoders.len(), 2);
}

#[test]
fn remove_encoder() {
    let mut deps = setup_contract();

    // Remove encoder
    let msg = ExecuteMsg::RemoveEncoder {
        version: "v1.0.0".to_string(),
    };
    let res = execute(
        deps.as_mut(),
        mock_env(),
        message_info(&MockApi::default().addr_make(OWNER), &[]),
        msg,
    )
    .unwrap();

    assert_eq!(res.attributes.len(), 2);
    assert_eq!(res.attributes[0].key, "method");
    assert_eq!(res.attributes[0].value, "remove_encoder");
    assert_eq!(res.attributes[1].key, "version");
    assert_eq!(res.attributes[1].value, "v1.0.0");

    // Verify encoder was removed
    let encoders: Vec<(String, Addr)> =
        from_json(query(deps.as_ref(), mock_env(), QueryMsg::ListEncoders {}).unwrap()).unwrap();
    assert_eq!(encoders.len(), 0);
}

#[test]
fn get_encoder() {
    let deps = setup_contract();

    // Test existing encoder
    let encoder: String = from_json(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Encoder {
                version: "v1.0.0".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(encoder, MockApi::default().addr_make(ENCODER).to_string());

    // Test non-existent encoder
    let encoder: String = from_json(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Encoder {
                version: "non-existent".to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(encoder, "");
}

#[test]
fn update_ownership() {
    let mut deps = setup_contract();
    let api = MockApi::default();
    let owner = api.addr_make(OWNER);
    let new_owner = api.addr_make(NEW_OWNER);

    // Transfer ownership
    let msg = ExecuteMsg::UpdateOwnership(Action::TransferOwnership {
        new_owner: new_owner.to_string(),
        expiry: None,
    });
    execute(deps.as_mut(), mock_env(), message_info(&owner, &[]), msg).unwrap();

    // Query ownership
    let ownership: Ownership<Addr> =
        from_json(query(deps.as_ref(), mock_env(), QueryMsg::Ownership {}).unwrap()).unwrap();
    assert_eq!(ownership.owner.unwrap(), owner);
    assert_eq!(ownership.pending_owner.unwrap(), new_owner);

    // Accept ownership
    let msg = ExecuteMsg::UpdateOwnership(Action::AcceptOwnership);
    execute(
        deps.as_mut(),
        mock_env(),
        message_info(&new_owner, &[]),
        msg,
    )
    .unwrap();

    // Verify new ownership
    let ownership: Ownership<Addr> =
        from_json(query(deps.as_ref(), mock_env(), QueryMsg::Ownership {}).unwrap()).unwrap();
    assert_eq!(ownership.owner.unwrap(), new_owner);
    assert!(ownership.pending_owner.is_none());
}

#[test]
fn unauthorized_encoder_operations() {
    let mut deps = setup_contract();
    let unauthorized_info = message_info(&Addr::unchecked("unauthorized"), &[]);

    // Try to register encoder
    let msg = ExecuteMsg::RegisterEncoder {
        version: "v2.0.0".to_string(),
        address: "new_encoder".to_string(),
    };
    let err = execute(deps.as_mut(), mock_env(), unauthorized_info.clone(), msg).unwrap_err();
    assert_eq!(
        err.to_string(),
        cw_ownable::OwnershipError::NotOwner.to_string()
    );

    // Try to remove encoder
    let msg = ExecuteMsg::RemoveEncoder {
        version: "v1.0.0".to_string(),
    };
    let err = execute(deps.as_mut(), mock_env(), unauthorized_info, msg).unwrap_err();
    assert_eq!(
        err.to_string(),
        cw_ownable::OwnershipError::NotOwner.to_string()
    );
}
