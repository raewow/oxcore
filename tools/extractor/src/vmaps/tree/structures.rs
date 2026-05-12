//! BVH Tree Structures
//!
//! Bounding Volume Hierarchy for spatial organization of collision geometry.

use glam::Vec3;
use crate::vmaps::types::BoundingBox;

/// Triangle data stored in BVH leaf nodes
#[derive(Debug, Clone)]
pub struct TriangleData {
    /// Triangle vertices
    pub vertices: [Vec3; 3],
    /// Material ID
    pub material_id: u16,
    /// Normal vector
    pub normal: Vec3,
}

impl TriangleData {
    /// Create new triangle data
    pub fn new(v0: Vec3, v1: Vec3, v2: Vec3, material_id: u16) -> Self {
        let normal = Self::calculate_normal(v0, v1, v2);
        Self {
            vertices: [v0, v1, v2],
            material_id,
            normal,
        }
    }

    /// Calculate triangle normal
    fn calculate_normal(v0: Vec3, v1: Vec3, v2: Vec3) -> Vec3 {
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        edge1.cross(edge2).normalize_or_zero()
    }

    /// Get triangle centroid
    pub fn centroid(&self) -> Vec3 {
        (self.vertices[0] + self.vertices[1] + self.vertices[2]) / 3.0
    }

    /// Calculate bounding box for this triangle
    pub fn bounding_box(&self) -> BoundingBox {
        let mut bbox = BoundingBox::new(self.vertices[0], self.vertices[0]);
        bbox.expand_point(self.vertices[1]);
        bbox.expand_point(self.vertices[2]);
        bbox
    }
}

/// BVH Node - either a branch or a leaf
#[derive(Debug, Clone)]
pub enum BVHNode {
    /// Branch node with two children
    Branch {
        /// Bounding box containing all children
        bbox: BoundingBox,
        /// Left child index
        left: usize,
        /// Right child index
        right: usize,
    },
    /// Leaf node containing triangles
    Leaf {
        /// Bounding box containing all triangles
        bbox: BoundingBox,
        /// Triangle indices in the tree's triangle array
        triangle_indices: Vec<usize>,
    },
}

impl BVHNode {
    /// Get the bounding box of this node
    pub fn bounding_box(&self) -> &BoundingBox {
        match self {
            BVHNode::Branch { bbox, .. } => bbox,
            BVHNode::Leaf { bbox, .. } => bbox,
        }
    }

    /// Check if this is a leaf node
    pub fn is_leaf(&self) -> bool {
        matches!(self, BVHNode::Leaf { .. })
    }

    /// Get triangle count for leaf nodes
    pub fn triangle_count(&self) -> usize {
        match self {
            BVHNode::Leaf { triangle_indices, .. } => triangle_indices.len(),
            BVHNode::Branch { .. } => 0,
        }
    }
}

/// Complete BVH Tree
#[derive(Debug, Clone)]
pub struct BVHTree {
    /// All nodes in the tree (index 0 = root)
    pub nodes: Vec<BVHNode>,
    /// All triangles referenced by leaf nodes
    pub triangles: Vec<TriangleData>,
    /// Overall bounding box
    pub bounds: BoundingBox,
}

impl BVHTree {
    /// Create new empty BVH tree
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            triangles: Vec::new(),
            bounds: BoundingBox::default(),
        }
    }

    /// Create BVH tree from triangles
    pub fn from_triangles(triangles: Vec<TriangleData>) -> Self {
        if triangles.is_empty() {
            return Self::new();
        }

        // Calculate overall bounds
        let mut bounds = triangles[0].bounding_box();
        for tri in &triangles[1..] {
            bounds = bounds.union(&tri.bounding_box());
        }

        Self {
            nodes: Vec::new(),
            triangles,
            bounds,
        }
    }

    /// Get the root node (if tree is built)
    pub fn root(&self) -> Option<&BVHNode> {
        self.nodes.last()
    }

    /// Get total triangle count
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// Get total node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Check if tree is empty
    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }
}

impl Default for BVHTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triangle_data_creation() {
        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(1.0, 0.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);

        let tri = TriangleData::new(v0, v1, v2, 0);

        assert_eq!(tri.vertices[0], v0);
        assert_eq!(tri.vertices[1], v1);
        assert_eq!(tri.vertices[2], v2);
        assert_eq!(tri.material_id, 0);

        // Normal should point up in Z
        assert!(tri.normal.z > 0.99);
    }

    #[test]
    fn test_triangle_centroid() {
        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(3.0, 0.0, 0.0);
        let v2 = Vec3::new(0.0, 3.0, 0.0);

        let tri = TriangleData::new(v0, v1, v2, 0);
        let centroid = tri.centroid();

        assert_eq!(centroid, Vec3::new(1.0, 1.0, 0.0));
    }

    #[test]
    fn test_triangle_bounding_box() {
        let v0 = Vec3::new(1.0, 2.0, 3.0);
        let v1 = Vec3::new(4.0, 1.0, 2.0);
        let v2 = Vec3::new(2.0, 3.0, 1.0);

        let tri = TriangleData::new(v0, v1, v2, 0);
        let bbox = tri.bounding_box();

        assert_eq!(bbox.min, Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(bbox.max, Vec3::new(4.0, 3.0, 3.0));
    }

    #[test]
    fn test_bvh_node_leaf() {
        let bbox = BoundingBox::new(Vec3::ZERO, Vec3::ONE);
        let node = BVHNode::Leaf {
            bbox,
            triangle_indices: vec![0, 1, 2],
        };

        assert!(node.is_leaf());
        assert_eq!(node.triangle_count(), 3);
        assert_eq!(node.bounding_box().min, Vec3::ZERO);
    }

    #[test]
    fn test_bvh_node_branch() {
        let bbox = BoundingBox::new(Vec3::ZERO, Vec3::ONE);
        let node = BVHNode::Branch {
            bbox,
            left: 1,
            right: 2,
        };

        assert!(!node.is_leaf());
        assert_eq!(node.triangle_count(), 0);
    }

    #[test]
    fn test_bvh_tree_empty() {
        let tree = BVHTree::new();
        assert!(tree.is_empty());
        assert_eq!(tree.triangle_count(), 0);
        assert_eq!(tree.node_count(), 0);
    }

    #[test]
    fn test_bvh_tree_from_triangles() {
        let triangles = vec![
            TriangleData::new(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                0,
            ),
            TriangleData::new(
                Vec3::new(2.0, 0.0, 0.0),
                Vec3::new(3.0, 0.0, 0.0),
                Vec3::new(2.0, 1.0, 0.0),
                0,
            ),
        ];

        let tree = BVHTree::from_triangles(triangles);

        assert!(!tree.is_empty());
        assert_eq!(tree.triangle_count(), 2);

        // Bounds should contain all triangles
        assert_eq!(tree.bounds.min, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(tree.bounds.max, Vec3::new(3.0, 1.0, 0.0));
    }
}
