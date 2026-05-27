//! Placeholder — filled in TODO 1.
use crate::types::{AssetBlueprint, AssetIR};
use crate::validate::WacError;

pub fn compile(_bp: &AssetBlueprint) -> Result<AssetIR, WacError> {
    Err(WacError::InvalidSpec("compiler not yet implemented".into()))
}
