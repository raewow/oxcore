//! Placing a loaded `.vmo` world model into world space.
//!
//! Shared by the static tile tree and the dynamic gameobject tree so both apply
//! the same scale → Z-rotation → translation transform.

use std::sync::Arc;

use super::bsp_tree::BSPModelInstance;
use super::file_loader::WorldModelData;
use super::types::{BoundingBox, LiquidLevel, ModelType};
use oxcore_shared::protocol::Position;

/// Transform every group of a world model into world space.
///
/// `position.o` supplies the rotation about Z. Returns one
/// [`BSPModelInstance`] per model group.
pub fn place_world_model(
    world_model: &WorldModelData,
    position: Position,
    scale: f32,
    model_id: u32,
    model_type: ModelType,
) -> Vec<Arc<BSPModelInstance>> {
    let (cos_o, sin_o) = if position.o != 0.0 {
        (position.o.cos(), position.o.sin())
    } else {
        (1.0, 0.0)
    };
    let rotate = position.o != 0.0;

    let mut placed = Vec::with_capacity(world_model.groups.len());

    for group in &world_model.groups {
        let mut triangles = group.triangles.clone();

        for triangle in &mut triangles {
            for v in [&mut triangle.v0, &mut triangle.v1, &mut triangle.v2] {
                // Scale about the model origin.
                v.x *= scale;
                v.y *= scale;
                v.z *= scale;

                // Rotate about Z.
                if rotate {
                    let (x, y) = (v.x, v.y);
                    v.x = x * cos_o - y * sin_o;
                    v.y = x * sin_o + y * cos_o;
                }

                // Translate into world space.
                v.x += position.x;
                v.y += position.y;
                v.z += position.z;
            }
        }

        let mut bbox = BoundingBox {
            min: Position::new(
                group.bounding_box.min.x * scale,
                group.bounding_box.min.y * scale,
                group.bounding_box.min.z * scale,
                0.0,
            ),
            max: Position::new(
                group.bounding_box.max.x * scale,
                group.bounding_box.max.y * scale,
                group.bounding_box.max.z * scale,
                0.0,
            ),
        };

        // Rotating a box is not closed under axis alignment, so re-fit the box
        // around all eight rotated corners.
        if rotate {
            let corners = [
                (bbox.min.x, bbox.min.y, bbox.min.z),
                (bbox.max.x, bbox.min.y, bbox.min.z),
                (bbox.min.x, bbox.max.y, bbox.min.z),
                (bbox.max.x, bbox.max.y, bbox.min.z),
                (bbox.min.x, bbox.min.y, bbox.max.z),
                (bbox.max.x, bbox.min.y, bbox.max.z),
                (bbox.min.x, bbox.max.y, bbox.max.z),
                (bbox.max.x, bbox.max.y, bbox.max.z),
            ];

            let mut min = (f32::INFINITY, f32::INFINITY, f32::INFINITY);
            let mut max = (f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);

            for (x, y, z) in corners {
                let rot_x = x * cos_o - y * sin_o;
                let rot_y = x * sin_o + y * cos_o;

                min = (min.0.min(rot_x), min.1.min(rot_y), min.2.min(z));
                max = (max.0.max(rot_x), max.1.max(rot_y), max.2.max(z));
            }

            bbox = BoundingBox {
                min: Position::new(min.0, min.1, min.2, 0.0),
                max: Position::new(max.0, max.1, max.2, 0.0),
            };
        }

        bbox.min.x += position.x;
        bbox.min.y += position.y;
        bbox.min.z += position.z;
        bbox.max.x += position.x;
        bbox.max.y += position.y;
        bbox.max.z += position.z;

        // Liquid planes are horizontal, so only the Z offset matters.
        let liquid_data = group.liquid_data.as_ref().map(|ld| LiquidLevel {
            level: ld.level + position.z,
            floor: ld.floor + position.z,
            liquid_type: ld.liquid_type,
        });

        placed.push(Arc::new(BSPModelInstance {
            model_id,
            model_type,
            bounding_box: bbox,
            triangles,
            liquid_data,
        }));
    }

    placed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pathfinding::vmap::file_loader::{GroupModel, Triangle};

    fn unit_triangle_model() -> WorldModelData {
        WorldModelData {
            model_name: "Test.wmo".to_string(),
            groups: vec![GroupModel {
                bounding_box: BoundingBox {
                    min: Position::new(0.0, 0.0, 0.0, 0.0),
                    max: Position::new(1.0, 1.0, 1.0, 0.0),
                },
                triangles: vec![Triangle {
                    v0: Position::new(0.0, 0.0, 0.0, 0.0),
                    v1: Position::new(1.0, 0.0, 0.0, 0.0),
                    v2: Position::new(0.0, 1.0, 0.0, 0.0),
                }],
                liquid_data: None,
            }],
        }
    }

    #[test]
    fn translation_only_shifts_geometry() {
        let model = unit_triangle_model();
        let placed = place_world_model(
            &model,
            Position::new(10.0, 20.0, 30.0, 0.0),
            1.0,
            7,
            ModelType::WMO,
        );

        assert_eq!(placed.len(), 1);
        let inst = &placed[0];
        assert_eq!(inst.model_id, 7);
        assert_eq!(inst.triangles[0].v0.x, 10.0);
        assert_eq!(inst.triangles[0].v0.y, 20.0);
        assert_eq!(inst.triangles[0].v0.z, 30.0);
        assert_eq!(inst.bounding_box.min.x, 10.0);
        assert_eq!(inst.bounding_box.max.z, 31.0);
    }

    #[test]
    fn scale_is_applied_before_translation() {
        let model = unit_triangle_model();
        let placed = place_world_model(
            &model,
            Position::new(0.0, 0.0, 0.0, 0.0),
            2.0,
            0,
            ModelType::M2,
        );

        assert_eq!(placed[0].triangles[0].v1.x, 2.0);
        assert_eq!(placed[0].bounding_box.max.x, 2.0);
    }

    #[test]
    fn quarter_turn_rotates_about_z() {
        let model = unit_triangle_model();
        let placed = place_world_model(
            &model,
            Position::new(0.0, 0.0, 0.0, std::f32::consts::FRAC_PI_2),
            1.0,
            0,
            ModelType::WMO,
        );

        // (1, 0) rotated 90° about Z becomes (0, 1).
        let v1 = placed[0].triangles[0].v1;
        assert!(v1.x.abs() < 1e-5, "x was {}", v1.x);
        assert!((v1.y - 1.0).abs() < 1e-5, "y was {}", v1.y);

        // The re-fitted box must still contain the rotated geometry.
        let bbox = placed[0].bounding_box;
        assert!(bbox.min.x <= v1.x && v1.x <= bbox.max.x);
        assert!(bbox.min.y <= v1.y && v1.y <= bbox.max.y);
    }

    #[test]
    fn liquid_plane_is_offset_by_z_only() {
        let mut model = unit_triangle_model();
        model.groups[0].liquid_data = Some(crate::pathfinding::vmap::file_loader::LiquidData {
            level: 1.0,
            floor: 0.0,
            liquid_type: 2,
        });

        let placed = place_world_model(
            &model,
            Position::new(100.0, 200.0, 5.0, 0.0),
            1.0,
            0,
            ModelType::WMO,
        );

        let liquid = placed[0].liquid_data.as_ref().unwrap();
        assert_eq!(liquid.level, 6.0);
        assert_eq!(liquid.floor, 5.0);
        assert_eq!(liquid.liquid_type, 2);
    }
}
