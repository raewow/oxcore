//! VMap Shared Types and Structures
//!
//! Common data structures used across VMap extraction and assembly.

use glam::{Quat, Vec3};
use std::collections::HashSet;

/// VMAP binary format magic number (8 bytes including null terminator, matching MaNGOS)
pub const VMAP_MAGIC: &[u8; 8] = b"VMAPs05\0";

/// Maximum triangles per BVH leaf node
pub const MAX_TRIANGLES_PER_LEAF: usize = 32;

/// 3D Axis-Aligned Bounding Box
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

impl BoundingBox {
    /// Create a new bounding box
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Create an empty bounding box (inverted)
    pub fn empty() -> Self {
        Self {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
        }
    }

    /// Check if bounding box is valid (min <= max)
    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x
            && self.min.y <= self.max.y
            && self.min.z <= self.max.z
    }

    /// Expand this bounding box to include another
    pub fn expand(&mut self, other: &BoundingBox) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }

    /// Expand this bounding box to include a point
    pub fn expand_point(&mut self, point: Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }

    /// Check if point is contained within bounding box
    pub fn contains_point(&self, point: Vec3) -> bool {
        point.cmpge(self.min).all() && point.cmple(self.max).all()
    }

    /// Get center of bounding box
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Get extents (half-size) of bounding box
    pub fn extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    /// Get size of bounding box
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    /// Get longest axis (0=X, 1=Y, 2=Z)
    pub fn longest_axis(&self) -> usize {
        let size = self.size();
        if size.x >= size.y && size.x >= size.z {
            0
        } else if size.y >= size.z {
            1
        } else {
            2
        }
    }

    /// Create union of two bounding boxes
    pub fn union(&self, other: &BoundingBox) -> BoundingBox {
        BoundingBox {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Check if bounding box is zero (min and max are both zero)
    pub fn is_zero(&self) -> bool {
        self.min == Vec3::ZERO && self.max == Vec3::ZERO
    }
}

impl Default for BoundingBox {
    fn default() -> Self {
        Self::empty()
    }
}

/// WMO Doodad Spawn Information
#[derive(Debug, Clone)]
pub struct WMODoodadSpawn {
    pub name_index: u32,
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: f32,
    pub flags: u32,
}

/// WMO Doodad Data Collection
#[derive(Debug, Clone, Default)]
pub struct WMODoodadData {
    pub spawns: Vec<WMODoodadSpawn>,
    pub references: HashSet<u16>,
}

/// WMO Placement in World
#[derive(Debug, Clone)]
pub struct WMOPlacement {
    pub unique_id: u32,
    pub position: Vec3,
    pub rotation: Vec3, // Euler angles (radians)
    pub bounding_box: BoundingBox,
    pub flags: u16,
    pub doodad_set: u16,
    pub name_set: u16,
    pub scale: u16,
}

/// M2 Doodad Placement in World
#[derive(Debug, Clone)]
pub struct DoodadPlacement {
    pub name_index: u32,
    pub unique_id: u32,
    pub position: Vec3,
    pub rotation: Vec3, // Euler angles (radians)
    pub scale: u16,
    pub flags: u16,
}

/// WMO Material Information
#[derive(Debug, Clone)]
pub struct WMOMaterial {
    pub flags: u32,
    pub shader: u32,
    pub blend_mode: u32,
    pub texture1: u32,
    pub color1: u32,
    pub texture2: u32,
    pub color2: u32,
    pub ground_type: u32,
}

/// WMO Group Information (from MOGI chunk)
#[derive(Debug, Clone)]
pub struct WMOGroupInfo {
    pub flags: u32,
    pub bounding_box: BoundingBox,
    pub name_offset: u32,
}

/// WMO Doodad Definition (from MODD chunk)
#[derive(Debug, Clone)]
pub struct WMODoodadDef {
    pub name_index: u32,
    pub flags: u32,
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: f32,
    pub color: u32,
}

/// WMO Doodad Set (from MODS chunk)
#[derive(Debug, Clone)]
pub struct WMODoodadSet {
    pub name: [u8; 20],
    pub start_index: u32,
    pub count: u32,
    pub unused: u32,
}

/// WMO Render Batch (from MOBA chunk)
#[derive(Debug, Clone)]
pub struct WMOBatch {
    pub start_index: u32,
    pub count: u16,
    pub min_index: u16,
    pub max_index: u16,
    pub material_id: u8,
}

/// Triangle with vertices and material
#[derive(Debug, Clone)]
pub struct Triangle {
    pub vertices: [Vec3; 3],
    pub normal: Vec3,
    pub material: u16,
}

impl Triangle {
    /// Create a new triangle
    pub fn new(v0: Vec3, v1: Vec3, v2: Vec3, material: u16) -> Self {
        // Calculate normal using cross product
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let normal = edge1.cross(edge2).normalize();

        Self {
            vertices: [v0, v1, v2],
            normal,
            material,
        }
    }

    /// Get centroid of triangle
    pub fn centroid(&self) -> Vec3 {
        (self.vertices[0] + self.vertices[1] + self.vertices[2]) / 3.0
    }

    /// Get bounding box of triangle
    pub fn bounding_box(&self) -> BoundingBox {
        let mut bbox = BoundingBox::empty();
        for vertex in &self.vertices {
            bbox.expand_point(*vertex);
        }
        bbox
    }
}

/// VMAP Root Header (binary format)
/// MaNGOS format: magic(8) + nVectors(u32) + nGroups(u32) + RootWMOID(u32)
#[derive(Debug, Clone)]
pub struct VMapRootHeader {
    pub magic: [u8; 8], // "VMAPs05\0"
    pub n_vectors: u32,
    pub n_groups: u32,
    pub root_wmo_id: u32,
}

/// VMAP Group Header (binary format)
#[derive(Debug, Clone)]
pub struct VMapGroupHeader {
    pub flags: u32,
    pub bounding_box: BoundingBox,
    pub liquid_flags: u32,
    pub n_vertices: u32,
    pub n_triangles: u32,
    pub n_batches: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vmap_magic_is_8_bytes() {
        assert_eq!(VMAP_MAGIC.len(), 8);
        assert_eq!(&VMAP_MAGIC[..7], b"VMAPs05");
        assert_eq!(VMAP_MAGIC[7], 0);
    }

    #[test]
    fn test_bounding_box_empty() {
        let bbox = BoundingBox::empty();
        assert!(!bbox.is_valid());
    }

    #[test]
    fn test_bounding_box_new() {
        let bbox = BoundingBox::new(
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0),
        );
        assert!(bbox.is_valid());
        assert_eq!(bbox.center(), Vec3::ZERO);
        assert_eq!(bbox.size(), Vec3::splat(2.0));
    }

    #[test]
    fn test_bounding_box_expand() {
        let mut bbox1 = BoundingBox::new(Vec3::ZERO, Vec3::ONE);
        let bbox2 = BoundingBox::new(Vec3::splat(-1.0), Vec3::ZERO);

        bbox1.expand(&bbox2);

        assert_eq!(bbox1.min, Vec3::splat(-1.0));
        assert_eq!(bbox1.max, Vec3::ONE);
    }

    #[test]
    fn test_bounding_box_expand_point() {
        let mut bbox = BoundingBox::empty();
        bbox.expand_point(Vec3::ZERO);
        bbox.expand_point(Vec3::ONE);
        bbox.expand_point(Vec3::new(-1.0, 0.5, 0.5));

        assert_eq!(bbox.min, Vec3::new(-1.0, 0.0, 0.0));
        assert_eq!(bbox.max, Vec3::ONE);
    }

    #[test]
    fn test_bounding_box_contains_point() {
        let bbox = BoundingBox::new(Vec3::ZERO, Vec3::ONE);

        assert!(bbox.contains_point(Vec3::splat(0.5)));
        assert!(bbox.contains_point(Vec3::ZERO));
        assert!(bbox.contains_point(Vec3::ONE));
        assert!(!bbox.contains_point(Vec3::splat(-0.1)));
        assert!(!bbox.contains_point(Vec3::splat(1.1)));
    }

    #[test]
    fn test_bounding_box_longest_axis() {
        let bbox_x = BoundingBox::new(Vec3::ZERO, Vec3::new(10.0, 1.0, 1.0));
        assert_eq!(bbox_x.longest_axis(), 0); // X

        let bbox_y = BoundingBox::new(Vec3::ZERO, Vec3::new(1.0, 10.0, 1.0));
        assert_eq!(bbox_y.longest_axis(), 1); // Y

        let bbox_z = BoundingBox::new(Vec3::ZERO, Vec3::new(1.0, 1.0, 10.0));
        assert_eq!(bbox_z.longest_axis(), 2); // Z
    }

    #[test]
    fn test_triangle_new() {
        let v0 = Vec3::ZERO;
        let v1 = Vec3::new(1.0, 0.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);

        let tri = Triangle::new(v0, v1, v2, 0);

        // Normal should point in +Z direction
        assert!((tri.normal.z - 1.0).abs() < 0.001);
        assert!(tri.normal.x.abs() < 0.001);
        assert!(tri.normal.y.abs() < 0.001);
    }

    #[test]
    fn test_triangle_centroid() {
        let tri = Triangle::new(
            Vec3::ZERO,
            Vec3::new(3.0, 0.0, 0.0),
            Vec3::new(0.0, 3.0, 0.0),
            0,
        );

        let centroid = tri.centroid();
        assert_eq!(centroid, Vec3::new(1.0, 1.0, 0.0));
    }

    #[test]
    fn test_triangle_bounding_box() {
        let tri = Triangle::new(
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.0, 3.0, 1.0),
            0,
        );

        let bbox = tri.bounding_box();
        assert_eq!(bbox.min, Vec3::new(-1.0, -1.0, -1.0));
        assert_eq!(bbox.max, Vec3::new(2.0, 3.0, 1.0));
    }
}
