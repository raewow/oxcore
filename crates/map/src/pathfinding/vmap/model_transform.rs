//! Placing a loaded `.vmo` world model into world space.
//!
//! Static map models are stored in the extractor's internal frame: positions
//! are mirrored in X and Y about the centre of the 64x64 tile grid, and
//! rotations are Euler angles in degrees. Gameobject models are placed in
//! world space with a rotation about Z only. Both go through one
//! [`ModelTransform`] so geometry, bounds and liquid are placed identically.

use std::sync::Arc;

use super::bsp_tree::BSPModelInstance;
use super::file_loader::WorldModelData;
use super::types::{BoundingBox, LiquidLevel, ModelInstance, ModelType};
use oxcore_shared::protocol::Position;

/// Half the width of the 64x64 tile grid; the internal frame mirrors world X
/// and Y about this value.
const GRID_MID: f32 = 0.5 * 64.0 * 533.333_33;
/// Side of one WMO liquid cell.
const LIQUID_TILE_SIZE: f32 = 533.333_33 / 128.0;

/// Model space to world space: scale, rotate, translate, then (for static
/// models) leave the internal frame.
#[derive(Debug, Clone, Copy)]
pub struct ModelTransform {
    /// Translation, in the internal frame when `internal_frame` is set.
    pub pos: [f32; 3],
    /// Row-major rotation matrix.
    pub rot: [[f32; 3]; 3],
    pub scale: f32,
    pub internal_frame: bool,
}

impl ModelTransform {
    /// A gameobject: world-space position, rotation `position.o` about Z.
    pub fn world(position: Position, scale: f32) -> Self {
        let (c, s) = (position.o.cos(), position.o.sin());
        Self {
            pos: [position.x, position.y, position.z],
            rot: [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]],
            scale,
            internal_frame: false,
        }
    }

    /// A static map model spawn: internal-frame position, Euler angles in
    /// degrees (rotation[0] pitch about Y, rotation[1] yaw about Z,
    /// rotation[2] roll about X), applied as Rz * Ry * Rx.
    pub fn spawn(instance: &ModelInstance) -> Self {
        let rad = |deg: f32| deg.to_radians();
        let (cz, sz) = (rad(instance.rotation[1]).cos(), rad(instance.rotation[1]).sin());
        let (cy, sy) = (rad(instance.rotation[0]).cos(), rad(instance.rotation[0]).sin());
        let (cx, sx) = (rad(instance.rotation[2]).cos(), rad(instance.rotation[2]).sin());
        let rz = [[cz, -sz, 0.0], [sz, cz, 0.0], [0.0, 0.0, 1.0]];
        let ry = [[cy, 0.0, sy], [0.0, 1.0, 0.0], [-sy, 0.0, cy]];
        let rx = [[1.0, 0.0, 0.0], [0.0, cx, -sx], [0.0, sx, cx]];
        Self {
            pos: [instance.position.x, instance.position.y, instance.position.z],
            rot: mat_mul(rz, mat_mul(ry, rx)),
            scale: if instance.scale > 0.0 { instance.scale } else { 1.0 },
            internal_frame: true,
        }
    }

    pub fn to_world(&self, v: [f32; 3]) -> Position {
        let r = mat_vec(self.rot, [v[0] * self.scale, v[1] * self.scale, v[2] * self.scale]);
        let f = [r[0] + self.pos[0], r[1] + self.pos[1], r[2] + self.pos[2]];
        if self.internal_frame {
            Position::new(GRID_MID - f[0], GRID_MID - f[1], f[2], 0.0)
        } else {
            Position::new(f[0], f[1], f[2], 0.0)
        }
    }

    pub fn to_model(&self, p: &Position) -> [f32; 3] {
        let f = if self.internal_frame {
            [GRID_MID - p.x, GRID_MID - p.y, p.z]
        } else {
            [p.x, p.y, p.z]
        };
        let d = [f[0] - self.pos[0], f[1] - self.pos[1], f[2] - self.pos[2]];
        // Inverse of a rotation is its transpose.
        let t = transpose(self.rot);
        let m = mat_vec(t, d);
        [m[0] / self.scale, m[1] / self.scale, m[2] / self.scale]
    }
}

fn mat_mul(a: [[f32; 3]; 3], b: [[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let mut out = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    out
}

fn mat_vec(m: [[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

fn transpose(m: [[f32; 3]; 3]) -> [[f32; 3]; 3] {
    [
        [m[0][0], m[1][0], m[2][0]],
        [m[0][1], m[1][1], m[2][1]],
        [m[0][2], m[1][2], m[2][2]],
    ]
}

/// A WMO group's liquid grid together with its placement.
#[derive(Debug, Clone)]
pub struct PlacedLiquidGrid {
    pub tiles_x: u32,
    pub tiles_y: u32,
    pub corner: [f32; 3],
    pub heights: Vec<f32>,
    pub flags: Vec<u8>,
    pub transform: ModelTransform,
}

impl PlacedLiquidGrid {
    /// World-space liquid surface height above (x, y), if that cell has liquid.
    /// Each cell is split into two triangles along its diagonal and the height
    /// interpolated on the one containing the point.
    pub fn surface_height(&self, pos: &Position) -> Option<f32> {
        let p = self.transform.to_model(pos);
        let tx_f = (p[0] - self.corner[0]) / LIQUID_TILE_SIZE;
        let ty_f = (p[1] - self.corner[1]) / LIQUID_TILE_SIZE;
        if tx_f < 0.0 || ty_f < 0.0 {
            return None;
        }
        let (tx, ty) = (tx_f as u32, ty_f as u32);
        if tx >= self.tiles_x || ty >= self.tiles_y {
            return None;
        }
        if self.flags.get((tx + ty * self.tiles_x) as usize)? & 0x0F == 0x0F {
            return None;
        }
        let dx = tx_f - tx as f32;
        let dy = ty_f - ty as f32;
        let row = self.tiles_x + 1;
        let h = |x: u32, y: u32| self.heights.get((x + y * row) as usize).copied();
        let base = h(tx, ty)?;
        let model_z = if dx > dy {
            let sx = h(tx + 1, ty)? - base;
            let sy = h(tx + 1, ty + 1)? - h(tx + 1, ty)?;
            base + dx * sx + dy * sy
        } else {
            let sx = h(tx + 1, ty + 1)? - h(tx, ty + 1)?;
            let sy = h(tx, ty + 1)? - base;
            base + dx * sx + dy * sy
        };
        Some(self.transform.to_world([p[0], p[1], model_z]).z)
    }
}

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
    place_with(world_model, ModelTransform::world(position, scale), model_id, model_type)
}

/// Place a static map model spawn (from a `.vmtile` or a non-tiled `.vmtree`).
pub fn place_static_model(
    world_model: &WorldModelData,
    instance: &ModelInstance,
) -> Vec<Arc<BSPModelInstance>> {
    place_with(
        world_model,
        ModelTransform::spawn(instance),
        instance.model_id,
        instance.model_type,
    )
}

fn place_with(
    world_model: &WorldModelData,
    transform: ModelTransform,
    model_id: u32,
    model_type: ModelType,
) -> Vec<Arc<BSPModelInstance>> {
    let mut placed = Vec::with_capacity(world_model.groups.len());

    for group in &world_model.groups {
        let mut triangles = group.triangles.clone();
        for triangle in &mut triangles {
            for v in [&mut triangle.v0, &mut triangle.v1, &mut triangle.v2] {
                *v = transform.to_world([v.x, v.y, v.z]);
            }
        }

        // Fit the box around the group's declared bounds and its placed
        // geometry; rotation does not keep a box axis-aligned, so all eight
        // corners are placed and re-fitted.
        let gb = group.bounding_box;
        let mut points: Vec<Position> = triangles
            .iter()
            .flat_map(|t| [t.v0, t.v1, t.v2])
            .collect();
        for x in [gb.min.x, gb.max.x] {
            for y in [gb.min.y, gb.max.y] {
                for z in [gb.min.z, gb.max.z] {
                    points.push(transform.to_world([x, y, z]));
                }
            }
        }
        let mut bbox = BoundingBox {
            min: Position::new(f32::INFINITY, f32::INFINITY, f32::INFINITY, 0.0),
            max: Position::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY, 0.0),
        };
        for p in &points {
            bbox.min.x = bbox.min.x.min(p.x);
            bbox.min.y = bbox.min.y.min(p.y);
            bbox.min.z = bbox.min.z.min(p.z);
            bbox.max.x = bbox.max.x.max(p.x);
            bbox.max.y = bbox.max.y.max(p.y);
            bbox.max.z = bbox.max.z.max(p.z);
        }

        let liquid_data = group.liquid_data.as_ref().map(|ld| {
            let surface = transform.to_world([ld.corner[0], ld.corner[1], ld.level]);
            let floor = transform.to_world([ld.corner[0], ld.corner[1], ld.floor]);
            let grid = (ld.tiles_x > 0 && ld.tiles_y > 0).then(|| {
                Arc::new(PlacedLiquidGrid {
                    tiles_x: ld.tiles_x,
                    tiles_y: ld.tiles_y,
                    corner: ld.corner,
                    heights: ld.heights.clone(),
                    flags: ld.flags.clone(),
                    transform,
                })
            });
            LiquidLevel {
                level: surface.z,
                floor: floor.z,
                liquid_type: ld.liquid_type,
                grid,
            }
        });

        placed.push(Arc::new(BSPModelInstance {
            model_id,
            model_type,
            bounding_box: bbox,
            triangles,
            liquid_data,
            footprint: Default::default(),
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
            tiles_x: 1,
            tiles_y: 1,
            corner: [0.0, 0.0, 0.0],
            heights: vec![1.0; 4],
            flags: vec![0],
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

    fn sloped_liquid(flags: Vec<u8>) -> PlacedLiquidGrid {
        // 2x1 cells; surface rises by 1 per cell in x.
        PlacedLiquidGrid {
            tiles_x: 2,
            tiles_y: 1,
            corner: [0.0, 0.0, 0.0],
            heights: vec![0.0, 1.0, 2.0, 0.0, 1.0, 2.0],
            flags,
            transform: ModelTransform::world(Position::new(0.0, 0.0, 10.0, 0.0), 1.0),
        }
    }

    #[test]
    fn liquid_surface_interpolates_within_its_cell() {
        let grid = sloped_liquid(vec![0, 0]);
        let half = LIQUID_TILE_SIZE / 2.0;
        let h = grid
            .surface_height(&Position::new(half, half * 0.5, 0.0, 0.0))
            .unwrap();
        assert!((h - 10.5).abs() < 1e-4, "height was {h}");
        let h = grid
            .surface_height(&Position::new(LIQUID_TILE_SIZE + half, half * 0.5, 0.0, 0.0))
            .unwrap();
        assert!((h - 11.5).abs() < 1e-4, "height was {h}");
    }

    #[test]
    fn disabled_cells_and_points_outside_the_grid_have_no_liquid() {
        let grid = sloped_liquid(vec![0, 0x0F]);
        let half = LIQUID_TILE_SIZE / 2.0;
        assert!(grid
            .surface_height(&Position::new(LIQUID_TILE_SIZE + half, half, 0.0, 0.0))
            .is_none());
        assert!(grid
            .surface_height(&Position::new(-1.0, half, 0.0, 0.0))
            .is_none());
        assert!(grid
            .surface_height(&Position::new(3.0 * LIQUID_TILE_SIZE, half, 0.0, 0.0))
            .is_none());
    }

    #[test]
    fn static_spawns_leave_the_internal_frame() {
        // The first spawn of 000_32_48.vmtile sits at internal XY
        // (25618.014, 17610.342), which is world XY (-8551.347, -543.675).
        let instance = ModelInstance {
            model_id: 1,
            model_type: ModelType::WMO,
            position: Position::new(25618.014, 17610.342, 0.0, 0.0),
            scale: 1.0,
            rotation: [0.0, 0.0, 0.0],
            model_name: String::new(),
        };
        let world = ModelTransform::spawn(&instance).to_world([0.0, 0.0, 0.0]);
        assert!((world.x - -8551.347).abs() < 0.01, "x was {}", world.x);
        assert!((world.y - -543.675).abs() < 0.01, "y was {}", world.y);
    }

    #[test]
    fn spawn_yaw_rotates_before_the_frame_mirror_and_inverts_cleanly() {
        let instance = ModelInstance {
            model_id: 1,
            model_type: ModelType::WMO,
            position: Position::new(100.0, 200.0, 30.0, 0.0),
            scale: 2.0,
            rotation: [10.0, 90.0, 5.0],
            model_name: String::new(),
        };
        let t = ModelTransform::spawn(&instance);
        let v = [3.0, -4.0, 1.5];
        let back = t.to_model(&t.to_world(v));
        for k in 0..3 {
            assert!((back[k] - v[k]).abs() < 1e-3, "component {k}: {:?}", back);
        }
        // A pure 90° yaw maps model +x onto internal +y, i.e. world -y.
        let yaw = ModelTransform::spawn(&ModelInstance {
            rotation: [0.0, 90.0, 0.0],
            scale: 1.0,
            ..instance
        });
        let origin = yaw.to_world([0.0, 0.0, 0.0]);
        let moved = yaw.to_world([1.0, 0.0, 0.0]);
        assert!((moved.x - origin.x).abs() < 1e-3);
        assert!((moved.y - origin.y + 1.0).abs() < 1e-3);
    }
}
