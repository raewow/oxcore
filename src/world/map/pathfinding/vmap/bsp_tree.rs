//! BSP (Binary Space Partition) tree for efficient spatial queries

use super::file_loader::Triangle;
use super::types::{AreaInfo, BoundingBox, LiquidLevel, ModelType};
use crate::shared::protocol::Position;
use std::sync::Arc;

/// BSP tree node
#[derive(Debug)]
pub enum BSPNode {
    /// Internal node (splits space)
    Internal {
        axis: Axis,
        split_value: f32,
        left: Box<BSPNode>,
        right: Box<BSPNode>,
    },
    /// Leaf node (contains model instances)
    Leaf { models: Vec<Arc<BSPModelInstance>> },
}

/// Axis for space partitioning
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

/// Model instance reference for BSP tree
#[derive(Debug, Clone)]
pub struct BSPModelInstance {
    pub model_id: u32,
    pub model_type: ModelType,
    pub bounding_box: BoundingBox,
    pub triangles: Vec<Triangle>,
    pub liquid_data: Option<LiquidLevel>,
}

/// BSP tree for spatial queries
pub struct BSPTree {
    root: Option<BSPNode>,
    bounds: BoundingBox,
}

impl BSPTree {
    pub fn new(bounds: BoundingBox) -> Self {
        Self { root: None, bounds }
    }

    /// Build BSP tree from model instances
    pub fn build(&mut self, models: Vec<Arc<BSPModelInstance>>) {
        if models.is_empty() {
            self.root = None;
            return;
        }

        self.root = Some(self.build_node(models, &self.bounds, 0));
    }

    fn build_node(
        &self,
        models: Vec<Arc<BSPModelInstance>>,
        bounds: &BoundingBox,
        depth: usize,
    ) -> BSPNode {
        if depth > 20 || models.len() <= 4 {
            return BSPNode::Leaf { models };
        }

        let size_x = bounds.max.x - bounds.min.x;
        let size_y = bounds.max.y - bounds.min.y;
        let size_z = bounds.max.z - bounds.min.z;

        let axis = if size_x >= size_y && size_x >= size_z {
            Axis::X
        } else if size_y >= size_z {
            Axis::Y
        } else {
            Axis::Z
        };

        let mut values: Vec<f32> = models
            .iter()
            .map(|m| match axis {
                Axis::X => m.bounding_box.min.x,
                Axis::Y => m.bounding_box.min.y,
                Axis::Z => m.bounding_box.min.z,
            })
            .collect();
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let split_value = values[values.len() / 2];

        let mut left_models = Vec::new();
        let mut right_models = Vec::new();

        for model in models {
            let model_min = match axis {
                Axis::X => model.bounding_box.min.x,
                Axis::Y => model.bounding_box.min.y,
                Axis::Z => model.bounding_box.min.z,
            };

            if model_min < split_value {
                left_models.push(model);
            } else {
                right_models.push(model);
            }
        }

        let mut left_bounds = *bounds;
        let mut right_bounds = *bounds;

        match axis {
            Axis::X => {
                left_bounds.max.x = split_value;
                right_bounds.min.x = split_value;
            }
            Axis::Y => {
                left_bounds.max.y = split_value;
                right_bounds.min.y = split_value;
            }
            Axis::Z => {
                left_bounds.max.z = split_value;
                right_bounds.min.z = split_value;
            }
        }

        let left = Box::new(self.build_node(left_models, &left_bounds, depth + 1));
        let right = Box::new(self.build_node(right_models, &right_bounds, depth + 1));

        BSPNode::Internal {
            axis,
            split_value,
            left,
            right,
        }
    }

    /// Raycast through BSP tree. Returns true if ray hits any geometry.
    pub fn raycast(&self, start: &Position, end: &Position) -> bool {
        self.raycast_with_filter(start, end, false)
    }

    /// Raycast with model type filtering
    pub fn raycast_with_filter(&self, start: &Position, end: &Position, ignore_m2: bool) -> bool {
        if let Some(ref root) = self.root {
            self.raycast_node_with_filter(root, start, end, ignore_m2)
        } else {
            false
        }
    }

    fn raycast_node_with_filter(
        &self,
        node: &BSPNode,
        start: &Position,
        end: &Position,
        ignore_m2: bool,
    ) -> bool {
        match node {
            BSPNode::Internal {
                axis,
                split_value,
                left,
                right,
            } => {
                let start_val = match axis {
                    Axis::X => start.x,
                    Axis::Y => start.y,
                    Axis::Z => start.z,
                };
                let end_val = match axis {
                    Axis::X => end.x,
                    Axis::Y => end.y,
                    Axis::Z => end.z,
                };

                if start_val < *split_value && end_val < *split_value {
                    self.raycast_node_with_filter(left, start, end, ignore_m2)
                } else if start_val >= *split_value && end_val >= *split_value {
                    self.raycast_node_with_filter(right, start, end, ignore_m2)
                } else {
                    self.raycast_node_with_filter(left, start, end, ignore_m2)
                        || self.raycast_node_with_filter(right, start, end, ignore_m2)
                }
            }
            BSPNode::Leaf { models } => {
                for model in models {
                    if ignore_m2 && model.model_type == ModelType::M2 {
                        continue;
                    }

                    if self.ray_intersects_bounding_box(start, end, &model.bounding_box) {
                        for triangle in &model.triangles {
                            if self.ray_intersects_triangle(start, end, triangle) {
                                return true;
                            }
                        }
                    }
                }
                false
            }
        }
    }

    fn ray_intersects_bounding_box(
        &self,
        start: &Position,
        end: &Position,
        bbox: &BoundingBox,
    ) -> bool {
        let dir = Position::new(end.x - start.x, end.y - start.y, end.z - start.z, 0.0);
        let inv_dir = Position::new(
            if dir.x != 0.0 {
                1.0 / dir.x
            } else {
                f32::INFINITY
            },
            if dir.y != 0.0 {
                1.0 / dir.y
            } else {
                f32::INFINITY
            },
            if dir.z != 0.0 {
                1.0 / dir.z
            } else {
                f32::INFINITY
            },
            0.0,
        );

        let t1 = (bbox.min.x - start.x) * inv_dir.x;
        let t2 = (bbox.max.x - start.x) * inv_dir.x;
        let t3 = (bbox.min.y - start.y) * inv_dir.y;
        let t4 = (bbox.max.y - start.y) * inv_dir.y;
        let t5 = (bbox.min.z - start.z) * inv_dir.z;
        let t6 = (bbox.max.z - start.z) * inv_dir.z;

        let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
        let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

        tmax >= tmin.max(0.0) && tmin <= 1.0
    }

    /// Query height at position
    pub fn get_height(&self, pos: &Position, max_search_dist: f32) -> Option<f32> {
        if let Some(ref root) = self.root {
            self.get_height_node(root, pos, max_search_dist)
        } else {
            None
        }
    }

    fn get_height_node(&self, node: &BSPNode, pos: &Position, max_search_dist: f32) -> Option<f32> {
        match node {
            BSPNode::Internal {
                axis,
                split_value,
                left,
                right,
            } => {
                let pos_val = match axis {
                    Axis::X => pos.x,
                    Axis::Y => pos.y,
                    Axis::Z => pos.z,
                };

                if (pos_val - split_value).abs() < max_search_dist {
                    let left_height = self.get_height_node(left, pos, max_search_dist);
                    let right_height = self.get_height_node(right, pos, max_search_dist);

                    match (left_height, right_height) {
                        (Some(lh), Some(rh)) => {
                            let dist_l = (pos.z - lh).abs();
                            let dist_r = (pos.z - rh).abs();
                            Some(if dist_l < dist_r { lh } else { rh })
                        }
                        (Some(h), None) | (None, Some(h)) => Some(h),
                        (None, None) => None,
                    }
                } else if pos_val < *split_value {
                    self.get_height_node(left, pos, max_search_dist)
                } else {
                    self.get_height_node(right, pos, max_search_dist)
                }
            }
            BSPNode::Leaf { models } => {
                let mut best_height: Option<f32> = None;
                let mut best_dist = max_search_dist;

                for model in models {
                    if !model.bounding_box.contains(pos) {
                        continue;
                    }

                    for triangle in &model.triangles {
                        if let Some(height) =
                            self.get_height_from_triangle(pos, triangle, max_search_dist)
                        {
                            let dist = (pos.z - height).abs();
                            if dist < best_dist {
                                best_dist = dist;
                                best_height = Some(height);
                            }
                        }
                    }
                }

                best_height
            }
        }
    }

    fn get_height_from_triangle(
        &self,
        pos: &Position,
        triangle: &Triangle,
        max_search_dist: f32,
    ) -> Option<f32> {
        let v0 = &triangle.v0;
        let v1 = &triangle.v1;
        let v2 = &triangle.v2;

        let edge1 = Position::new(v1.x - v0.x, v1.y - v0.y, v1.z - v0.z, 0.0);
        let edge2 = Position::new(v2.x - v0.x, v2.y - v0.y, v2.z - v0.z, 0.0);

        // Cross product for normal
        let normal = Position::new(
            edge1.y * edge2.z - edge1.z * edge2.y,
            edge1.z * edge2.x - edge1.x * edge2.z,
            edge1.x * edge2.y - edge1.y * edge2.x,
            0.0,
        );

        let normal_len = (normal.x * normal.x + normal.y * normal.y + normal.z * normal.z).sqrt();
        if normal_len < 0.0001 {
            return None;
        }

        let normal = Position::new(
            normal.x / normal_len,
            normal.y / normal_len,
            normal.z / normal_len,
            0.0,
        );

        if normal.z.abs() < 0.5 {
            return None; // Too steep
        }

        let d = -(normal.x * v0.x + normal.y * v0.y + normal.z * v0.z);
        let height = -(normal.x * pos.x + normal.y * pos.y + d) / normal.z;

        if !self.point_in_triangle_2d(pos, triangle) {
            return None;
        }

        if (pos.z - height).abs() > max_search_dist {
            return None;
        }

        if normal.z > 0.0 && pos.z < height {
            return None;
        }
        if normal.z < 0.0 && pos.z > height {
            return None;
        }

        Some(height)
    }

    fn point_in_triangle_2d(&self, pos: &Position, triangle: &Triangle) -> bool {
        let v0 = &triangle.v0;
        let v1 = &triangle.v1;
        let v2 = &triangle.v2;

        let denom = (v1.y - v2.y) * (v0.x - v2.x) + (v2.x - v1.x) * (v0.y - v2.y);
        if denom.abs() < 0.0001 {
            return false;
        }

        let a = ((v1.y - v2.y) * (pos.x - v2.x) + (v2.x - v1.x) * (pos.y - v2.y)) / denom;
        let b = ((v2.y - v0.y) * (pos.x - v2.x) + (v0.x - v2.x) * (pos.y - v2.y)) / denom;
        let c = 1.0 - a - b;

        a >= 0.0 && b >= 0.0 && c >= 0.0
    }

    /// Möller-Trumbore ray-triangle intersection
    fn ray_intersects_triangle(
        &self,
        start: &Position,
        end: &Position,
        triangle: &Triangle,
    ) -> bool {
        let v0 = &triangle.v0;
        let v1 = &triangle.v1;
        let v2 = &triangle.v2;

        let edge1 = Position::new(v1.x - v0.x, v1.y - v0.y, v1.z - v0.z, 0.0);
        let edge2 = Position::new(v2.x - v0.x, v2.y - v0.y, v2.z - v0.z, 0.0);
        let ray_dir = Position::new(end.x - start.x, end.y - start.y, end.z - start.z, 0.0);

        let h = Position::new(
            ray_dir.y * edge2.z - ray_dir.z * edge2.y,
            ray_dir.z * edge2.x - ray_dir.x * edge2.z,
            ray_dir.x * edge2.y - ray_dir.y * edge2.x,
            0.0,
        );

        let a = edge1.x * h.x + edge1.y * h.y + edge1.z * h.z;
        if a.abs() < 0.0001 {
            return false;
        }

        let f = 1.0 / a;
        let s = Position::new(start.x - v0.x, start.y - v0.y, start.z - v0.z, 0.0);

        let u = f * (s.x * h.x + s.y * h.y + s.z * h.z);
        if u < 0.0 || u > 1.0 {
            return false;
        }

        let q = Position::new(
            s.y * edge1.z - s.z * edge1.y,
            s.z * edge1.x - s.x * edge1.z,
            s.x * edge1.y - s.y * edge1.x,
            0.0,
        );

        let v = f * (ray_dir.x * q.x + ray_dir.y * q.y + ray_dir.z * q.z);
        if v < 0.0 || u + v > 1.0 {
            return false;
        }

        let t = f * (edge2.x * q.x + edge2.y * q.y + edge2.z * q.z);
        t >= 0.0 && t <= 1.0
    }

    /// Get area information at position
    pub fn get_area_info(&self, pos: &Position) -> Option<AreaInfo> {
        if let Some(ref root) = self.root {
            self.get_area_info_node(root, pos)
        } else {
            None
        }
    }

    fn get_area_info_node(&self, node: &BSPNode, pos: &Position) -> Option<AreaInfo> {
        match node {
            BSPNode::Internal {
                axis,
                split_value,
                left,
                right,
            } => {
                let pos_val = match axis {
                    Axis::X => pos.x,
                    Axis::Y => pos.y,
                    Axis::Z => pos.z,
                };

                if pos_val < *split_value {
                    self.get_area_info_node(left, pos)
                } else {
                    self.get_area_info_node(right, pos)
                }
            }
            BSPNode::Leaf { models } => {
                for model in models {
                    if model.bounding_box.contains(pos) {
                        for triangle in &model.triangles {
                            if self.point_in_triangle_2d(pos, triangle) {
                                return Some(AreaInfo {
                                    z: pos.z,
                                    flags: 0,
                                    adt_id: 0,
                                    root_id: 0,
                                    group_id: 0,
                                });
                            }
                        }
                    }
                }
                None
            }
        }
    }

    /// Get liquid level at position
    pub fn get_liquid_level(
        &self,
        pos: &Position,
        req_liquid_type_mask: u8,
    ) -> Option<LiquidLevel> {
        if let Some(ref root) = self.root {
            self.get_liquid_level_node(root, pos, req_liquid_type_mask)
        } else {
            None
        }
    }

    fn get_liquid_level_node(
        &self,
        node: &BSPNode,
        pos: &Position,
        req_liquid_type_mask: u8,
    ) -> Option<LiquidLevel> {
        match node {
            BSPNode::Internal {
                axis,
                split_value,
                left,
                right,
            } => {
                let pos_val = match axis {
                    Axis::X => pos.x,
                    Axis::Y => pos.y,
                    Axis::Z => pos.z,
                };

                if pos_val < *split_value {
                    self.get_liquid_level_node(left, pos, req_liquid_type_mask)
                } else {
                    self.get_liquid_level_node(right, pos, req_liquid_type_mask)
                }
            }
            BSPNode::Leaf { models } => {
                for model in models {
                    if model.bounding_box.contains(pos) {
                        if let Some(ref liquid) = model.liquid_data {
                            let liquid_type_u8 = (liquid.liquid_type & 0xFF) as u8;
                            if (liquid_type_u8 & req_liquid_type_mask) != 0 {
                                return Some(liquid.clone());
                            }
                        }
                    }
                }
                None
            }
        }
    }

    /// Raycast with hit position
    pub fn raycast_hit_pos(&self, start: &Position, end: &Position) -> Option<Position> {
        if let Some(ref root) = self.root {
            self.raycast_hit_pos_node(root, start, end)
        } else {
            None
        }
    }

    fn raycast_hit_pos_node(
        &self,
        node: &BSPNode,
        start: &Position,
        end: &Position,
    ) -> Option<Position> {
        match node {
            BSPNode::Internal {
                axis,
                split_value,
                left,
                right,
            } => {
                let start_val = match axis {
                    Axis::X => start.x,
                    Axis::Y => start.y,
                    Axis::Z => start.z,
                };
                let end_val = match axis {
                    Axis::X => end.x,
                    Axis::Y => end.y,
                    Axis::Z => end.z,
                };

                if start_val < *split_value && end_val < *split_value {
                    self.raycast_hit_pos_node(left, start, end)
                } else if start_val >= *split_value && end_val >= *split_value {
                    self.raycast_hit_pos_node(right, start, end)
                } else {
                    let left_hit = self.raycast_hit_pos_node(left, start, end);
                    let right_hit = self.raycast_hit_pos_node(right, start, end);

                    match (left_hit, right_hit) {
                        (Some(lh), Some(rh)) => {
                            let dist_l = distance_3d(start, &lh);
                            let dist_r = distance_3d(start, &rh);
                            Some(if dist_l < dist_r { lh } else { rh })
                        }
                        (Some(h), None) | (None, Some(h)) => Some(h),
                        (None, None) => None,
                    }
                }
            }
            BSPNode::Leaf { models } => {
                let mut closest_hit: Option<Position> = None;
                let mut closest_dist = f32::INFINITY;

                for model in models {
                    if self.ray_intersects_bounding_box(start, end, &model.bounding_box) {
                        for triangle in &model.triangles {
                            if let Some(hit_pos) =
                                self.ray_triangle_intersection_point(start, end, triangle)
                            {
                                let dist = distance_3d(start, &hit_pos);
                                if dist < closest_dist {
                                    closest_dist = dist;
                                    closest_hit = Some(hit_pos);
                                }
                            }
                        }
                    }
                }

                closest_hit
            }
        }
    }

    fn ray_triangle_intersection_point(
        &self,
        start: &Position,
        end: &Position,
        triangle: &Triangle,
    ) -> Option<Position> {
        let v0 = &triangle.v0;
        let v1 = &triangle.v1;
        let v2 = &triangle.v2;

        let edge1 = Position::new(v1.x - v0.x, v1.y - v0.y, v1.z - v0.z, 0.0);
        let edge2 = Position::new(v2.x - v0.x, v2.y - v0.y, v2.z - v0.z, 0.0);
        let ray_dir = Position::new(end.x - start.x, end.y - start.y, end.z - start.z, 0.0);

        let h = Position::new(
            ray_dir.y * edge2.z - ray_dir.z * edge2.y,
            ray_dir.z * edge2.x - ray_dir.x * edge2.z,
            ray_dir.x * edge2.y - ray_dir.y * edge2.x,
            0.0,
        );

        let a = edge1.x * h.x + edge1.y * h.y + edge1.z * h.z;
        if a.abs() < 0.0001 {
            return None;
        }

        let f = 1.0 / a;
        let s = Position::new(start.x - v0.x, start.y - v0.y, start.z - v0.z, 0.0);

        let u = f * (s.x * h.x + s.y * h.y + s.z * h.z);
        if u < 0.0 || u > 1.0 {
            return None;
        }

        let q = Position::new(
            s.y * edge1.z - s.z * edge1.y,
            s.z * edge1.x - s.x * edge1.z,
            s.x * edge1.y - s.y * edge1.x,
            0.0,
        );

        let v = f * (ray_dir.x * q.x + ray_dir.y * q.y + ray_dir.z * q.z);
        if v < 0.0 || u + v > 1.0 {
            return None;
        }

        let t = f * (edge2.x * q.x + edge2.y * q.y + edge2.z * q.z);
        if t < 0.0 || t > 1.0 {
            return None;
        }

        Some(Position::new(
            start.x + ray_dir.x * t,
            start.y + ray_dir.y * t,
            start.z + ray_dir.z * t,
            0.0,
        ))
    }
}

fn distance_3d(a: &Position, b: &Position) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}
