#[macro_export]
macro_rules! register_types {
    (
        $(
            $type_id:expr => {
                native_type: $native:ty,
                adapter: $adapter:ty,
                to_valence: $to_valence:ty,
            }
        ),*
    ) => {
        pub fn handle_query(msg: $crate::type_registry::types::RegistryQueryMsg)
            -> ::cosmwasm_std::StdResult<::cosmwasm_std::Binary>
        {
            use ::cosmwasm_std::{StdError, Binary, from_json, to_json_binary};
            use $crate::type_registry::types::{ValenceType, NativeTypeWrapper};
            use $crate::canonical_types::ValenceTypeAdapter;
            use $crate::IcqIntegration;
            use $crate::type_registry::types::RegistryQueryMsg;

            match msg {
                RegistryQueryMsg::ToCanonical { type_url, binary } => {
                    match type_url.as_str() {
                        $(
                            $type_id => {
                                let native: $native = from_json(&binary)?;
                                let adapter = <$adapter>::from(native);
                                let canonical = adapter.try_to_canonical()
                                    .map_err(|_| StdError::generic_err("failed to convert to canonical"))?;
                                to_json_binary(&canonical)
                            },
                        )*
                        _ => Err(StdError::generic_err("unknown type"))
                    }
                },
                RegistryQueryMsg::FromCanonical { obj } => {
                    $(
                        if let Ok(native) = <$adapter>::try_from_canonical(obj.clone()) {
                            return to_json_binary(&NativeTypeWrapper {
                                binary: to_json_binary(&native)?
                            });
                        }
                    )*
                    Err(StdError::generic_err("no matching type found for conversion"))
                },
                RegistryQueryMsg::KVKey { type_id, params } => {
                    match type_id.as_str() {
                        $(
                            $type_id => {
                                let kv_key = <$adapter>::get_kv_key(params)
                                    .map_err(|_| StdError::generic_err("failed to get kvkey"))?;
                                to_json_binary(&kv_key)
                            },
                        )*
                        _ => Err(::cosmwasm_std::StdError::generic_err("unknown type"))
                    }
                },
                RegistryQueryMsg::ReconstructProto { type_id, icq_result } => {
                    match type_id.as_str() {
                        $(
                            $type_id => {
                                let binary = <$adapter>::decode_and_reconstruct(type_id, icq_result)
                                    .map_err(|_| StdError::generic_err("failed to reconstruct type from proto"))?;
                                to_json_binary(&NativeTypeWrapper { binary })
                            },
                        )*
                        _ => Err(::cosmwasm_std::StdError::generic_err("unknown type"))
                    }
                }
            }
        }
    }
}
