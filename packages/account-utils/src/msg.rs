use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    from_json, to_json_string, Attribute, Binary, CosmosMsg, Reply, StdError, StdResult, SubMsg,
    SubMsgResult,
};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

pub const VALENCE_CALLBACK_KEY: &str = "valence_callback";
pub const VALENCE_PAYLOAD_KEY: &str = "valence_payload";
pub const WASM_EVENT_TYPE: &str = "wasm";

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String, // Initial owner of the contract
    pub approved_services: Vec<String>,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    ApproveService {
        service: String,
    }, // Add service to approved list (only admin)
    RemoveService {
        service: String,
    }, // Remove service from approved list (only admin)
    ExecuteMsg {
        msgs: Vec<CosmosMsg>,
    }, // Execute any CosmosMsg (approved services or admin)
    ExecuteSubmsgs {
        msgs: Vec<SubMsg>,
        // json encoded
        payload: Option<String>,
    },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec<String>)]
    ListApprovedServices {}, // Get list of approved services
}

#[cw_serde]
pub struct ValenceCallback {
    pub id: u64,
    pub result: SubMsgResult,
    pub payload: Binary,
}

impl From<Reply> for ValenceCallback {
    fn from(value: Reply) -> Self {
        ValenceCallback {
            id: value.id,
            result: value.result,
            payload: value.payload,
        }
    }
}

impl TryInto<Attribute> for ValenceCallback {
    type Error = StdError;

    fn try_into(self) -> Result<Attribute, Self::Error> {
        let attr = Attribute {
            key: VALENCE_CALLBACK_KEY.to_string(),
            value: to_json_string(&self)?,
        };
        Ok(attr)
    }
}

impl TryFrom<SubMsgResult> for ValenceCallback {
    type Error = StdError;

    fn try_from(value: SubMsgResult) -> Result<Self, Self::Error> {
        let sub_result = value.into_result().map_err(StdError::generic_err)?;

        for event in sub_result.events {
            if event.ty == WASM_EVENT_TYPE {
                for attr in event.attributes {
                    if attr.key == VALENCE_CALLBACK_KEY {
                        let valence_callback: ValenceCallback = from_json(attr.value)?;
                        return Ok(valence_callback);
                    }
                }
            }
        }
        Err(StdError::generic_err("valence callback not found"))
    }
}

pub fn parse_valence_payload<T>(resp: &SubMsgResult) -> StdResult<T>
where
    T: serde::de::DeserializeOwned,
{
    if let Ok(sub_result) = resp.clone().into_result() {
        for event in sub_result.events {
            if event.ty == WASM_EVENT_TYPE {
                for attr in event.attributes {
                    if attr.key == VALENCE_PAYLOAD_KEY {
                        let valence_callback: StdResult<T> = from_json(&attr.value);
                        return valence_callback;
                    }
                }
            }
        }
    }
    Err(StdError::generic_err("valence payload not found"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{Event, SubMsgResponse};

    #[test]
    fn test_valence_callback_from_reply() {
        #[allow(deprecated)]
        let reply = Reply {
            id: 1,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(
                    b"CAISATAaBzEwMDAwMDAqHTE5OTk5NDk5OTg3NDk5NzM3NTUyMTg3MjczMzkzMJj4/////////wE="
                        .into(),
                ),
                msg_responses: vec![],
            }),
            payload: Binary::from(vec![1, 2, 3]),
            gas_used: 5555,
        };

        let callback: ValenceCallback = reply.into();

        assert_eq!(callback.id, 1);
        assert_eq!(callback.payload, Binary::from(vec![1, 2, 3]));
        assert!(!callback.payload.is_empty());
    }

    #[test]
    fn test_valence_callback_into_attribute() {
        #[allow(deprecated)]
        let callback = ValenceCallback {
            id: 1,
            result: SubMsgResult::Ok(SubMsgResponse {
                events: vec![],
                data: Some(
                    b"CAISATAaBzEwMDAwMDAqHTE5OTk5NDk5OTg3NDk5NzM3NTUyMTg3MjczMzkzMJj4/////////wE="
                        .into(),
                ),
                msg_responses: vec![],
            }),
            payload: Binary::from(vec![1, 2, 3]),
        };

        let attr: Attribute = callback.try_into().unwrap();

        assert_eq!(attr.key, VALENCE_CALLBACK_KEY);
        assert!(attr.value.contains("\"id\":1"));
        assert!(attr.value.contains("\"payload\":\"AQID\""));
    }

    #[test]
    fn test_valence_callback_try_from_submsg_result() {
        let resp = "{\"id\":314,\"result\":{\"ok\":{\"events\":[],\"data\":\"CAISATAaBzEwMDAwMDAqHTE5OTk5NDk5OTg3NDk5NzM3NTUyMTg3MjczMzkzMJj4/////////wE=\",\"msg_responses\":[]}},\"payload\":\"\"}";
        let event = Event::new(WASM_EVENT_TYPE).add_attribute(VALENCE_CALLBACK_KEY, resp);

        #[allow(deprecated)]
        let submsg_result = SubMsgResult::Ok(SubMsgResponse {
            events: vec![event],
            data: None,
            msg_responses: vec![],
        });

        let callback: ValenceCallback = submsg_result.try_into().unwrap();

        assert_eq!(callback.id, 314);
        assert!(callback.payload.is_empty());
    }

    #[test]
    fn test_parse_valence_payload() {
        #[cw_serde]
        struct TestPayload {
            value: String,
        }

        let event =
            Event::new(WASM_EVENT_TYPE).add_attribute(VALENCE_PAYLOAD_KEY, r#"{"value":"test"}"#);

        #[allow(deprecated)]
        let submsg_result = SubMsgResult::Ok(SubMsgResponse {
            events: vec![event],
            data: None,
            msg_responses: vec![],
        });

        let payload: TestPayload = parse_valence_payload(&submsg_result).unwrap();

        assert_eq!(
            payload,
            TestPayload {
                value: "test".to_string()
            }
        );
    }
}
