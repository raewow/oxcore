//! Passenger geometry for transports.
//!
//! A passenger on a moving transport is tracked by its offset from the transport's local
//! frame; converting between that offset and a world position is pure rotation math. This
//! is the foundation the rest of the transport system (not yet ported) will build on.

use oxcore_shared::protocol::Position;

/// Wrap an orientation into [0, 2π).
pub fn normalize_orientation(orientation: f32) -> f32 {
    let two_pi = std::f32::consts::TAU;
    // Negative orientations wrap by subtracting from 2π (fmod semantics for negatives).
    if orientation < 0.0 {
        two_pi - (-orientation % two_pi)
    } else {
        orientation % two_pi
    }
}

/// Where a transport sits in the world, with its facing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransportFrame {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: f32,
}

impl TransportFrame {
    pub fn new(x: f32, y: f32, z: f32, orientation: f32) -> Self {
        Self {
            x,
            y,
            z,
            orientation,
        }
    }

    /// A passenger's absolute orientation from its transport-local one.
    pub fn passenger_orientation(&self, local_orientation: f32) -> f32 {
        normalize_orientation(self.orientation + local_orientation)
    }

    /// World position of a passenger from its transport-local offset.
    ///
    /// Rotates the offset by the transport's facing and adds the transport's position;
    /// the orientation is composed with the transport's.
    pub fn passenger_position(&self, offset: Position) -> Position {
        let (sin_o, cos_o) = self.orientation.sin_cos();

        Position {
            x: self.x + offset.x * cos_o - offset.y * sin_o,
            y: self.y + offset.y * cos_o + offset.x * sin_o,
            z: self.z + offset.z,
            o: self.passenger_orientation(offset.o),
        }
    }

    /// Transport-local offset of a passenger from its world position.
    /// The inverse of [`Self::passenger_position`].
    pub fn passenger_offset(&self, world: Position) -> Position {
        let dx = world.x - self.x;
        let dy = world.y - self.y;
        let (sin_o, cos_o) = self.orientation.sin_cos();

        Position {
            x: dx * cos_o + dy * sin_o,
            y: dy * cos_o - dx * sin_o,
            z: world.z - self.z,
            o: normalize_orientation(world.o - self.orientation),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(x: f32, y: f32, z: f32, o: f32) -> Position {
        Position { x, y, z, o }
    }

    fn assert_close(a: Position, b: Position) {
        let tol = 1e-4;
        assert!(
            (a.x - b.x).abs() < tol
                && (a.y - b.y).abs() < tol
                && (a.z - b.z).abs() < tol
                && (a.o - b.o).abs() < tol,
            "expected {b:?}, got {a:?}"
        );
    }

    #[test]
    fn orientation_wraps_positive_and_negative_into_one_turn() {
        let two_pi = std::f32::consts::TAU;

        assert!((normalize_orientation(0.0)).abs() < 1e-6);
        assert!((normalize_orientation(two_pi) - 0.0).abs() < 1e-4);
        assert!((normalize_orientation(two_pi + 1.0) - 1.0).abs() < 1e-4);
        // A negative angle comes back in [0, 2*PI).
        assert!((normalize_orientation(-1.0) - (two_pi - 1.0)).abs() < 1e-4);
    }

    #[test]
    fn a_stationary_transport_leaves_the_offset_unchanged() {
        let frame = TransportFrame::new(100.0, 200.0, 50.0, 0.0);
        let offset = pos(5.0, -3.0, 2.0, 0.5);

        // Zero facing at the origin-ish: world = transport + offset.
        let world = frame.passenger_position(offset);
        assert_close(world, pos(105.0, 197.0, 52.0, 0.5));
    }

    #[test]
    fn a_quarter_turn_rotates_the_offset() {
        let frame = TransportFrame::new(0.0, 0.0, 0.0, std::f32::consts::FRAC_PI_2);
        // A point one unit ahead of the transport (local +x) is one unit to its left
        // (world +y) once the transport faces 90 degrees.
        let world = frame.passenger_position(pos(1.0, 0.0, 0.0, 0.0));
        assert_close(world, pos(0.0, 1.0, 0.0, std::f32::consts::FRAC_PI_2));
    }

    #[test]
    fn offset_is_the_inverse_of_position() {
        let frame = TransportFrame::new(-40.0, 12.0, 8.0, 1.234);
        let offset = pos(3.5, -7.25, 1.0, 2.0);

        let world = frame.passenger_position(offset);
        let recovered = frame.passenger_offset(world);

        assert_close(recovered, offset);
    }

    #[test]
    fn passenger_orientation_composes_and_wraps() {
        let frame = TransportFrame::new(0.0, 0.0, 0.0, std::f32::consts::PI);

        // PI + PI wraps back to 0.
        assert!((frame.passenger_orientation(std::f32::consts::PI)).abs() < 1e-4);
        assert!((frame.passenger_orientation(0.5) - (std::f32::consts::PI + 0.5)).abs() < 1e-4);
    }
}
