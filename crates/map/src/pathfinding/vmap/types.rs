//! VMap types and constants

use oxcore_shared::protocol::Position;

/// VMap load result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VMapLoadResult {
    Ok,
    Error,
    Ignored,
}

/// Invalid height value (indicates no valid height found)
pub const VMAP_INVALID_HEIGHT_VALUE: f32 = -100000.0;

/// VMap configuration flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VMapConfig {
    pub enable_los: bool,
    pub enable_height: bool,
    pub enable_indoor_check: bool,
}

impl Default for VMapConfig {
    fn default() -> Self {
        Self {
            enable_los: true,
            enable_height: true,
            enable_indoor_check: true,
        }
    }
}

/// Area information from VMap query
#[derive(Debug, Clone)]
pub struct AreaInfo {
    pub z: f32,
    pub flags: u32,
    pub adt_id: i32,
    pub root_id: i32,
    pub group_id: i32,
}

/// Liquid level information
#[derive(Debug, Clone)]
pub struct LiquidLevel {
    pub level: f32,
    pub floor: f32,
    pub liquid_type: u32,
}

/// Liquid found inside a WMO volume.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WmoLiquidInfo {
    /// Z of the liquid surface.
    pub level: f32,
    /// Z of the WMO floor beneath the liquid.
    pub floor: f32,
    /// Raw WMO liquid type index (not a bitmask).
    pub liquid_type: u32,
    /// `MAP_LIQUID_TYPE_*` bitmask derived from `liquid_type`.
    pub type_flags: u32,
}

/// Convert a raw WMO liquid type index into a `MAP_LIQUID_TYPE_*` bitmask.
///
/// WMO groups store a liquid type *index*, not a flag, so it must be mapped
/// before it can be tested against a request mask. Ported from
/// `VMAP::GetLiquidMask`.
pub fn wmo_liquid_type_mask(liquid_type: u32) -> u32 {
    use crate::terrain::defines::*;

    match liquid_type {
        1 | 41 | 61 => MAP_LIQUID_TYPE_WATER,
        2 => MAP_LIQUID_TYPE_OCEAN,
        3 => MAP_LIQUID_TYPE_MAGMA,
        4 | 21 => MAP_LIQUID_TYPE_SLIME,
        _ => MAP_LIQUID_TYPE_NO_WATER,
    }
}

/// Model type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelType {
    /// WMO (World Model Object) - buildings, structures
    WMO,
    /// M2 (Model 2) - character/creature models
    M2,
    /// Unknown type
    Unknown,
}

/// Model instance in the world
#[derive(Debug, Clone)]
pub struct ModelInstance {
    pub model_id: u32,
    pub model_type: ModelType,
    pub position: Position,
    pub scale: f32,
    pub rotation: [f32; 3],
    /// Model name from spawn entry (used to load .vmo file)
    pub model_name: String,
}

/// Bounding box for spatial queries
#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub min: Position,
    pub max: Position,
}

impl BoundingBox {
    pub fn contains(&self, pos: &Position) -> bool {
        pos.x >= self.min.x
            && pos.x <= self.max.x
            && pos.y >= self.min.y
            && pos.y <= self.max.y
            && pos.z >= self.min.z
            && pos.z <= self.max.z
    }
}
