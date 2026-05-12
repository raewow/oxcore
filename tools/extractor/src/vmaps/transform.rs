//! 3D Transformation Utilities
//!
//! Provides functions for transforming vertices, calculating bounding boxes,
//! and converting between coordinate systems.

use glam::{Mat3, Quat, Vec3};
use crate::vmaps::types::BoundingBox;

/// Convert Euler angles (radians) to rotation matrix
///
/// Applies rotations in ZYX order (yaw, pitch, roll)
pub fn euler_to_matrix(rotation: Vec3) -> Mat3 {
    let quat = euler_to_quat(rotation);
    Mat3::from_quat(quat)
}

/// Convert Euler angles (radians) to quaternion
///
/// Applies rotations in ZYX order (yaw, pitch, roll)
pub fn euler_to_quat(rotation: Vec3) -> Quat {
    Quat::from_euler(
        glam::EulerRot::ZYX,
        rotation.z, // Yaw (Z)
        rotation.y, // Pitch (Y)
        rotation.x, // Roll (X)
    )
}

/// Transform a single vertex by scale, rotation, and translation
///
/// Order of operations: scale -> rotate -> translate
pub fn transform_vertex(
    vertex: Vec3,
    scale: f32,
    rotation: &Mat3,
    position: &Vec3,
) -> Vec3 {
    rotation.mul_vec3(vertex * scale) + position
}

/// Transform multiple vertices by scale, rotation, and translation
///
/// Order of operations: scale -> rotate -> translate
pub fn transform_vertices(
    vertices: &[Vec3],
    scale: f32,
    rotation: &Mat3,
    position: &Vec3,
) -> Vec<Vec3> {
    vertices
        .iter()
        .map(|&v| transform_vertex(v, scale, rotation, position))
        .collect()
}

/// Calculate bounding box from a collection of vertices
pub fn calculate_bounding_box(vertices: &[Vec3]) -> BoundingBox {
    if vertices.is_empty() {
        return BoundingBox::default();
    }

    let mut min = vertices[0];
    let mut max = vertices[0];

    for &vertex in &vertices[1..] {
        min = min.min(vertex);
        max = max.max(vertex);
    }

    BoundingBox::new(min, max)
}

/// Transform a bounding box by scale, rotation, and translation
///
/// Transforms all 8 corners and recalculates the axis-aligned box
pub fn transform_bounding_box(
    bbox: &BoundingBox,
    scale: f32,
    rotation: &Mat3,
    position: &Vec3,
) -> BoundingBox {
    // Transform all 8 corners of the bounding box
    let corners = [
        Vec3::new(bbox.min.x, bbox.min.y, bbox.min.z),
        Vec3::new(bbox.max.x, bbox.min.y, bbox.min.z),
        Vec3::new(bbox.min.x, bbox.max.y, bbox.min.z),
        Vec3::new(bbox.max.x, bbox.max.y, bbox.min.z),
        Vec3::new(bbox.min.x, bbox.min.y, bbox.max.z),
        Vec3::new(bbox.max.x, bbox.min.y, bbox.max.z),
        Vec3::new(bbox.min.x, bbox.max.y, bbox.max.z),
        Vec3::new(bbox.max.x, bbox.max.y, bbox.max.z),
    ];

    let transformed_corners: Vec<Vec3> = corners
        .iter()
        .map(|&corner| transform_vertex(corner, scale, rotation, position))
        .collect();

    calculate_bounding_box(&transformed_corners)
}

/// Transform a normal vector by rotation only
///
/// Normals should not be affected by translation or scale
pub fn transform_normal(normal: Vec3, rotation: &Mat3) -> Vec3 {
    rotation.mul_vec3(normal).normalize()
}

/// Transform multiple normals by rotation only
pub fn transform_normals(normals: &[Vec3], rotation: &Mat3) -> Vec<Vec3> {
    normals
        .iter()
        .map(|&n| transform_normal(n, rotation))
        .collect()
}

/// Convert WoW coordinate system to standard coordinate system
///
/// WoW uses: X=East, Y=North, Z=Up
/// We may need to swap axes depending on server requirements
#[inline]
pub fn convert_coordinate_system(pos: Vec3) -> Vec3 {
    // For now, keep WoW coordinate system as-is
    // Adjust if needed based on server requirements
    pos
}

/// Calculate scale factor from uint16 scale value
///
/// WoW stores scale as u16 with value 1024 = 1.0x scale
#[inline]
pub fn decode_scale(scale: u16) -> f32 {
    (scale as f32) / 1024.0
}

/// Encode scale to uint16 format
#[inline]
pub fn encode_scale(scale: f32) -> u16 {
    (scale * 1024.0).clamp(0.0, 65535.0) as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const EPSILON: f32 = 0.001;

    fn vec3_near(a: Vec3, b: Vec3, epsilon: f32) -> bool {
        (a - b).length() < epsilon
    }

    #[test]
    fn test_euler_to_quat_identity() {
        let quat = euler_to_quat(Vec3::ZERO);
        assert!(vec3_near(quat.mul_vec3(Vec3::X), Vec3::X, EPSILON));
        assert!(vec3_near(quat.mul_vec3(Vec3::Y), Vec3::Y, EPSILON));
        assert!(vec3_near(quat.mul_vec3(Vec3::Z), Vec3::Z, EPSILON));
    }

    #[test]
    fn test_euler_to_matrix_identity() {
        let mat = euler_to_matrix(Vec3::ZERO);
        assert!(vec3_near(mat.mul_vec3(Vec3::X), Vec3::X, EPSILON));
        assert!(vec3_near(mat.mul_vec3(Vec3::Y), Vec3::Y, EPSILON));
        assert!(vec3_near(mat.mul_vec3(Vec3::Z), Vec3::Z, EPSILON));
    }

    #[test]
    fn test_euler_to_matrix_90deg_z() {
        // 90 degree rotation around Z axis
        let rotation = Vec3::new(0.0, 0.0, PI / 2.0);
        let mat = euler_to_matrix(rotation);

        // X should become Y
        assert!(vec3_near(mat.mul_vec3(Vec3::X), Vec3::Y, EPSILON));
        // Y should become -X
        assert!(vec3_near(mat.mul_vec3(Vec3::Y), -Vec3::X, EPSILON));
        // Z should remain Z
        assert!(vec3_near(mat.mul_vec3(Vec3::Z), Vec3::Z, EPSILON));
    }

    #[test]
    fn test_transform_vertex_identity() {
        let vertex = Vec3::new(1.0, 2.0, 3.0);
        let transformed = transform_vertex(vertex, 1.0, &Mat3::IDENTITY, &Vec3::ZERO);
        assert_eq!(transformed, vertex);
    }

    #[test]
    fn test_transform_vertex_scale() {
        let vertex = Vec3::new(1.0, 2.0, 3.0);
        let transformed = transform_vertex(vertex, 2.0, &Mat3::IDENTITY, &Vec3::ZERO);
        assert_eq!(transformed, Vec3::new(2.0, 4.0, 6.0));
    }

    #[test]
    fn test_transform_vertex_translation() {
        let vertex = Vec3::new(1.0, 2.0, 3.0);
        let position = Vec3::new(10.0, 20.0, 30.0);
        let transformed = transform_vertex(vertex, 1.0, &Mat3::IDENTITY, &position);
        assert_eq!(transformed, Vec3::new(11.0, 22.0, 33.0));
    }

    #[test]
    fn test_transform_vertex_combined() {
        let vertex = Vec3::new(1.0, 0.0, 0.0);
        let scale = 2.0;
        let rotation = euler_to_matrix(Vec3::new(0.0, 0.0, PI / 2.0)); // 90deg Z
        let position = Vec3::new(10.0, 20.0, 0.0);

        let transformed = transform_vertex(vertex, scale, &rotation, &position);

        // Expected: scale 1->2, rotate X->Y, translate by (10,20,0)
        // Result should be approximately (10, 22, 0)
        assert!(vec3_near(transformed, Vec3::new(10.0, 22.0, 0.0), EPSILON));
    }

    #[test]
    fn test_transform_vertices() {
        let vertices = vec![
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ];

        let transformed = transform_vertices(&vertices, 2.0, &Mat3::IDENTITY, &Vec3::ZERO);

        assert_eq!(transformed.len(), 3);
        assert_eq!(transformed[0], Vec3::new(2.0, 0.0, 0.0));
        assert_eq!(transformed[1], Vec3::new(0.0, 2.0, 0.0));
        assert_eq!(transformed[2], Vec3::new(0.0, 0.0, 2.0));
    }

    #[test]
    fn test_calculate_bounding_box_empty() {
        let bbox = calculate_bounding_box(&[]);
        assert!(!bbox.is_valid());
    }

    #[test]
    fn test_calculate_bounding_box_single() {
        let vertices = vec![Vec3::new(1.0, 2.0, 3.0)];
        let bbox = calculate_bounding_box(&vertices);
        assert_eq!(bbox.min, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(bbox.max, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_calculate_bounding_box_multiple() {
        let vertices = vec![
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.0, 3.0, 0.0),
            Vec3::new(0.0, 0.0, 4.0),
        ];

        let bbox = calculate_bounding_box(&vertices);
        assert_eq!(bbox.min, Vec3::new(-1.0, -1.0, -1.0));
        assert_eq!(bbox.max, Vec3::new(2.0, 3.0, 4.0));
    }

    #[test]
    fn test_transform_bounding_box_identity() {
        let bbox = BoundingBox::new(Vec3::ZERO, Vec3::ONE);
        let transformed = transform_bounding_box(&bbox, 1.0, &Mat3::IDENTITY, &Vec3::ZERO);
        assert!(vec3_near(transformed.min, Vec3::ZERO, EPSILON));
        assert!(vec3_near(transformed.max, Vec3::ONE, EPSILON));
    }

    #[test]
    fn test_transform_bounding_box_scale() {
        let bbox = BoundingBox::new(Vec3::ZERO, Vec3::ONE);
        let transformed = transform_bounding_box(&bbox, 2.0, &Mat3::IDENTITY, &Vec3::ZERO);
        assert!(vec3_near(transformed.min, Vec3::ZERO, EPSILON));
        assert!(vec3_near(transformed.max, Vec3::splat(2.0), EPSILON));
    }

    #[test]
    fn test_transform_bounding_box_translation() {
        let bbox = BoundingBox::new(Vec3::ZERO, Vec3::ONE);
        let position = Vec3::new(10.0, 20.0, 30.0);
        let transformed = transform_bounding_box(&bbox, 1.0, &Mat3::IDENTITY, &position);
        assert!(vec3_near(transformed.min, position, EPSILON));
        assert!(vec3_near(transformed.max, position + Vec3::ONE, EPSILON));
    }

    #[test]
    fn test_transform_normal() {
        let normal = Vec3::X;
        let rotation = euler_to_matrix(Vec3::new(0.0, 0.0, PI / 2.0)); // 90deg Z
        let transformed = transform_normal(normal, &rotation);

        // X normal should become Y normal
        assert!(vec3_near(transformed, Vec3::Y, EPSILON));
    }

    #[test]
    fn test_transform_normals() {
        let normals = vec![Vec3::X, Vec3::Y, Vec3::Z];
        let rotation = euler_to_matrix(Vec3::new(0.0, 0.0, PI / 2.0));
        let transformed = transform_normals(&normals, &rotation);

        assert_eq!(transformed.len(), 3);
        assert!(vec3_near(transformed[0], Vec3::Y, EPSILON));
        assert!(vec3_near(transformed[1], -Vec3::X, EPSILON));
        assert!(vec3_near(transformed[2], Vec3::Z, EPSILON));
    }

    #[test]
    fn test_decode_scale() {
        assert_eq!(decode_scale(1024), 1.0);
        assert_eq!(decode_scale(2048), 2.0);
        assert_eq!(decode_scale(512), 0.5);
        assert_eq!(decode_scale(0), 0.0);
    }

    #[test]
    fn test_encode_scale() {
        assert_eq!(encode_scale(1.0), 1024);
        assert_eq!(encode_scale(2.0), 2048);
        assert_eq!(encode_scale(0.5), 512);
        assert_eq!(encode_scale(0.0), 0);
    }

    #[test]
    fn test_scale_roundtrip() {
        let original = 1.5;
        let encoded = encode_scale(original);
        let decoded = decode_scale(encoded);
        assert!((original - decoded).abs() < 0.01);
    }
}
