//! Elevator and tram motion (game object type 11).
//!
//! Unlike ships, elevators do not follow taxi-path splines. They replay a canned animation:
//! a table of `(time, position)` keyframes ([`super::animation`]) looped over a fixed period.
//! Their local position at any moment is a straight linear interpolation between the two
//! bracketing keyframes, computed here. Turning that local position into a world position
//! (the object's rotation, the vanilla Y-flip and the stationary offset) and relocating the
//! object and its passengers belong to the full `Update`, which needs the Object
//! and Map subsystems and is not yet ported.

use crate::game::creature::movement::spline_base::Vec3;

use super::animation::TransportAnimation;

/// Where in its looping animation the elevator is, given how long it has existed.
///
/// The path repeats every `total_time` milliseconds. A zero-length animation has no cycle to
/// index into, so there is no progress to report.
pub fn path_progress(total_time: u32, time_since_creation: u32) -> Option<u32> {
    if total_time == 0 {
        None
    } else {
        Some(time_since_creation % total_time)
    }
}

/// The elevator's position in its own local frame at `path_progress`.
///
/// Linearly interpolates between the keyframes bracketing `path_progress` by the fraction of
/// the way between their times. Two keyframes sharing a position (a pause in the animation)
/// yield that position directly. Returns `None` when `path_progress` is not bracketed by two
/// keyframes - before the first or after the last - matching the reference guard that both a
/// prev and a next node exist.
pub fn interpolate_local_position(anim: &TransportAnimation, path_progress: u32) -> Option<Vec3> {
    let prev = anim.prev_anim_node(path_progress)?;
    let next = anim.next_anim_node(path_progress)?;

    let pos_prev = Vec3::new(prev.x, prev.y, prev.z);
    let pos_next = Vec3::new(next.x, next.y, next.z);

    if pos_prev == pos_next {
        return Some(pos_prev);
    }

    let time_elapsed = (path_progress - prev.time_seg) as f32;
    let time_diff = (next.time_seg - prev.time_seg) as f32;
    let fraction = time_elapsed / time_diff;

    Some(Vec3::new(
        pos_prev.x + (pos_next.x - pos_prev.x) * fraction,
        pos_prev.y + (pos_next.y - pos_prev.y) * fraction,
        pos_prev.z + (pos_next.z - pos_prev.z) * fraction,
    ))
}

#[cfg(test)]
mod tests {
    use super::super::animation::TransportAnimationEntry;
    use super::*;

    fn frame(time_seg: u32, x: f32, z: f32) -> TransportAnimationEntry {
        TransportAnimationEntry {
            time_seg,
            x,
            y: 0.0,
            z,
        }
    }

    /// A lift that rises from z=0 to z=10 over the first 2s, then holds there to 4s.
    fn lift() -> TransportAnimation {
        let mut anim = TransportAnimation::new();
        anim.add_frame(frame(0, 5.0, 0.0));
        anim.add_frame(frame(2000, 5.0, 10.0));
        anim.add_frame(frame(4000, 5.0, 10.0));
        anim
    }

    #[test]
    fn progress_loops_over_the_animation_period() {
        assert_eq!(path_progress(4000, 1000), Some(1000));
        // One full period on has looped back to the start.
        assert_eq!(path_progress(4000, 5000), Some(1000));
        // A zero-length animation has no cycle.
        assert_eq!(path_progress(0, 1000), None);
    }

    #[test]
    fn position_interpolates_linearly_between_keyframes() {
        let lift = lift();
        // Halfway through the 2s rise, the lift is halfway up.
        let mid = interpolate_local_position(&lift, 1000).unwrap();
        assert!((mid.z - 5.0).abs() < 1e-4, "got {}", mid.z);
        assert!((mid.x - 5.0).abs() < 1e-4);
        // A quarter of the way up.
        let quarter = interpolate_local_position(&lift, 500).unwrap();
        assert!((quarter.z - 2.5).abs() < 1e-4, "got {}", quarter.z);
    }

    #[test]
    fn a_paused_segment_holds_position() {
        let lift = lift();
        // Between the two identical top keyframes the lift stays at the top.
        let held = interpolate_local_position(&lift, 3000).unwrap();
        assert!((held.z - 10.0).abs() < 1e-4, "got {}", held.z);
    }

    #[test]
    fn positions_outside_the_keyframes_are_unbracketed() {
        let lift = lift();
        // Before the first keyframe there is no previous node to interpolate from.
        assert!(interpolate_local_position(&lift, 0).is_none());
        // Past the last keyframe there is no next node.
        assert!(interpolate_local_position(&lift, 4001).is_none());
    }
}
