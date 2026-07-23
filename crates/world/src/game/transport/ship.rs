//! Ship and zeppelin path traversal (`ShipTransport`, game object type 15).
//!
//! A ship replays the keyframe schedule built by [`super::schedule`]: as time advances it
//! walks a cursor along the keyframes, pausing at stop frames and moving between them. That
//! cursor logic - which keyframe is current at a given path progress, and whether the ship is
//! moving or waiting - is pure and ported here. Evaluating the actual position along the
//! segment's spline, the cross-map teleports and the passenger repositioning that
//! `ShipTransport::Update` also does need the spline geometry and Map subsystems and stay in
//! the (blocked) full update.

use super::schedule::KeyFrame;

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
/// windows, mirroring the two range checks at the top of `ShipTransport::Update`'s loop.
pub fn classify_frame(frame: &KeyFrame, path_progress: u32) -> FramePhase {
    if path_progress >= frame.arrive_time && path_progress < frame.departure_time {
        FramePhase::WaitingHere
    } else if path_progress >= frame.departure_time && path_progress < frame.next_arrive_time {
        FramePhase::MovingHere
    } else {
        FramePhase::PastThisFrame
    }
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
/// (the `while(true)` loop in `ShipTransport::Update`, without the teleport and spline work).
///
/// Starting from `cursor`, steps forward over frames already behind - cyclically, since the
/// path loops - until it lands on the stop frame it is waiting at or the segment it is
/// travelling. Bounded by the keyframe count so a progress value that matches no window (a
/// malformed schedule) terminates instead of spinning.
pub fn advance_to_current_frame(keyframes: &[KeyFrame], cursor: usize, path_progress: u32) -> ShipFrameState {
    let n = keyframes.len();
    if n == 0 {
        return ShipFrameState { cursor, moving: false };
    }

    let mut cursor = cursor % n;
    // One extra step of slack so a cursor starting just past its frame can still wrap around.
    for _ in 0..=n {
        match classify_frame(&keyframes[cursor], path_progress) {
            FramePhase::WaitingHere => return ShipFrameState { cursor, moving: false },
            FramePhase::MovingHere => return ShipFrameState { cursor, moving: true },
            FramePhase::PastThisFrame => cursor = (cursor + 1) % n,
        }
    }
    ShipFrameState { cursor, moving: true }
}

#[cfg(test)]
mod tests {
    use super::super::schedule::{compute_schedule, KeyFrame, ScheduleProfile};
    use super::*;

    fn profile() -> ScheduleProfile {
        ScheduleProfile { speed: 10.0, accel: 5.0 }
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
        assert_eq!(classify_frame(f0, f0.departure_time), FramePhase::MovingHere);
        // Far past its whole window it is behind us.
        assert_eq!(classify_frame(f0, f0.next_arrive_time), FramePhase::PastThisFrame);
    }

    #[test]
    fn the_ship_waits_at_a_stop_frame() {
        let kf = scheduled_path();
        // During frame 0's dwell (it departs at 2000ms) the ship is stopped there.
        let state = advance_to_current_frame(&kf, 0, kf[0].arrive_time);
        assert_eq!(state, ShipFrameState { cursor: 0, moving: false });
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
        assert_eq!(state, ShipFrameState { cursor: 0, moving: false });
    }

    #[test]
    fn an_empty_schedule_does_not_spin() {
        let state = advance_to_current_frame(&[], 0, 1234);
        assert!(!state.moving);
    }
}
