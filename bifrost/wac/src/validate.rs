//! Placeholder — filled in TODO 1.
use crate::types::AssetBlueprint;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WacError {
    #[error("seed must not be zero")]
    ZeroSeed,
    #[error("invalid spec: {0}")]
    InvalidSpec(String),
    #[error("constraint violated: {0}")]
    ConstraintViolated(String),
}

pub fn validate(_bp: &AssetBlueprint) -> Result<(), WacError> { Ok(()) }
