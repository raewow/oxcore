//! WMO Group File Structures
//!
//! Group files contain the actual 3D geometry for sections of a building.
//! Each WMO can have multiple groups (e.g., different rooms or sections).

use glam::{Vec2, Vec3};
use crate::vmaps::types::{BoundingBox, WMOBatch};
use crate::vmaps::wmo::root::WMORoot;

/// WMO Group File
///
/// Contains geometry for one section of a WMO
#[derive(Debug, Clone)]
pub struct WMOGroup {
    /// Group flags (from MOGP header)
    pub mogp_flags: u32,
    /// Group WMO ID (from MOGP header, MaNGOS: groupWMOID)
    pub group_wmo_id: u32,
    /// Bounding box
    pub bounding_box: BoundingBox,
    /// Name offset in MOGN chunk
    pub name_offset: u32,
    /// Descriptive group name offset
    pub desc_group_name: u32,

    // Geometry data
    /// Vertex positions (from MOVT chunk)
    pub vertices: Vec<Vec3>,
    /// Vertex normals (from MONR chunk)
    pub normals: Vec<Vec3>,
    /// Texture coordinates (from MOTV chunk)
    pub tex_coords: Vec<Vec2>,
    /// Vertex indices (from MOVI chunk)
    pub indices: Vec<u16>,
    /// Material IDs per triangle (from MOPY chunk)
    pub materials: Vec<u16>,

    // Additional data
    /// Render batches (from MOBA chunk)
    pub batch_info: Vec<WMOBatch>,
    /// Doodad references (from MODR chunk)
    pub doodad_references: Vec<u16>,
    /// Portal references
    pub portal_refs: Vec<u16>,

    // Liquid data (from MLIQ chunk)
    /// Liquid type
    pub liquid_type: u32,
    /// Liquid vertices
    pub liquid_vertices: Vec<Vec3>,
    /// Liquid indices
    pub liquid_indices: Vec<u16>,
}

impl WMOGroup {
    /// Create a new empty WMO group
    pub fn new() -> Self {
        Self {
            mogp_flags: 0,
            group_wmo_id: 0,
            bounding_box: BoundingBox::default(),
            name_offset: 0,
            desc_group_name: 0,
            vertices: Vec::new(),
            normals: Vec::new(),
            tex_coords: Vec::new(),
            indices: Vec::new(),
            materials: Vec::new(),
            batch_info: Vec::new(),
            doodad_references: Vec::new(),
            portal_refs: Vec::new(),
            liquid_type: 0,
            liquid_vertices: Vec::new(),
            liquid_indices: Vec::new(),
        }
    }

    /// Check if this group should be skipped during extraction
    /// Matches MaNGOS WMOGroup::ShouldSkip()
    pub fn should_skip(&self, _root: &WMORoot) -> bool {
        // Skip groups without geometry
        if self.vertices.is_empty() || self.indices.is_empty() {
            return true;
        }

        // Skip unreachable groups (MaNGOS: mogpFlags & 0x80)
        if self.mogp_flags & 0x80 != 0 {
            return true;
        }

        // Skip antiportal groups (MaNGOS: mogpFlags & 0x4000000)
        if self.mogp_flags & 0x4000000 != 0 {
            return true;
        }

        false
    }

    /// Check if group has collision data
    pub fn has_collision(&self) -> bool {
        !self.vertices.is_empty() && !self.indices.is_empty()
    }

    /// Check if group has liquid data
    pub fn has_liquid(&self) -> bool {
        !self.liquid_vertices.is_empty() && !self.liquid_indices.is_empty()
    }

    /// Get number of vertices
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Get number of triangles
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Get triangle by index (returns indices into vertex array)
    pub fn triangle_indices(&self, triangle_index: usize) -> Option<(u16, u16, u16)> {
        let base = triangle_index * 3;
        if base + 2 < self.indices.len() {
            Some((
                self.indices[base],
                self.indices[base + 1],
                self.indices[base + 2],
            ))
        } else {
            None
        }
    }
}

impl Default for WMOGroup {
    fn default() -> Self {
        Self::new()
    }
}

/// MOGP Header (Group Header)
#[derive(Debug, Clone, Copy)]
pub struct MOGPHeader {
    pub group_name_offset: u32,
    pub desc_group_name: u32,
    pub flags: u32,
    pub bounding_box: BoundingBox,
    pub portal_start: u16,
    pub portal_count: u16,
    pub trans_batch_count: u16,
    pub int_batch_count: u16,
    pub ext_batch_count: u16,
    pub padding: u16,
    pub fogs: [u8; 4],
    pub liquid_type: u32,
    pub group_id: u32,
    pub unknown1: u32,
    pub unknown2: u32,
}

/// MOGP Flags
pub mod flags {
    /// Group has BSP tree
    pub const HAS_BSP: u32 = 0x01;
    /// Unknown flag
    pub const HAS_LIGHT_MAP: u32 = 0x02;
    /// Group has vertex colors
    pub const HAS_VERTEX_COLORS: u32 = 0x04;
    /// Group is outside
    pub const OUTDOOR: u32 = 0x08;
    /// Unknown flag
    pub const FLAG_10: u32 = 0x10;
    /// Unknown flag
    pub const FLAG_20: u32 = 0x20;
    /// Do not use local lighting
    pub const DO_NOT_USE_LIGHTING: u32 = 0x40;
    /// Unknown flag
    pub const FLAG_80: u32 = 0x80;
    /// Has lights
    pub const HAS_LIGHTS: u32 = 0x200;
    /// Has doodads
    pub const HAS_DOODADS: u32 = 0x800;
    /// Has water (MLIQ chunk)
    pub const HAS_WATER: u32 = 0x1000;
    /// Indoor
    pub const INDOOR: u32 = 0x2000;
    /// Unknown flag
    pub const FLAG_4000: u32 = 0x4000;
    /// Unknown flag
    pub const FLAG_8000: u32 = 0x8000;
    /// Show skybox
    pub const SHOW_SKYBOX: u32 = 0x40000;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wmo_group_new() {
        let group = WMOGroup::new();
        assert_eq!(group.vertex_count(), 0);
        assert_eq!(group.triangle_count(), 0);
        assert!(!group.has_collision());
        assert!(!group.has_liquid());
    }

    #[test]
    fn test_wmo_group_has_collision() {
        let mut group = WMOGroup::new();
        assert!(!group.has_collision());

        group.vertices.push(Vec3::ZERO);
        assert!(!group.has_collision()); // Need both vertices and indices

        group.indices.push(0);
        assert!(group.has_collision());
    }

    #[test]
    fn test_wmo_group_triangle_count() {
        let mut group = WMOGroup::new();
        group.indices = vec![0, 1, 2, 3, 4, 5];
        assert_eq!(group.triangle_count(), 2);
    }

    #[test]
    fn test_wmo_group_triangle_indices() {
        let mut group = WMOGroup::new();
        group.indices = vec![10, 20, 30, 40, 50, 60];

        assert_eq!(group.triangle_indices(0), Some((10, 20, 30)));
        assert_eq!(group.triangle_indices(1), Some((40, 50, 60)));
        assert_eq!(group.triangle_indices(2), None);
    }

    #[test]
    fn test_wmo_group_should_skip() {
        let root = WMORoot::new();
        let mut group = WMOGroup::new();

        // Should skip empty group
        assert!(group.should_skip(&root));

        // Should not skip group with geometry
        group.vertices.push(Vec3::ZERO);
        group.indices.push(0);
        assert!(!group.should_skip(&root));
    }
}
