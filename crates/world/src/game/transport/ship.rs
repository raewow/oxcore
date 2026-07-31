//! Ship and zeppelin path traversal (game object type 15).
//!
//! A ship replays the keyframe schedule built by [`super::schedule`]: as time advances it
//! walks a cursor along the keyframes, pausing at stop frames and moving between them. That
//! cursor logic - which keyframe is current at a given path progress, and whether the ship is
//! moving or waiting - is pure and ported here. Evaluating the actual position along the
//! segment's spline, the cross-map teleports and the passenger repositioning that
//! the full update also does need the spline geometry and Map subsystems and stay in
//! the (blocked) full update.

use crate::game::creature::movement::spline_base::{SplineBase, Vec3};

use super::schedule::KeyFrame;
use super::segment::{calculate_segment_pos, MotionProfile, SegmentFrame};

/// Where a given path progress falls relative to one keyframe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FramePhase {
    /// Paused at this stop frame: progress is within `[arrive_time, departure_time)`.
    WaitingHere,
    /// Travelling this segment: progress is within `[departure_time, next_arrive_time)`.
    MovingHere,
    /// This frame is already behind; advance to the next one.
    PastThisFrame,
}

/// Classify `path_progress` (ms within the path period) against a single keyframe's timing
/// windows, mirroring the two range checks at the top of the update loop.
pub fn classify_frame(frame: &KeyFrame, path_progress: u32) -> FramePhase {
    if path_progress >= frame.arrive_time && path_progress < frame.departure_time {
        FramePhase::WaitingHere
    } else if path_progress >= frame.departure_time && path_progress < frame.next_arrive_time {
        FramePhase::MovingHere
    } else {
        FramePhase::PastThisFrame
    }
}

/// Advance the keyframe cursor one step along the cyclic path.
///
/// The path loops, so stepping off the last keyframe wraps back to the first.
pub fn move_to_next_waypoint(cursor: usize, keyframe_count: usize) -> usize {
    if keyframe_count == 0 {
        return cursor;
    }
    (cursor + 1) % keyframe_count
}

/// The ship's keyframe cursor after advancing it for a path progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShipFrameState {
    /// Index of the keyframe the ship is now on.
    pub cursor: usize,
    /// Whether the ship is moving (`false` means paused at a stop frame).
    pub moving: bool,
}

/// Advance the keyframe cursor to the frame current at `path_progress`
/// (the update's advance loop, without the teleport and spline work).
///
/// Starting from `cursor`, steps forward over frames already behind - cyclically, since the
/// path loops - until it lands on the stop frame it is waiting at or the segment it is
/// travelling. Bounded by the keyframe count so a progress value that matches no window (a
/// malformed schedule) terminates instead of spinning.
pub fn advance_to_current_frame(
    keyframes: &[KeyFrame],
    cursor: usize,
    path_progress: u32,
) -> ShipFrameState {
    let n = keyframes.len();
    if n == 0 {
        return ShipFrameState {
            cursor,
            moving: false,
        };
    }

    let mut cursor = cursor % n;
    // One extra step of slack so a cursor starting just past its frame can still wrap around.
    for _ in 0..=n {
        match classify_frame(&keyframes[cursor], path_progress) {
            FramePhase::WaitingHere => {
                return ShipFrameState {
                    cursor,
                    moving: false,
                }
            }
            FramePhase::MovingHere => {
                return ShipFrameState {
                    cursor,
                    moving: true,
                }
            }
            FramePhase::PastThisFrame => cursor = move_to_next_waypoint(cursor, n),
        }
    }
    ShipFrameState {
        cursor,
        moving: true,
    }
}

/// The ship's world position and facing at a moment it is moving, plus the keyframe cursor
/// it resolved to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShipMotion {
    pub cursor: usize,
    pub position: Vec3,
    pub orientation: f32,
}

/// Where the ship is along its path at `path_progress` (ms within the period), evaluating the
/// current segment's spline.
///
/// Advances the keyframe cursor to the current frame, then - only while moving, as the
/// reference guards with `IsMoving() && pathProgress` - finds how far along the segment the
/// ship is via [`calculate_segment_pos`](super::segment::calculate_segment_pos) and reads the
/// position and tangent off that segment's spline, facing `atan2(dir.y, dir.x) + PI`.
/// Returns `None` when the ship is paused at a stop (its position is unchanged) or the spline
/// cannot be evaluated.
pub fn ship_position(
    keyframes: &[KeyFrame],
    splines: &[SplineBase],
    profile: &MotionProfile,
    cursor: usize,
    path_progress: u32,
) -> Option<ShipMotion> {
    let state = advance_to_current_frame(keyframes, cursor, path_progress);
    if !state.moving || path_progress == 0 {
        return None;
    }

    let frame = &keyframes[state.cursor];
    let segment = SegmentFrame {
        time_from: frame.time_from,
        time_to: frame.time_to,
        dist_since_stop: frame.dist_since_stop,
        dist_until_stop: frame.dist_until_stop,
        next_dist_from_prev: frame.next_dist_from_prev,
        departure_time_ms: frame.departure_time,
    };
    let t = calculate_segment_pos(profile, &segment, path_progress as f32 / 1000.0);

    let spline = splines.get(frame.spline_id)?;
    let position = spline.evaluate(frame.index as usize, t)?;
    let direction = spline.evaluate_derivative(frame.index as usize, t)?;
    let orientation = direction.y.atan2(direction.x) + std::f32::consts::PI;

    Some(ShipMotion {
        cursor: state.cursor,
        position,
        orientation,
    })
}

#[cfg(test)]
mod tests {
    use super::super::schedule::{compute_schedule, KeyFrame, ScheduleProfile};
    use super::super::waypoints::{generate_waypoints, TaxiPathNode};
    use super::*;

    fn profile() -> ScheduleProfile {
        ScheduleProfile {
            speed: 10.0,
            accel: 5.0,
        }
    }

    /// A two-stop path: stop at frame 0, run out to frame 2, stop there, run back.
    fn scheduled_path() -> Vec<KeyFrame> {
        let mut kf = vec![
            KeyFrame::new(0.0, true, 2), // stop, 2s dwell
            KeyFrame::new(10.0, false, 0),
            KeyFrame::new(10.0, true, 2), // stop, 2s dwell
            KeyFrame::new(10.0, false, 0),
        ];
        compute_schedule(&profile(), &mut kf);
        kf
    }

    #[test]
    fn a_frame_is_classified_against_its_timing_windows() {
        let kf = scheduled_path();
        // Frame 0 is a stop: at its arrive time it is being waited at.
        let f0 = &kf[0];
        assert_eq!(classify_frame(f0, f0.arrive_time), FramePhase::WaitingHere);
        // Once past its departure it is being travelled (moving into the next segment).
        assert_eq!(
            classify_frame(f0, f0.departure_time),
            FramePhase::MovingHere
        );
        // Far past its whole window it is behind us.
        assert_eq!(
            classify_frame(f0, f0.next_arrive_time),
            FramePhase::PastThisFrame
        );
    }

    #[test]
    fn the_ship_waits_at_a_stop_frame() {
        let kf = scheduled_path();
        // During frame 0's dwell (it departs at 2000ms) the ship is stopped there.
        let state = advance_to_current_frame(&kf, 0, kf[0].arrive_time);
        assert_eq!(
            state,
            ShipFrameState {
                cursor: 0,
                moving: false
            }
        );
    }

    #[test]
    fn the_ship_advances_to_the_segment_it_is_travelling() {
        let kf = scheduled_path();
        // Just after leaving the first stop, the ship is moving on frame 0's segment.
        let state = advance_to_current_frame(&kf, 0, kf[0].departure_time);
        assert!(state.moving);

        // A progress in the middle of the path lands on a later frame, still moving.
        let mid = kf[2].departure_time;
        let state = advance_to_current_frame(&kf, 0, mid);
        assert!(state.moving);
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn the_cursor_wraps_the_cyclic_path() {
        let kf = scheduled_path();
        // Starting the cursor on the last frame with progress back at the start wraps it
        // around to the first stop rather than spinning forever.
        let state = advance_to_current_frame(&kf, kf.len() - 1, kf[0].arrive_time);
        assert_eq!(
            state,
            ShipFrameState {
                cursor: 0,
                moving: false
            }
        );
    }

    #[test]
    fn an_empty_schedule_does_not_spin() {
        let state = advance_to_current_frame(&[], 0, 1234);
        assert!(!state.moving);
    }

    #[test]
    fn the_waypoint_cursor_steps_and_wraps() {
        // Four keyframes: stepping advances, and the last wraps back to the first.
        assert_eq!(move_to_next_waypoint(0, 4), 1);
        assert_eq!(move_to_next_waypoint(2, 4), 3);
        assert_eq!(move_to_next_waypoint(3, 4), 0);
        // An empty path has nowhere to step.
        assert_eq!(move_to_next_waypoint(0, 0), 0);
    }

    /// A generated straight-line path (7 nodes along +x, spaced 10 yards) and its motion
    /// profile, for driving the runtime position.
    fn straight_transport() -> (Vec<KeyFrame>, Vec<SplineBase>, MotionProfile) {
        let nodes: Vec<TaxiPathNode> = (0..7)
            .map(|i| TaxiPathNode {
                map_id: 0,
                x: i as f32 * 10.0,
                y: 0.0,
                z: 0.0,
                action_flag: 0,
                delay: 0,
            })
            .collect();
        let path = generate_waypoints(
            &nodes,
            &ScheduleProfile {
                speed: 10.0,
                accel: 5.0,
            },
        )
        .unwrap();
        let profile = MotionProfile {
            speed: path.speed,
            accel: path.accel,
            accel_time: path.accel_time,
            accel_dist: path.accel_dist,
        };
        (path.keyframes, path.segment_splines, profile)
    }

    #[test]
    fn a_paused_or_zero_progress_ship_reports_no_motion() {
        let (kf, splines, profile) = straight_transport();
        // At the very start of the cycle there is no motion to report.
        assert!(ship_position(&kf, &splines, &profile, 0, 0).is_none());
    }

    #[test]
    fn a_moving_ship_sits_on_its_path() {
        let (kf, splines, profile) = straight_transport();
        // Part way along the path the ship is moving and sits on the straight line: y and z
        // stay zero and x lies within the interior node span the keyframes cover.
        let mid = kf.last().unwrap().departure_time / 2;
        let motion = ship_position(&kf, &splines, &profile, 0, mid).expect("moving mid-path");
        assert!(
            motion.position.y.abs() < 1e-2,
            "off the line: {:?}",
            motion.position
        );
        assert!(motion.position.z.abs() < 1e-2);
        assert!(
            motion.position.x > 5.0 && motion.position.x < 55.0,
            "x out of span: {}",
            motion.position.x
        );
        // Travelling +x, the facing is atan2(0, +) + PI = PI.
        assert!(
            (motion.orientation - std::f32::consts::PI).abs() < 1e-2,
            "got {}",
            motion.orientation
        );
    }

    #[test]
    fn the_ship_advances_along_the_line_over_time() {
        let (kf, splines, profile) = straight_transport();
        let period = kf.last().unwrap().departure_time;
        // Earlier in the run the ship is further back along +x than later in the run.
        let early = ship_position(&kf, &splines, &profile, 0, period / 4).expect("moving");
        let late = ship_position(&kf, &splines, &profile, 0, period / 2).expect("moving");
        assert!(
            late.position.x > early.position.x,
            "did not advance: {} -> {}",
            early.position.x,
            late.position.x
        );
    }
}
