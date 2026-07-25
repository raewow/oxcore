//! Terrain file format constants and liquid types.
//!
//! Ported from MaNGOS `GridMapDefines.h`, `GridDefines.h` and `GridMap.h`.

/// Side length of one grid in world units.
pub const SIZE_OF_GRIDS: f32 = 533.33333;
/// Number of grids along one map axis.
pub const MAX_NUMBER_OF_GRIDS: usize = 64;
/// Grid index of the map origin.
pub const CENTER_GRID_ID: i32 = (MAX_NUMBER_OF_GRIDS / 2) as i32;
/// Height/liquid sample resolution per grid.
pub const MAP_RESOLUTION: i32 = 128;

/// Sentinel used when *checking* whether a height lookup succeeded.
pub const INVALID_HEIGHT: f32 = -100000.0;
/// Sentinel *returned* when no height could be determined.
pub const INVALID_HEIGHT_VALUE: f32 = -200000.0;

/// Default search distance for ground height lookups.
pub const DEFAULT_HEIGHT_SEARCH: f32 = 10.0;
/// Default search distance when locating a water surface.
pub const DEFAULT_WATER_SEARCH: f32 = 50.0;

// ---- File magics ----
pub(crate) const MAP_MAGIC: &[u8; 4] = b"MAPS";
pub(crate) const MAP_VERSION_MAGIC: &[u8; 4] = b"z1.4";
pub(crate) const MAP_AREA_MAGIC: &[u8; 4] = b"AREA";
pub(crate) const MAP_HEIGHT_MAGIC: &[u8; 4] = b"MHGT";
pub(crate) const MAP_LIQUID_MAGIC: &[u8; 4] = b"MLIQ";

// ---- Section flags ----
pub(crate) const MAP_AREA_NO_AREA: u16 = 0x0001;
pub(crate) const MAP_HEIGHT_NO_HEIGHT: u32 = 0x0001;
pub(crate) const MAP_HEIGHT_AS_INT16: u32 = 0x0002;
pub(crate) const MAP_HEIGHT_AS_INT8: u32 = 0x0004;
pub(crate) const MAP_LIQUID_NO_TYPE: u8 = 0x01;
pub(crate) const MAP_LIQUID_NO_HEIGHT: u8 = 0x02;

// ---- Liquid type flags (from LiquidType.dbc, left-shifted for flag usage) ----
pub const MAP_LIQUID_TYPE_NO_WATER: u32 = 0x00;
pub const MAP_LIQUID_TYPE_MAGMA: u32 = 0x01;
pub const MAP_LIQUID_TYPE_OCEAN: u32 = 0x02;
pub const MAP_LIQUID_TYPE_SLIME: u32 = 0x04;
pub const MAP_LIQUID_TYPE_WATER: u32 = 0x08;
/// Not a liquid kind: marks the cell as fatigue-inducing deep sea.
pub const MAP_LIQUID_TYPE_DEEP_WATER: u32 = 0x10;
/// Not a liquid kind: marks liquid sourced from a WMO volume rather than the ADT.
pub const MAP_LIQUID_TYPE_WMO_WATER: u32 = 0x20;

/// Every liquid kind a query can ask for.
pub const MAP_ALL_LIQUIDS: u32 =
    MAP_LIQUID_TYPE_WATER | MAP_LIQUID_TYPE_MAGMA | MAP_LIQUID_TYPE_OCEAN | MAP_LIQUID_TYPE_SLIME;

/// Where a position sits relative to a liquid surface.
///
/// A bitmask rather than an enum because callers test membership
/// (`status.intersects(IN_WATER | UNDER_WATER)`), matching `GridMapLiquidStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LiquidStatusFlags(pub u32);

impl LiquidStatusFlags {
    pub const NO_WATER: Self = Self(0x00);
    pub const ABOVE_WATER: Self = Self(0x01);
    pub const WATER_WALK: Self = Self(0x02);
    pub const IN_WATER: Self = Self(0x04);
    pub const UNDER_WATER: Self = Self(0x08);

    /// On or below the surface — the player is swimming.
    pub const MASK_SWIMMING: Self = Self(Self::IN_WATER.0 | Self::UNDER_WATER.0);
    /// On, below, or just barely above the surface.
    pub const MASK_TOUCHING: Self = Self(Self::MASK_SWIMMING.0 | Self::WATER_WALK.0);

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }
}

/// Liquid found at a position: kind, surface height, and floor height.
///
/// Mirrors `GridMapLiquidData`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiquidData {
    /// `MAP_LIQUID_TYPE_*` bitmask describing the liquid kind.
    pub type_flags: u32,
    /// `LiquidType.dbc` entry, or the raw ADT liquid type when unresolved.
    pub entry: u32,
    /// Z of the liquid surface.
    pub level: f32,
    /// Z of the ground beneath the liquid.
    pub depth_level: f32,
}

impl Default for LiquidData {
    fn default() -> Self {
        Self {
            type_flags: 0,
            entry: 0,
            level: INVALID_HEIGHT_VALUE,
            depth_level: INVALID_HEIGHT_VALUE,
        }
    }
}

impl LiquidData {
    /// Depth of the liquid column (surface minus floor), clamped at zero.
    pub fn depth(&self) -> f32 {
        (self.level - self.depth_level).max(0.0)
    }

    pub fn is_magma(&self) -> bool {
        (self.type_flags & MAP_LIQUID_TYPE_MAGMA) != 0
    }

    pub fn is_slime(&self) -> bool {
        (self.type_flags & MAP_LIQUID_TYPE_SLIME) != 0
    }

    /// Plain water or ocean — the kinds that make a player swim rather than burn.
    pub fn is_water(&self) -> bool {
        (self.type_flags & (MAP_LIQUID_TYPE_WATER | MAP_LIQUID_TYPE_OCEAN)) != 0
    }

    /// Deep sea, which drains the fatigue timer.
    pub fn is_deep_water(&self) -> bool {
        (self.type_flags & MAP_LIQUID_TYPE_DEEP_WATER) != 0
    }
}

/// Convert world coordinates to the terrain grid indices used by the `.map` files.
///
/// The axes are swapped and mirrored relative to world space, matching
/// `TerrainInfo::GetGrid`.
pub fn terrain_grid_coords(x: f32, y: f32) -> Option<(usize, usize)> {
    let gx = (CENTER_GRID_ID as f32 - y / SIZE_OF_GRIDS) as i32;
    let gy = (CENTER_GRID_ID as f32 - x / SIZE_OF_GRIDS) as i32;

    if gx < 0 || gy < 0 || gx >= MAX_NUMBER_OF_GRIDS as i32 || gy >= MAX_NUMBER_OF_GRIDS as i32 {
        return None;
    }

    Some((gx as usize, gy as usize))
}
