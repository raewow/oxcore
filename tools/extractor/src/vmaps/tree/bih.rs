//! Bounding Interval Hierarchy (BIH) Tree
//!
//! Implementation of the BIH spatial data structure as used by MaNGOS/CMaNGOS.
//! This is the expected format for the server's vmap system.

use crate::vmaps::types::BoundingBox;
use anyhow::Result;
use byteorder::{LittleEndian, WriteBytesExt};
use glam::Vec3;
use std::io::Write;

/// Maximum stack size for tree traversal/building
const MAX_STACK_SIZE: usize = 64;

/// Default maximum primitives per leaf node
const DEFAULT_MAX_PRIMS: usize = 3;

/// Bounding Interval Hierarchy tree
#[derive(Debug, Clone)]
pub struct BIH {
    /// Tree node data (packed uint32 format)
    pub tree: Vec<u32>,
    /// Object indices
    pub objects: Vec<u32>,
    /// Overall bounding box
    pub bounds: BoundingBox,
}

/// Axis-aligned bounding box for internal use
#[derive(Debug, Clone, Copy)]
struct AABound {
    lo: Vec3,
    hi: Vec3,
}

impl AABound {
    fn from_bbox(bbox: &BoundingBox) -> Self {
        Self {
            lo: bbox.min,
            hi: bbox.max,
        }
    }
}

/// Build data used during tree construction
struct BuildData {
    indices: Vec<u32>,
    prim_bound: Vec<BoundingBox>,
    num_prims: usize,
    max_prims: usize,
}

/// Statistics collected during tree building
#[derive(Debug, Default)]
pub struct BuildStats {
    pub num_nodes: u32,
    pub num_leaves: u32,
    pub sum_objects: u32,
    pub min_objects: u32,
    pub max_objects: u32,
    pub sum_depth: u32,
    pub min_depth: u32,
    pub max_depth: u32,
    pub num_bvh2: u32,
    pub num_leaves_n: [u32; 6],
}

impl BuildStats {
    fn new() -> Self {
        Self {
            min_objects: u32::MAX,
            max_objects: 0,
            min_depth: u32::MAX,
            max_depth: 0,
            ..Default::default()
        }
    }

    fn update_inner(&mut self) {
        self.num_nodes += 1;
    }

    fn update_bvh2(&mut self) {
        self.num_bvh2 += 1;
    }

    fn update_leaf(&mut self, depth: u32, n: u32) {
        self.num_leaves += 1;
        self.min_depth = self.min_depth.min(depth);
        self.max_depth = self.max_depth.max(depth);
        self.sum_depth += depth;
        self.min_objects = self.min_objects.min(n);
        self.max_objects = self.max_objects.max(n);
        self.sum_objects += n;
        let nl = (n as usize).min(5);
        self.num_leaves_n[nl] += 1;
    }
}

/// Convert float to raw int bits (for storing in tree)
fn float_to_raw_int_bits(f: f32) -> u32 {
    f.to_bits()
}

impl BIH {
    /// Create a new empty BIH
    pub fn new() -> Self {
        let mut tree = Vec::new();
        // Create space for dummy leaf
        tree.push(3 << 30); // dummy leaf
        tree.push(0);
        tree.push(0);

        Self {
            tree,
            objects: Vec::new(),
            bounds: BoundingBox::default(),
        }
    }

    /// Build BIH from primitives using a bounds function
    ///
    /// The `get_bounds` function takes (primitive_index) and returns the bounding box
    pub fn build<F>(primitives_count: usize, get_bounds: F) -> (Self, BuildStats)
    where
        F: Fn(usize) -> BoundingBox,
    {
        Self::build_with_leaf_size(primitives_count, get_bounds, DEFAULT_MAX_PRIMS)
    }

    /// Build BIH with custom leaf size
    pub fn build_with_leaf_size<F>(
        primitives_count: usize,
        get_bounds: F,
        leaf_size: usize,
    ) -> (Self, BuildStats)
    where
        F: Fn(usize) -> BoundingBox,
    {
        let mut stats = BuildStats::new();

        if primitives_count == 0 {
            return (Self::new(), stats);
        }

        // Initialize build data
        let mut dat = BuildData {
            max_prims: leaf_size,
            num_prims: primitives_count,
            indices: (0..primitives_count as u32).collect(),
            prim_bound: Vec::with_capacity(primitives_count),
        };

        // Calculate bounds for all primitives
        let first_bounds = get_bounds(0);
        let mut bounds = first_bounds;
        dat.prim_bound.push(first_bounds);

        for i in 1..primitives_count {
            let prim_bbox = get_bounds(i);
            bounds = bounds.union(&prim_bbox);
            dat.prim_bound.push(prim_bbox);
        }

        // Build the tree
        let mut temp_tree = Vec::new();
        Self::build_hierarchy(&mut temp_tree, &mut dat, &bounds, &mut stats);

        // Extract object indices
        let objects = dat.indices;

        (
            Self {
                tree: temp_tree,
                objects,
                bounds,
            },
            stats,
        )
    }

    /// Build the hierarchy recursively
    fn build_hierarchy(
        temp_tree: &mut Vec<u32>,
        dat: &mut BuildData,
        bounds: &BoundingBox,
        stats: &mut BuildStats,
    ) {
        // Create space for the first node
        temp_tree.push(3 << 30); // dummy leaf
        temp_tree.push(0);
        temp_tree.push(0);

        // Seed bbox
        let grid_box = AABound::from_bbox(bounds);
        let node_box = grid_box;

        // Seed subdivide function
        Self::subdivide(
            0,
            dat.num_prims as i32 - 1,
            temp_tree,
            dat,
            grid_box,
            node_box,
            0,
            1,
            stats,
        );
    }

    /// Recursively subdivide primitives
    fn subdivide(
        left: i32,
        right: i32,
        temp_tree: &mut Vec<u32>,
        dat: &mut BuildData,
        mut grid_box: AABound,
        mut node_box: AABound,
        node_index: usize,
        depth: usize,
        stats: &mut BuildStats,
    ) {
        let prim_count = (right - left + 1) as usize;

        // Create leaf if too few primitives or max depth reached
        if prim_count <= dat.max_prims || depth >= MAX_STACK_SIZE {
            stats.update_leaf(depth as u32, prim_count as u32);
            Self::create_node(temp_tree, node_index, left as u32, right as u32);
            return;
        }

        // Find split parameters
        let mut axis: i32 = -1;
        let mut right_orig = right;
        let mut clip_l = f32::NAN;
        let mut clip_r = f32::NAN;
        let mut prev_clip = f32::NAN;
        let mut split = f32::NAN;
        let mut was_left = true;
        let mut prev_axis: i32;
        let mut prev_split: f32;

        loop {
            prev_axis = axis;
            prev_split = split;

            // Find longest axis
            let d = grid_box.hi - grid_box.lo;
            axis = if d.x >= d.y && d.x >= d.z {
                0
            } else if d.y >= d.z {
                1
            } else {
                2
            };

            split = 0.5 * (grid_box.lo[axis as usize] + grid_box.hi[axis as usize]);

            // Partition L/R subsets
            clip_l = f32::NEG_INFINITY;
            clip_r = f32::INFINITY;
            right_orig = right;

            let mut node_l = f32::INFINITY;
            let mut node_r = f32::NEG_INFINITY;

            let mut i = left;
            let mut current_right = right;

            while i <= current_right {
                let obj = dat.indices[i as usize];
                let prim_bbox = &dat.prim_bound[obj as usize];
                let minb = prim_bbox.min[axis as usize];
                let maxb = prim_bbox.max[axis as usize];
                let center = (minb + maxb) * 0.5;

                if center <= split {
                    // Stay left
                    i += 1;
                    if clip_l < maxb {
                        clip_l = maxb;
                    }
                } else {
                    // Move to right
                    dat.indices.swap(i as usize, current_right as usize);
                    current_right -= 1;
                    if clip_r > minb {
                        clip_r = minb;
                    }
                }

                node_l = node_l.min(minb);
                node_r = node_r.max(maxb);
            }

            // Update right after partitioning
            let partitioned_right = current_right;

            // Check for empty space (BVH2 optimization)
            if node_l > node_box.lo[axis as usize] && node_r < node_box.hi[axis as usize] {
                let node_box_w = node_box.hi[axis as usize] - node_box.lo[axis as usize];
                let node_new_w = node_r - node_l;

                // Node box is too big compared to space occupied by primitives?
                if 1.3 * node_new_w < node_box_w {
                    stats.update_bvh2();
                    let next_index = temp_tree.len();

                    // Allocate child
                    temp_tree.push(0);
                    temp_tree.push(0);
                    temp_tree.push(0);

                    // Write BVH2 clip node
                    stats.update_inner();
                    temp_tree[node_index] = ((axis as u32) << 30) | (1 << 29) | (next_index as u32);
                    temp_tree[node_index + 1] = float_to_raw_int_bits(node_l);
                    temp_tree[node_index + 2] = float_to_raw_int_bits(node_r);

                    // Update nodebox and recurse
                    node_box.lo[axis as usize] = node_l;
                    node_box.hi[axis as usize] = node_r;

                    Self::subdivide(
                        left,
                        right_orig,
                        temp_tree,
                        dat,
                        grid_box,
                        node_box,
                        next_index,
                        depth + 1,
                        stats,
                    );
                    return;
                }
            }

            // Ensure we are making progress
            if partitioned_right == right_orig {
                // All left
                if prev_axis == axis && (prev_split - split).abs() < f32::EPSILON {
                    // Stuck - create leaf
                    stats.update_leaf(depth as u32, prim_count as u32);
                    Self::create_node(temp_tree, node_index, left as u32, right as u32);
                    return;
                }

                if clip_l <= split {
                    // Keep looping on left half
                    grid_box.hi[axis as usize] = split;
                    prev_clip = clip_l;
                    was_left = true;
                    continue;
                }

                grid_box.hi[axis as usize] = split;
                prev_clip = f32::NAN;
            } else if left > partitioned_right {
                // All right - restore right
                let _ = partitioned_right; // Intentionally unused after restore

                if prev_axis == axis && (prev_split - split).abs() < f32::EPSILON {
                    // Stuck - create leaf
                    stats.update_leaf(depth as u32, prim_count as u32);
                    Self::create_node(temp_tree, node_index, left as u32, right as u32);
                    return;
                }

                if clip_r >= split {
                    // Keep looping on right half
                    grid_box.lo[axis as usize] = split;
                    prev_clip = clip_r;
                    was_left = false;
                    continue;
                }

                grid_box.lo[axis as usize] = split;
                prev_clip = f32::NAN;
            } else {
                // Actually splitting
                if prev_axis != -1 && !prev_clip.is_nan() {
                    // Second time through - create previous split since it produced empty space
                    let next_index = temp_tree.len();

                    // Allocate child node
                    temp_tree.push(0);
                    temp_tree.push(0);
                    temp_tree.push(0);

                    if was_left {
                        // Create node with left child
                        stats.update_inner();
                        temp_tree[node_index] = ((prev_axis as u32) << 30) | (next_index as u32);
                        temp_tree[node_index + 1] = float_to_raw_int_bits(prev_clip);
                        temp_tree[node_index + 2] = float_to_raw_int_bits(f32::INFINITY);
                    } else {
                        // Create node with right child
                        stats.update_inner();
                        temp_tree[node_index] =
                            ((prev_axis as u32) << 30) | ((next_index - 3) as u32);
                        temp_tree[node_index + 1] = float_to_raw_int_bits(f32::NEG_INFINITY);
                        temp_tree[node_index + 2] = float_to_raw_int_bits(prev_clip);
                    }

                    // Count stats for unused leaf
                    stats.update_leaf((depth + 1) as u32, 0);

                    // Continue with new node_index
                    Self::subdivide(
                        left,
                        right_orig,
                        temp_tree,
                        dat,
                        grid_box,
                        node_box,
                        next_index,
                        depth + 1,
                        stats,
                    );
                    return;
                }

                // Split the primitives
                let actual_right = partitioned_right;
                let nl = actual_right - left + 1;
                let nr = right_orig - actual_right;

                // Compute index of child nodes
                let mut next_index = temp_tree.len();

                // Allocate left node
                if nl > 0 {
                    temp_tree.push(0);
                    temp_tree.push(0);
                    temp_tree.push(0);
                } else {
                    next_index = next_index.saturating_sub(3);
                }

                // Allocate right node
                if nr > 0 {
                    temp_tree.push(0);
                    temp_tree.push(0);
                    temp_tree.push(0);
                }

                // Write interior node
                stats.update_inner();
                temp_tree[node_index] = ((axis as u32) << 30) | (next_index as u32);
                temp_tree[node_index + 1] = float_to_raw_int_bits(clip_l);
                temp_tree[node_index + 2] = float_to_raw_int_bits(clip_r);

                // Prepare L/R child boxes
                let mut grid_box_l = grid_box;
                let mut grid_box_r = grid_box;
                let mut node_box_l = node_box;
                let mut node_box_r = node_box;

                grid_box_l.hi[axis as usize] = split;
                grid_box_r.lo[axis as usize] = split;
                node_box_l.hi[axis as usize] = clip_l;
                node_box_r.lo[axis as usize] = clip_r;

                // Recurse
                if nl > 0 {
                    Self::subdivide(
                        left,
                        actual_right,
                        temp_tree,
                        dat,
                        grid_box_l,
                        node_box_l,
                        next_index,
                        depth + 1,
                        stats,
                    );
                } else {
                    stats.update_leaf((depth + 1) as u32, 0);
                }

                if nr > 0 {
                    Self::subdivide(
                        actual_right + 1,
                        right_orig,
                        temp_tree,
                        dat,
                        grid_box_r,
                        node_box_r,
                        next_index + 3,
                        depth + 1,
                        stats,
                    );
                } else {
                    stats.update_leaf((depth + 1) as u32, 0);
                }

                return;
            }
        }
    }

    /// Create a leaf node at the given index
    fn create_node(temp_tree: &mut Vec<u32>, node_index: usize, left: u32, right: u32) {
        // Write leaf node: axis=3 (bits 30-31), count in second word
        temp_tree[node_index] = (3 << 30) | left;
        temp_tree[node_index + 1] = right - left + 1;
    }

    /// Get primitive count
    pub fn prim_count(&self) -> usize {
        self.objects.len()
    }

    /// Write BIH to file in server-compatible format
    pub fn write_to_file<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Write bounds (6 floats: lo.xyz, hi.xyz)
        writer.write_f32::<LittleEndian>(self.bounds.min.x)?;
        writer.write_f32::<LittleEndian>(self.bounds.min.y)?;
        writer.write_f32::<LittleEndian>(self.bounds.min.z)?;
        writer.write_f32::<LittleEndian>(self.bounds.max.x)?;
        writer.write_f32::<LittleEndian>(self.bounds.max.y)?;
        writer.write_f32::<LittleEndian>(self.bounds.max.z)?;

        // Write tree size and data
        let tree_size = self.tree.len() as u32;
        writer.write_u32::<LittleEndian>(tree_size)?;
        for &node in &self.tree {
            writer.write_u32::<LittleEndian>(node)?;
        }

        // Write object count and indices
        let count = self.objects.len() as u32;
        writer.write_u32::<LittleEndian>(count)?;
        for &obj in &self.objects {
            writer.write_u32::<LittleEndian>(obj)?;
        }

        Ok(())
    }
}

impl Default for BIH {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_boxes() -> Vec<BoundingBox> {
        vec![
            BoundingBox::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0)),
            BoundingBox::new(Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 1.0)),
            BoundingBox::new(Vec3::new(4.0, 0.0, 0.0), Vec3::new(5.0, 1.0, 1.0)),
            BoundingBox::new(Vec3::new(6.0, 0.0, 0.0), Vec3::new(7.0, 1.0, 1.0)),
        ]
    }

    #[test]
    fn test_bih_empty() {
        let (bih, _stats) = BIH::build(0, |_| BoundingBox::default());
        assert_eq!(bih.prim_count(), 0);
    }

    #[test]
    fn test_bih_single_prim() {
        let boxes = vec![BoundingBox::new(Vec3::ZERO, Vec3::ONE)];
        let (bih, stats) = BIH::build(boxes.len(), |i| boxes[i]);

        assert_eq!(bih.prim_count(), 1);
        assert_eq!(stats.num_leaves, 1);
    }

    #[test]
    fn test_bih_multiple_prims() {
        let boxes = create_test_boxes();
        let (bih, stats) = BIH::build(boxes.len(), |i| boxes[i]);

        assert_eq!(bih.prim_count(), 4);
        assert!(stats.num_leaves >= 1);
        assert!(bih.tree.len() >= 3);
    }

    #[test]
    fn test_bih_write_to_file() {
        let boxes = create_test_boxes();
        let (bih, _stats) = BIH::build(boxes.len(), |i| boxes[i]);

        let mut buffer = Vec::new();
        bih.write_to_file(&mut buffer).unwrap();

        // Should have written:
        // - 6 floats for bounds (24 bytes)
        // - 1 u32 for tree size + tree data
        // - 1 u32 for object count + object indices
        assert!(buffer.len() >= 24 + 4 + 4);
    }

    #[test]
    fn test_bih_many_prims() {
        // Create many boxes to stress test
        let boxes: Vec<BoundingBox> = (0..100)
            .map(|i| {
                let x = (i % 10) as f32 * 2.0;
                let y = (i / 10) as f32 * 2.0;
                BoundingBox::new(Vec3::new(x, y, 0.0), Vec3::new(x + 1.0, y + 1.0, 1.0))
            })
            .collect();

        let (bih, stats) = BIH::build(boxes.len(), |i| boxes[i]);

        assert_eq!(bih.prim_count(), 100);
        assert!(stats.num_leaves > 0);
        assert!(stats.num_nodes > 0);
    }
}
