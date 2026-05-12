//! VMap Assembly Pipeline
//!
//! Combines extracted geometry with placement data to build final collision trees.

use anyhow::Result;
use glam::{Mat3, Vec3};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::vmaps::tree::{BVHTree, TriangleData};
use crate::vmaps::transform::{decode_scale, euler_to_matrix, transform_vertices};
use crate::vmaps::types::{DoodadPlacement, WMOPlacement};

/// Model instance with transformation
#[derive(Debug, Clone)]
pub struct ModelInstance {
    /// Model file path
    pub model_path: PathBuf,
    /// World position
    pub position: Vec3,
    /// Rotation matrix
    pub rotation: Mat3,
    /// Scale factor
    pub scale: f32,
    /// Unique instance ID
    pub instance_id: u32,
}

impl ModelInstance {
    /// Create from WMO placement
    pub fn from_wmo_placement(placement: &WMOPlacement, model_path: PathBuf) -> Self {
        let rotation = euler_to_matrix(placement.rotation);
        Self {
            model_path,
            position: placement.position,
            rotation,
            scale: decode_scale(placement.scale),
            instance_id: placement.unique_id,
        }
    }

    /// Create from M2 doodad placement
    pub fn from_doodad_placement(placement: &DoodadPlacement, model_path: PathBuf) -> Self {
        let rotation = euler_to_matrix(placement.rotation);
        Self {
            model_path,
            position: placement.position,
            rotation,
            scale: decode_scale(placement.scale),
            instance_id: placement.unique_id,
        }
    }

    /// Transform a set of vertices with this instance's transformation
    pub fn transform_vertices(&self, vertices: &[Vec3]) -> Vec<Vec3> {
        transform_vertices(vertices, self.scale, &self.rotation, &self.position)
    }

    /// Transform a single vertex
    pub fn transform_vertex(&self, vertex: Vec3) -> Vec3 {
        self.rotation.mul_vec3(vertex * self.scale) + self.position
    }
}

/// Tile coordinate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileCoord {
    pub x: u32,
    pub y: u32,
}

impl TileCoord {
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

/// Map assembly - organizes geometry by tiles
#[derive(Debug)]
pub struct MapAssembly {
    /// Map ID
    pub map_id: u32,
    /// Tile trees (tile coord -> BVH tree)
    pub tiles: HashMap<TileCoord, BVHTree>,
    /// Model instances per tile
    pub instances: HashMap<TileCoord, Vec<ModelInstance>>,
}

impl MapAssembly {
    /// Create new map assembly
    pub fn new(map_id: u32) -> Self {
        Self {
            map_id,
            tiles: HashMap::new(),
            instances: HashMap::new(),
        }
    }

    /// Add a model instance to the appropriate tile
    pub fn add_instance(&mut self, tile: TileCoord, instance: ModelInstance) {
        self.instances.entry(tile).or_insert_with(Vec::new).push(instance);
    }

    /// Build BVH tree for a specific tile
    pub fn build_tile_tree(&mut self, tile: TileCoord, triangles: Vec<TriangleData>) -> Result<()> {
        if triangles.is_empty() {
            return Ok(());
        }

        let tree = BVHTree::from_triangles(triangles).build();
        self.tiles.insert(tile, tree);

        Ok(())
    }

    /// Get tile tree
    pub fn get_tile_tree(&self, tile: TileCoord) -> Option<&BVHTree> {
        self.tiles.get(&tile)
    }

    /// Get number of tiles with geometry
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// Get total triangle count across all tiles
    pub fn total_triangle_count(&self) -> usize {
        self.tiles.values().map(|tree| tree.triangle_count()).sum()
    }
}

/// Assemble geometry for a single tile
pub fn assemble_tile_geometry(
    instances: &[ModelInstance],
    geometry_loader: &dyn Fn(&Path) -> Result<(Vec<Vec3>, Vec<u16>, Vec<u16>)>,
) -> Result<Vec<TriangleData>> {
    let mut all_triangles = Vec::new();

    for instance in instances {
        // Load model geometry (vertices, indices, materials)
        let (vertices, indices, materials) = match geometry_loader(&instance.model_path) {
            Ok(geom) => geom,
            Err(_) => continue, // Skip models that can't be loaded
        };

        // Transform vertices to world space
        let world_vertices = instance.transform_vertices(&vertices);

        // Build triangles
        for i in (0..indices.len()).step_by(3) {
            if i + 2 >= indices.len() {
                break;
            }

            let i0 = indices[i] as usize;
            let i1 = indices[i + 1] as usize;
            let i2 = indices[i + 2] as usize;

            if i0 >= world_vertices.len() || i1 >= world_vertices.len() || i2 >= world_vertices.len() {
                continue;
            }

            let v0 = world_vertices[i0];
            let v1 = world_vertices[i1];
            let v2 = world_vertices[i2];

            // Get material ID (one per triangle)
            let material_id = materials.get(i / 3).copied().unwrap_or(0);

            all_triangles.push(TriangleData::new(v0, v1, v2, material_id));
        }
    }

    Ok(all_triangles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_instance_from_wmo_placement() {
        let placement = WMOPlacement {
            unique_id: 123,
            position: Vec3::new(100.0, 200.0, 300.0),
            rotation: Vec3::new(0.0, 0.0, 0.0),
            bounding_box: Default::default(),
            flags: 0,
            doodad_set: 0,
            name_set: 0,
            scale: 1024, // 1024 = 1.0 scale
        };

        let instance = ModelInstance::from_wmo_placement(&placement, PathBuf::from("test.wmo"));

        assert_eq!(instance.instance_id, 123);
        assert_eq!(instance.position, Vec3::new(100.0, 200.0, 300.0));
        assert_eq!(instance.scale, 1.0);
    }

    #[test]
    fn test_model_instance_from_doodad_placement() {
        let placement = DoodadPlacement {
            name_index: 0,
            unique_id: 456,
            position: Vec3::new(10.0, 20.0, 30.0),
            rotation: Vec3::new(0.0, 0.0, 0.0),
            scale: 2048, // 2048 = 2.0 scale
            flags: 0,
        };

        let instance = ModelInstance::from_doodad_placement(&placement, PathBuf::from("test.m2"));

        assert_eq!(instance.instance_id, 456);
        assert_eq!(instance.position, Vec3::new(10.0, 20.0, 30.0));
        assert_eq!(instance.scale, 2.0);
    }

    #[test]
    fn test_model_instance_transform_vertex() {
        let placement = DoodadPlacement {
            name_index: 0,
            unique_id: 1,
            position: Vec3::new(10.0, 20.0, 30.0),
            rotation: Vec3::ZERO,
            scale: 2048, // 2048 = 2.0 scale
            flags: 0,
        };

        let instance = ModelInstance::from_doodad_placement(&placement, PathBuf::from("test.m2"));

        // Transform a vertex at origin with scale 2.0 and offset
        let transformed = instance.transform_vertex(Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(transformed, Vec3::new(12.0, 20.0, 30.0)); // (1 * 2) + 10 = 12
    }

    #[test]
    fn test_map_assembly_new() {
        let assembly = MapAssembly::new(0);
        assert_eq!(assembly.map_id, 0);
        assert_eq!(assembly.tile_count(), 0);
        assert_eq!(assembly.total_triangle_count(), 0);
    }

    #[test]
    fn test_map_assembly_add_instance() {
        let mut assembly = MapAssembly::new(0);
        let tile = TileCoord::new(32, 32);

        let placement = DoodadPlacement {
            name_index: 0,
            unique_id: 1,
            position: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: 1024, // 1024 = 1.0 scale
            flags: 0,
        };

        let instance = ModelInstance::from_doodad_placement(&placement, PathBuf::from("test.m2"));
        assembly.add_instance(tile, instance);

        assert_eq!(assembly.instances.len(), 1);
        assert_eq!(assembly.instances[&tile].len(), 1);
    }

    #[test]
    fn test_map_assembly_build_tile_tree() {
        let mut assembly = MapAssembly::new(0);
        let tile = TileCoord::new(32, 32);

        let triangles = vec![
            TriangleData::new(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                0,
            ),
        ];

        assembly.build_tile_tree(tile, triangles).unwrap();

        assert_eq!(assembly.tile_count(), 1);
        assert_eq!(assembly.total_triangle_count(), 1);
        assert!(assembly.get_tile_tree(tile).is_some());
    }

    #[test]
    fn test_assemble_tile_geometry() {
        // Mock geometry loader
        let loader = |_path: &Path| -> Result<(Vec<Vec3>, Vec<u16>, Vec<u16>)> {
            Ok((
                vec![
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                ],
                vec![0, 1, 2],
                vec![0],
            ))
        };

        let placement = DoodadPlacement {
            name_index: 0,
            unique_id: 1,
            position: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: 1024, // 1024 = 1.0 scale
            flags: 0,
        };

        let instance = ModelInstance::from_doodad_placement(&placement, PathBuf::from("test.m2"));
        let instances = vec![instance];

        let triangles = assemble_tile_geometry(&instances, &loader).unwrap();

        assert_eq!(triangles.len(), 1);
        assert_eq!(triangles[0].material_id, 0);
    }

    #[test]
    fn test_assemble_tile_geometry_with_transform() {
        // Mock geometry loader
        let loader = |_path: &Path| -> Result<(Vec<Vec3>, Vec<u16>, Vec<u16>)> {
            Ok((
                vec![
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                ],
                vec![0, 1, 2],
                vec![0],
            ))
        };

        let placement = DoodadPlacement {
            name_index: 0,
            unique_id: 1,
            position: Vec3::new(10.0, 20.0, 30.0),
            rotation: Vec3::ZERO,
            scale: 2048, // 2048 = 2.0 scale
            flags: 0,
        };

        let instance = ModelInstance::from_doodad_placement(&placement, PathBuf::from("test.m2"));
        let instances = vec![instance];

        let triangles = assemble_tile_geometry(&instances, &loader).unwrap();

        assert_eq!(triangles.len(), 1);

        // First vertex should be transformed: (0 * 2) + 10 = 10
        assert_eq!(triangles[0].vertices[0], Vec3::new(10.0, 20.0, 30.0));
        // Second vertex: (1 * 2) + 10 = 12
        assert_eq!(triangles[0].vertices[1], Vec3::new(12.0, 20.0, 30.0));
    }
}
