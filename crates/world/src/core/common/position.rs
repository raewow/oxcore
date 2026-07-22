//! Position - re-exported from shared::protocol::position, plus map-bound helpers.

pub use oxcore_shared::protocol::position::Position;

/// Width of one grid, in yards.
const SIZE_OF_GRIDS: f32 = 533.333_3;

/// Number of grids along one axis of a map.
const MAX_NUMBER_OF_GRIDS: f32 = 64.0;

/// Half the width of a map, in yards (`MAP_HALFSIZE`).
pub const MAP_HALFSIZE: f32 = SIZE_OF_GRIDS * MAX_NUMBER_OF_GRIDS / 2.0;

/// Largest X/Y coordinate a valid map position may hold.
pub const MAX_MAP_COORD: f32 = MAP_HALFSIZE - 0.5;

/// Height coordinates get a far wider bound than X/Y (`MaNGOS::IsValidZCoord`).
pub const MAX_Z_COORD: f32 = 400_000.0;

/// Clamp a coordinate into the map bounds (`MaNGOS::NormalizeMapCoord`).
pub fn normalize_map_coord(coord: f32) -> f32 {
    coord.clamp(-MAX_MAP_COORD, MAX_MAP_COORD)
}

/// Whether a position is usable (`MaNGOS::IsValidMapCoord`).
///
/// X/Y must be finite and inside the map, Z within the much wider height bound, and
/// the orientation finite and within 4π.
pub fn is_valid_map_coord(x: f32, y: f32, z: f32, orientation: f32) -> bool {
    x.is_finite()
        && y.is_finite()
        && x.abs() <= MAX_MAP_COORD
        && y.abs() <= MAX_MAP_COORD
        && z.is_finite()
        && z.abs() <= MAX_Z_COORD
        && orientation.is_finite()
        && orientation.abs() <= 4.0 * std::f32::consts::PI
}

/// Straight-line distance between two points in 3D (`Geometry::GetDistance3D`).
///
/// Clamped to zero so a non-finite computation can never yield a negative distance, matching
/// the C++ `dist > 0 ? dist : 0` guard.
pub fn get_distance_3d(from: (f32, f32, f32), to: (f32, f32, f32)) -> f32 {
    let dx = from.0 - to.0;
    let dy = from.1 - to.1;
    let dz = from.2 - to.2;
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
    if dist > 0.0 {
        dist
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_3d_is_the_euclidean_norm_of_the_delta() {
        // 3-4-12 -> 13, the classic integer 3D triple.
        assert!((get_distance_3d((0.0, 0.0, 0.0), (3.0, 4.0, 12.0)) - 13.0).abs() < 1e-4);
        // Coincident points are zero, never negative.
        assert_eq!(get_distance_3d((5.0, -2.0, 9.0), (5.0, -2.0, 9.0)), 0.0);
        // Order independent.
        let a = get_distance_3d((1.0, 2.0, 3.0), (-4.0, 0.5, 7.0));
        let b = get_distance_3d((-4.0, 0.5, 7.0), (1.0, 2.0, 3.0));
        assert!((a - b).abs() < 1e-6);
    }

    #[test]
    fn map_bound_is_half_the_map_width() {
        // 64 grids of 533.33 yards, halved, minus the half-yard margin.
        assert!((MAP_HALFSIZE - 17_066.656).abs() < 0.1);
        assert!((MAX_MAP_COORD - (MAP_HALFSIZE - 0.5)).abs() < f32::EPSILON);
    }

    #[test]
    fn normalize_map_coord_clamps_out_of_bounds_values() {
        assert_eq!(normalize_map_coord(0.0), 0.0);
        assert_eq!(normalize_map_coord(-1234.5), -1234.5);
        assert_eq!(normalize_map_coord(50_000.0), MAX_MAP_COORD);
        assert_eq!(normalize_map_coord(-50_000.0), -MAX_MAP_COORD);
    }

    #[test]
    fn valid_coords_accept_in_bounds_positions() {
        assert!(is_valid_map_coord(0.0, 0.0, 0.0, 0.0));
        assert!(is_valid_map_coord(-8_900.0, 2_200.0, 120.5, 3.14));
        // Z gets the height bound, not the map bound.
        assert!(is_valid_map_coord(0.0, 0.0, 20_000.0, 0.0));
    }

    #[test]
    fn valid_coords_reject_out_of_bounds_and_non_finite_values() {
        // Beyond the map half-size, even though within a full map width.
        assert!(!is_valid_map_coord(20_000.0, 0.0, 0.0, 0.0));
        assert!(!is_valid_map_coord(0.0, -20_000.0, 0.0, 0.0));
        assert!(!is_valid_map_coord(0.0, 0.0, 500_000.0, 0.0));
        assert!(!is_valid_map_coord(0.0, 0.0, 0.0, 100.0));
        assert!(!is_valid_map_coord(f32::NAN, 0.0, 0.0, 0.0));
        assert!(!is_valid_map_coord(0.0, f32::INFINITY, 0.0, 0.0));
        assert!(!is_valid_map_coord(0.0, 0.0, f32::NAN, 0.0));
        assert!(!is_valid_map_coord(0.0, 0.0, 0.0, f32::NAN));
    }
}
