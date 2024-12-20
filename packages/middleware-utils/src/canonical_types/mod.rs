use crate::{type_registry::types::ValenceType, MiddlewareError};

pub mod bank;
pub mod pools;

pub trait ValenceTypeAdapter {
    type External;

    fn try_to_canonical(&self) -> Result<ValenceType, MiddlewareError>;
    fn try_from_canonical(canonical: ValenceType) -> Result<Self::External, MiddlewareError>;
}
