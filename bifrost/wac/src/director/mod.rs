//! Placeholder — filled in TODO 2.
use crate::types::AssetBlueprint;
use crate::pressure::PressureGraph;

#[derive(Debug, Default)]
pub struct WorldDirector;

impl WorldDirector {
    pub fn new() -> Self { Self }
    pub fn tick(&mut self, _pressure: &PressureGraph) -> Vec<AssetBlueprint> { vec![] }
}
