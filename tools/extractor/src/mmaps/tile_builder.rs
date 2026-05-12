//! Tile Builder for MMap Generation
//!
//! Uses Recast/Detour to build navigation mesh tiles from terrain
//! and collision geometry.

use anyhow::Result;

use super::terrain_builder::{MeshData, GRID_SIZE};

// Recast/Detour FFI bindings
#[allow(non_camel_case_types)]
mod ffi {
    use std::ffi::c_void;

    pub type rcContext = c_void;
    pub type rcHeightfield = c_void;
    pub type rcCompactHeightfield = c_void;
    pub type rcContourSet = c_void;
    pub type rcPolyMesh = c_void;
    pub type rcPolyMeshDetail = c_void;
    pub type dtNavMesh = c_void;
    pub type dtNavMeshCreateParams = c_void;

    #[repr(C)]
    #[derive(Debug, Clone, Default)]
    pub struct rcConfig {
        pub width: i32,
        pub height: i32,
        pub tile_size: i32,
        pub border_size: i32,
        pub cs: f32,
        pub ch: f32,
        pub bmin: [f32; 3],
        pub bmax: [f32; 3],
        pub walkable_slope_angle: f32,
        pub walkable_height: i32,
        pub walkable_climb: i32,
        pub walkable_radius: i32,
        pub max_edge_len: i32,
        pub max_simplification_error: f32,
        pub min_region_area: i32,
        pub merge_region_area: i32,
        pub max_verts_per_poly: i32,
        pub detail_sample_dist: f32,
        pub detail_sample_max_error: f32,
    }

    // Note: In a full implementation, we would link to recastnavigation-sys
    // For now, we define the interface but the actual implementation
    // would require the FFI bindings to be properly linked
}

/// MMap generation parameters (matching MaNGOS)
pub struct MMapConfig {
    // Grid constants
    pub base_unit_dim: f32,
    pub vertex_per_tile: i32,
    pub tiles_per_map: i32,

    // Agent parameters
    pub agent_height: f32,
    pub agent_radius: f32,
    pub agent_max_climb: f32,
    pub agent_max_climb_terrain: f32,

    // Cell sizes
    pub cs: f32, // XZ cell size
    pub ch: f32, // Y cell size

    // Recast config
    pub walkable_slope_angle: f32,
    pub walkable_slope_angle_vmaps: f32,
    pub detail_sample_dist: f32,
    pub detail_sample_max_error: f32,
    pub max_simplification_error: f32,
    pub min_region_area: i32,
    pub merge_region_area: i32,
}

impl Default for MMapConfig {
    fn default() -> Self {
        Self {
            // Grid constants from MaNGOS MapBuilder.h
            base_unit_dim: 0.2666666,
            vertex_per_tile: 80,
            tiles_per_map: 25, // VERTEX_PER_MAP / VERTEX_PER_TILE

            // Agent parameters from TileWorker.cpp
            agent_height: 1.5,
            agent_radius: 0.2, // 0.3 for non-continents
            agent_max_climb: 1.2,
            agent_max_climb_terrain: 1.8,

            // Cell sizes
            cs: 0.2666666, // BASE_UNIT_DIM
            ch: 0.25,      // For continents (0.1 for other maps)

            // Recast config defaults
            walkable_slope_angle: 75.0,
            walkable_slope_angle_vmaps: 61.0,
            detail_sample_dist: 2.0,
            detail_sample_max_error: 0.5,
            max_simplification_error: 1.8,
            min_region_area: 30,
            merge_region_area: 10,
        }
    }
}

impl MMapConfig {
    /// Create config for continent maps (Azeroth, Kalimdor, etc)
    pub fn for_continent() -> Self {
        let mut config = Self::default();
        config.ch = 0.25;
        config.agent_radius = 0.2;
        config
    }

    /// Create config for instance/dungeon maps
    pub fn for_instance() -> Self {
        let mut config = Self::default();
        config.ch = 0.1;
        config.agent_radius = 0.3;
        config
    }

    /// Check if map is a continent
    pub fn is_continent(map_id: u32) -> bool {
        // Maps 0 (Azeroth), 1 (Kalimdor), 530 (Outland), 571 (Northrend)
        matches!(map_id, 0 | 1 | 530 | 571)
    }

    /// Get config for specific map
    pub fn for_map(map_id: u32) -> Self {
        if Self::is_continent(map_id) {
            Self::for_continent()
        } else {
            Self::for_instance()
        }
    }

    /// Calculate derived parameters
    pub fn calc_walkable_height(&self) -> i32 {
        (self.agent_height / self.ch).ceil() as i32
    }

    pub fn calc_walkable_climb(&self) -> i32 {
        (self.agent_max_climb / self.ch).floor() as i32
    }

    pub fn calc_walkable_climb_terrain(&self) -> i32 {
        (self.agent_max_climb_terrain / self.ch).floor() as i32
    }

    pub fn calc_walkable_radius(&self) -> i32 {
        (self.agent_radius / self.cs).ceil() as i32
    }

    pub fn calc_max_edge_len(&self) -> i32 {
        (12.0 / self.cs).floor() as i32
    }

    pub fn calc_border_size(&self) -> i32 {
        self.calc_walkable_radius() + 3
    }

    /// Build rcConfig for Recast
    pub fn build_rc_config(&self, bmin: [f32; 3], bmax: [f32; 3]) -> ffi::rcConfig {
        let border_size = self.calc_border_size();

        ffi::rcConfig {
            width: self.vertex_per_tile + border_size * 2,
            height: self.vertex_per_tile + border_size * 2,
            tile_size: self.vertex_per_tile,
            border_size,
            cs: self.cs,
            ch: self.ch,
            bmin,
            bmax,
            walkable_slope_angle: self.walkable_slope_angle,
            walkable_height: self.calc_walkable_height(),
            walkable_climb: self.calc_walkable_climb(),
            walkable_radius: self.calc_walkable_radius(),
            max_edge_len: self.calc_max_edge_len(),
            max_simplification_error: self.max_simplification_error,
            min_region_area: self.min_region_area * self.min_region_area, // rcSqr
            merge_region_area: self.merge_region_area * self.merge_region_area,
            max_verts_per_poly: 6, // DT_VERTS_PER_POLYGON
            detail_sample_dist: self.detail_sample_dist,
            detail_sample_max_error: self.detail_sample_max_error,
        }
    }
}

/// Built navigation mesh tile data
pub struct NavMeshTileData {
    pub data: Vec<u8>,
    pub uses_liquids: bool,
}

/// Tile builder for generating navmesh tiles
pub struct TileBuilder {
    config: MMapConfig,
}

impl TileBuilder {
    pub fn new(map_id: u32) -> Self {
        Self {
            config: MMapConfig::for_map(map_id),
        }
    }

    /// Build a navigation mesh tile from mesh data
    pub fn build_tile(
        &self,
        map_id: u32,
        tile_x: u32,
        tile_y: u32,
        mesh_data: &MeshData,
    ) -> Result<Option<NavMeshTileData>> {
        if mesh_data.solid_verts.is_empty() && mesh_data.liquid_verts.is_empty() {
            return Ok(None);
        }

        // Calculate tile bounds
        let (bmin, bmax) = self.get_tile_bounds(tile_x, tile_y, mesh_data);

        // Build Recast config
        let _rc_config = self.config.build_rc_config(bmin, bmax);

        // NOTE: Full implementation would call Recast/Detour functions here:
        // 1. rcCreateHeightfield
        // 2. rcRasterizeTriangles (for solid geometry)
        // 3. rcRasterizeTriangles (for liquid geometry)
        // 4. rcFilterLowHangingWalkableObstacles
        // 5. rcFilterLedgeSpans
        // 6. rcFilterWalkableLowHeightSpans
        // 7. rcBuildCompactHeightfield
        // 8. rcErodeWalkableArea
        // 9. rcBuildDistanceField
        // 10. rcBuildRegions
        // 11. rcBuildContours
        // 12. rcBuildPolyMesh
        // 13. rcBuildPolyMeshDetail
        // 14. dtCreateNavMeshData

        // For now, return a placeholder indicating this would need FFI implementation
        tracing::warn!(
            "NavMesh tile generation requires Recast FFI - tile [{},{}] skipped",
            tile_x,
            tile_y
        );

        // Return empty tile data as placeholder
        Ok(Some(NavMeshTileData {
            data: Vec::new(),
            uses_liquids: !mesh_data.liquid_verts.is_empty(),
        }))
    }

    /// Calculate tile bounds from mesh data
    fn get_tile_bounds(
        &self,
        tile_x: u32,
        tile_y: u32,
        mesh_data: &MeshData,
    ) -> ([f32; 3], [f32; 3]) {
        let mut bmin = [f32::MAX; 3];
        let mut bmax = [f32::MIN; 3];

        // Include solid vertices
        for i in (0..mesh_data.solid_verts.len()).step_by(3) {
            let x = mesh_data.solid_verts[i];
            let y = mesh_data.solid_verts[i + 1];
            let z = mesh_data.solid_verts[i + 2];

            bmin[0] = bmin[0].min(x);
            bmin[1] = bmin[1].min(y);
            bmin[2] = bmin[2].min(z);
            bmax[0] = bmax[0].max(x);
            bmax[1] = bmax[1].max(y);
            bmax[2] = bmax[2].max(z);
        }

        // Include liquid vertices
        for i in (0..mesh_data.liquid_verts.len()).step_by(3) {
            let x = mesh_data.liquid_verts[i];
            let y = mesh_data.liquid_verts[i + 1];
            let z = mesh_data.liquid_verts[i + 2];

            bmin[0] = bmin[0].min(x);
            bmin[1] = bmin[1].min(y);
            bmin[2] = bmin[2].min(z);
            bmax[0] = bmax[0].max(x);
            bmax[1] = bmax[1].max(y);
            bmax[2] = bmax[2].max(z);
        }

        // If no vertices, use tile-based defaults
        if bmin[0] == f32::MAX {
            let x_offset = (32.0 - tile_x as f32) * GRID_SIZE;
            let y_offset = (32.0 - tile_y as f32) * GRID_SIZE;

            bmin = [y_offset - GRID_SIZE, -500.0, x_offset - GRID_SIZE];
            bmax = [y_offset, 500.0, x_offset];
        }

        (bmin, bmax)
    }
}

/// Intermediate values for debugging (matches MaNGOS IntermediateValues)
#[derive(Default)]
pub struct IntermediateValues {
    pub heightfield_time: f64,
    pub compact_time: f64,
    pub contour_time: f64,
    pub polymesh_time: f64,
    pub detail_time: f64,
}
