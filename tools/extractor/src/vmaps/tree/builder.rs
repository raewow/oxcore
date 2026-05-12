//! VMap Tree Builder
//!
//! Constructs BIH trees for server-compatible vmap files.
//! Also includes legacy BVH tree construction using SAH (Surface Area Heuristic).

use crate::vmaps::dir_bin::{DirBinEntry, MOD_HAS_BOUND, MOD_M2};
use crate::vmaps::transform::euler_to_matrix;
use crate::vmaps::tree::bih::BIH;
use crate::vmaps::tree::output::{ModelSpawnEntry, VMapTileWriter, VMapTreeWriter};
use crate::vmaps::tree::structures::{BVHNode, BVHTree};
use crate::vmaps::types::BoundingBox;
use crate::vmaps::vmo_converter;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Build map tree from dir_bin entries
///
/// This creates:
/// - One .vmtree file per map (with BIH tree)
/// - One .vmtile file per tile that has spawns
pub fn build_map_tree(
    map_id: u32,
    entries: &[DirBinEntry],
    buildings_dir: &Path,
) -> Result<PathBuf> {
    let vmaps_dir = buildings_dir
        .parent()
        .context("Invalid buildings dir")?;

    info!(
        "Building map tree for map {} with {} entries",
        map_id,
        entries.len()
    );

    if entries.is_empty() {
        // Create minimal tree file even for empty maps
        let output_path = vmaps_dir.join(format!("{:03}.vmtree", map_id));
        let empty_bih = BIH::new();
        VMapTreeWriter::write(&output_path, &empty_bih, true, &[])?;
        info!("Created empty vmtree for map {}", map_id);
        return Ok(output_path);
    }

    // Calculate transformed bounds for M2 models that don't have bounds yet
    // (equivalent to MaNGOS TileAssembler::calculateTransformedBound)
    let mut entries_with_bounds: Vec<DirBinEntry> = entries.to_vec();
    for entry in &mut entries_with_bounds {
        if entry.flags & MOD_M2 != 0 && entry.flags & MOD_HAS_BOUND == 0 {
            let model_path = buildings_dir.join(&entry.name);
            if let Ok(vertices) = vmo_converter::read_raw_model_vertices(&model_path) {
                if !vertices.is_empty() {
                    // Transform vertices by position, rotation, scale
                    let rotation = euler_to_matrix(entry.rotation);
                    let scale = entry.scale;

                    let mut bound_min = glam::Vec3::splat(f32::INFINITY);
                    let mut bound_max = glam::Vec3::splat(f32::NEG_INFINITY);

                    for v in &vertices {
                        let transformed = rotation.mul_vec3(*v * scale);
                        bound_min = bound_min.min(transformed);
                        bound_max = bound_max.max(transformed);
                    }

                    // Add world position offset
                    bound_min += entry.position;
                    bound_max += entry.position;

                    entry.bounds = Some(BoundingBox::new(bound_min, bound_max));
                    entry.flags |= MOD_HAS_BOUND;
                }
            }
        }
    }

    // Group entries by tile (using entries_with_bounds which has computed M2 bounds)
    let mut tiles: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for (i, entry) in entries_with_bounds.iter().enumerate() {
        tiles
            .entry((entry.tile_x, entry.tile_y))
            .or_default()
            .push(i);
    }

    info!("Map {} has {} tiles with spawns", map_id, tiles.len());

    // Determine if this is a tiled map (more than 1 tile or has ADT tiles)
    // Maps with only global WMO are not tiled
    let is_tiled = tiles.len() > 1 || tiles.keys().any(|(x, y)| *x != 65 || *y != 65);

    // Collect all spawn entries with their bounds for the BIH
    let spawn_entries: Vec<ModelSpawnEntry> = entries_with_bounds
        .iter()
        .map(ModelSpawnEntry::from_dir_bin)
        .collect();

    // Get bounds for each spawn entry
    let bounds: Vec<BoundingBox> = entries_with_bounds
        .iter()
        .map(|e| {
            if let Some(ref b) = e.bounds {
                if e.flags & MOD_HAS_BOUND != 0 && b.is_valid() {
                    return *b;
                }
            }
            // Use position as a point bound (small box around position)
            BoundingBox::new(
                e.position - glam::Vec3::splat(0.5),
                e.position + glam::Vec3::splat(0.5),
            )
        })
        .collect();

    // Build BIH tree from bounds
    debug!("Building BIH tree with {} primitives", bounds.len());
    let (bih, stats) = BIH::build(bounds.len(), |i| bounds[i]);
    debug!(
        "BIH built: {} nodes, {} leaves, tree size: {}",
        stats.num_nodes,
        stats.num_leaves,
        bih.tree.len()
    );

    // Create spawn entries with referenced_val (index into BIH objects)
    // The BIH objects array maps original indices, so we need to find where each entry ended up
    let global_spawns: Vec<(ModelSpawnEntry, u32)> = if !is_tiled {
        // For non-tiled maps, all spawns go in the vmtree
        spawn_entries
            .iter()
            .enumerate()
            .map(|(i, spawn)| {
                // Find index in BIH objects array
                let referenced_val = bih
                    .objects
                    .iter()
                    .position(|&obj| obj == i as u32)
                    .unwrap_or(i) as u32;
                (spawn.clone(), referenced_val)
            })
            .collect()
    } else {
        Vec::new()
    };

    // Write .vmtree file
    let output_path = vmaps_dir.join(format!("{:03}.vmtree", map_id));
    VMapTreeWriter::write(&output_path, &bih, is_tiled, &global_spawns)?;
    info!("Created vmtree: {}", output_path.display());

    // Write .vmtile files for tiled maps
    if is_tiled {
        for ((tile_x, tile_y), tile_entry_indices) in &tiles {
            let tile_spawns: Vec<(ModelSpawnEntry, u32)> = tile_entry_indices
                .iter()
                .map(|&idx| {
                    let spawn = ModelSpawnEntry::from_dir_bin(&entries_with_bounds[idx]);

                    // Find index in BIH objects array
                    let referenced_val = bih
                        .objects
                        .iter()
                        .position(|&obj| obj == idx as u32)
                        .unwrap_or(idx) as u32;

                    (spawn, referenced_val)
                })
                .collect();

            if !tile_spawns.is_empty() {
                let tile_path = vmaps_dir.join(format!("{:03}_{:02}_{:02}.vmtile", map_id, tile_x, tile_y));
                VMapTileWriter::write(&tile_path, &tile_spawns)?;
                debug!(
                    "Created vmtile: {} ({} spawns)",
                    tile_path.display(),
                    tile_spawns.len()
                );
            }
        }
        info!(
            "Created {} vmtile files for map {}",
            tiles.len(),
            map_id
        );
    }

    Ok(output_path)
}

/// Maximum triangles per leaf node
const MAX_LEAF_TRIANGLES: usize = 8;

/// Minimum triangles to split (below this, create a leaf)
const MIN_SPLIT_TRIANGLES: usize = 4;

impl BVHTree {
    /// Build BVH tree from triangles using top-down construction
    pub fn build(mut self) -> Self {
        if self.triangles.is_empty() {
            return self;
        }

        // Build triangle indices
        let triangle_indices: Vec<usize> = (0..self.triangles.len()).collect();

        // Build the tree recursively (this adds all nodes)
        self.build_node(&triangle_indices);

        self
    }

    /// Recursively build a BVH node
    /// Returns the index of the created node
    fn build_node(&mut self, triangle_indices: &[usize]) -> usize {
        // Calculate bounding box for these triangles
        let bbox = self.calculate_bounds(triangle_indices);

        // Check if we should create a leaf
        if triangle_indices.len() <= MAX_LEAF_TRIANGLES {
            let node = BVHNode::Leaf {
                bbox,
                triangle_indices: triangle_indices.to_vec(),
            };
            let idx = self.nodes.len();
            self.nodes.push(node);
            return idx;
        }

        // Split triangles along longest axis
        let (left_indices, right_indices) = self.split_triangles(triangle_indices, &bbox);

        // If split didn't help, create leaf
        if left_indices.is_empty() || right_indices.is_empty() {
            let node = BVHNode::Leaf {
                bbox,
                triangle_indices: triangle_indices.to_vec(),
            };
            let idx = self.nodes.len();
            self.nodes.push(node);
            return idx;
        }

        // Build child nodes recursively (these add themselves to self.nodes)
        let left_idx = self.build_node(&left_indices);
        let right_idx = self.build_node(&right_indices);

        // Now create branch node
        let node = BVHNode::Branch {
            bbox,
            left: left_idx,
            right: right_idx,
        };
        let idx = self.nodes.len();
        self.nodes.push(node);
        idx
    }

    /// Calculate bounding box for a set of triangles
    fn calculate_bounds(&self, triangle_indices: &[usize]) -> BoundingBox {
        if triangle_indices.is_empty() {
            return BoundingBox::default();
        }

        let mut bbox = self.triangles[triangle_indices[0]].bounding_box();
        for &idx in &triangle_indices[1..] {
            bbox = bbox.union(&self.triangles[idx].bounding_box());
        }
        bbox
    }

    /// Split triangles along the longest axis
    fn split_triangles(
        &self,
        triangle_indices: &[usize],
        bbox: &BoundingBox,
    ) -> (Vec<usize>, Vec<usize>) {
        // Find longest axis
        let axis = bbox.longest_axis();

        // Calculate split position (median of centroids)
        let mut centroids: Vec<f32> = triangle_indices
            .iter()
            .map(|&idx| self.triangles[idx].centroid()[axis])
            .collect();

        centroids.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let split_pos = if centroids.len() % 2 == 0 {
            (centroids[centroids.len() / 2 - 1] + centroids[centroids.len() / 2]) / 2.0
        } else {
            centroids[centroids.len() / 2]
        };

        // Split triangles based on centroid position
        let mut left = Vec::new();
        let mut right = Vec::new();

        for &idx in triangle_indices {
            let centroid = self.triangles[idx].centroid();
            if centroid[axis] < split_pos {
                left.push(idx);
            } else {
                right.push(idx);
            }
        }

        // Handle edge case where all triangles went to one side
        if left.is_empty() && !right.is_empty() {
            let mid = right.len() / 2;
            left = right.split_off(mid);
        } else if right.is_empty() && !left.is_empty() {
            let mid = left.len() / 2;
            right = left.split_off(mid);
        }

        (left, right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vmaps::tree::structures::TriangleData;
    use glam::Vec3;

    fn create_test_triangles() -> Vec<TriangleData> {
        vec![
            // Triangle 1: near origin
            TriangleData::new(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                0,
            ),
            // Triangle 2: offset in X
            TriangleData::new(
                Vec3::new(10.0, 0.0, 0.0),
                Vec3::new(11.0, 0.0, 0.0),
                Vec3::new(10.0, 1.0, 0.0),
                0,
            ),
            // Triangle 3: offset in Y
            TriangleData::new(
                Vec3::new(0.0, 10.0, 0.0),
                Vec3::new(1.0, 10.0, 0.0),
                Vec3::new(0.0, 11.0, 0.0),
                0,
            ),
            // Triangle 4: offset in both
            TriangleData::new(
                Vec3::new(10.0, 10.0, 0.0),
                Vec3::new(11.0, 10.0, 0.0),
                Vec3::new(10.0, 11.0, 0.0),
                0,
            ),
        ]
    }

    #[test]
    fn test_build_empty_tree() {
        let tree = BVHTree::new().build();
        assert!(tree.is_empty());
        assert_eq!(tree.node_count(), 0);
    }

    #[test]
    fn test_build_single_triangle() {
        let triangles = vec![TriangleData::new(
            Vec3::ZERO,
            Vec3::X,
            Vec3::Y,
            0,
        )];

        let tree = BVHTree::from_triangles(triangles).build();

        assert_eq!(tree.triangle_count(), 1);
        assert_eq!(tree.node_count(), 1);

        // Should be a leaf
        assert!(tree.root().unwrap().is_leaf());
        assert_eq!(tree.root().unwrap().triangle_count(), 1);
    }

    #[test]
    fn test_build_multiple_triangles() {
        let triangles = create_test_triangles();
        let tree = BVHTree::from_triangles(triangles).build();

        assert_eq!(tree.triangle_count(), 4);
        assert!(tree.node_count() > 0);

        // Root should exist
        assert!(tree.root().is_some());

        // All triangles should be reachable
        let root_idx = tree.nodes.len() - 1;
        let leaf_count = count_leaf_triangles(&tree, root_idx);
        assert_eq!(leaf_count, 4);
    }

    #[test]
    fn test_build_many_triangles() {
        // Create many triangles to force deeper tree
        let mut triangles = Vec::new();
        for i in 0..50 {
            let x = (i % 10) as f32;
            let y = (i / 10) as f32;
            triangles.push(TriangleData::new(
                Vec3::new(x, y, 0.0),
                Vec3::new(x + 1.0, y, 0.0),
                Vec3::new(x, y + 1.0, 0.0),
                0,
            ));
        }

        let tree = BVHTree::from_triangles(triangles).build();

        assert_eq!(tree.triangle_count(), 50);

        // Should have multiple levels
        assert!(tree.node_count() > 1);

        // Verify all triangles are reachable
        // Root is the last node added
        let root_idx = tree.nodes.len() - 1;
        let leaf_count = count_leaf_triangles(&tree, root_idx);
        eprintln!("Total triangles: {}, Leaf count: {}, Nodes: {}, Root idx: {}", tree.triangle_count(), leaf_count, tree.node_count(), root_idx);
        assert_eq!(leaf_count, 50);
    }

    #[test]
    fn test_calculate_bounds() {
        let triangles = create_test_triangles();
        let tree = BVHTree::from_triangles(triangles);

        let indices = vec![0, 1];
        let bbox = tree.calculate_bounds(&indices);

        // Should contain both triangles
        assert!(bbox.contains_point(Vec3::ZERO));
        assert!(bbox.contains_point(Vec3::new(11.0, 1.0, 0.0)));
    }

    #[test]
    fn test_split_triangles() {
        let triangles = create_test_triangles();
        let tree = BVHTree::from_triangles(triangles);

        let indices: Vec<usize> = (0..4).collect();
        let bbox = tree.calculate_bounds(&indices);

        let (left, right) = tree.split_triangles(&indices, &bbox);

        // Should split into non-empty groups
        assert!(!left.is_empty());
        assert!(!right.is_empty());

        // Should contain all original indices
        assert_eq!(left.len() + right.len(), 4);
    }

    /// Helper: Count total triangles in all leaf nodes
    fn count_leaf_triangles(tree: &BVHTree, node_idx: usize) -> usize {
        if node_idx >= tree.nodes.len() {
            return 0;
        }

        match &tree.nodes[node_idx] {
            BVHNode::Leaf { triangle_indices, .. } => triangle_indices.len(),
            BVHNode::Branch { left, right, .. } => {
                count_leaf_triangles(tree, *left) + count_leaf_triangles(tree, *right)
            }
        }
    }
}
