//! Where along its current segment a moving transport sits at a given moment.
//!
//! A ship or zeppelin accelerates from each stop, cruises, then decelerates into the next.
//! The kinematics that turn "how long since the last stop" into "how far along this
//! segment" are pure and ported here; reading them off the transport's keyframe and
//! game-object template is the (unported) transport object's job.

/// Motion profile of a transport between two stops.
///
/// `accel_dist` is the distance covered while reaching cruise speed over `accel_time`.
#[derive(Debug, Clone, Copy)]
pub struct MotionProfile {
    pub speed: f32,
    pub accel: f32,
    pub accel_time: f32,
    pub accel_dist: f32,
}

/// The keyframe timing/distance a segment position is measured against.
///
/// Times are seconds since the transport departed this frame's stop; distances are yards.
#[derive(Debug, Clone, Copy)]
pub struct SegmentFrame {
    /// Seconds from this frame's stop to the frame.
    pub time_from: f32,
    /// Seconds from the frame to the next stop.
    pub time_to: f32,
    /// Distance already travelled since the last stop at this frame.
    pub dist_since_stop: f32,
    /// Distance still to travel until the next stop at this frame.
    pub dist_until_stop: f32,
    /// Length of this segment, used to normalize the result to [0, 1].
    pub next_dist_from_prev: f32,
    /// Milliseconds into this frame at which the transport departed.
    pub departure_time_ms: u32,
}

/// Distance covered from a stop after `elapsed` seconds under `profile`.
///
/// Quadratic while accelerating, then linear at cruise speed.
fn distance_from_stop(profile: &MotionProfile, elapsed: f32) -> f32 {
    if elapsed < profile.accel_time {
        0.5 * profile.accel * elapsed * elapsed
    } else {
        profile.accel_dist + (elapsed - profile.accel_time) * profile.speed
    }
}

/// Fraction along the current segment, in [0, 1], at world time `now` seconds
/// (`ShipTransport::CalculateSegmentPos`).
///
/// The calculation is done from whichever stop is nearer - measuring forward from the
/// last one while accelerating, backward from the next one while decelerating - because
/// the motion is symmetric about the segment's midpoint. Returns 0 for a zero-length
/// segment rather than dividing by zero.
pub fn calculate_segment_pos(profile: &MotionProfile, frame: &SegmentFrame, now: f32) -> f32 {
    let departed = now - frame.departure_time_ms as f32 / 1000.0;
    let time_since_stop = frame.time_from + departed;
    let time_until_stop = frame.time_to - departed;

    let segment_pos = if time_since_stop < time_until_stop {
        distance_from_stop(profile, time_since_stop) - frame.dist_since_stop
    } else {
        frame.dist_until_stop - distance_from_stop(profile, time_until_stop)
    };

    if frame.next_dist_from_prev == 0.0 {
        0.0
    } else {
        segment_pos / frame.next_dist_from_prev
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cruise 10 yd/s, reaching that over 2s (so accel 5, accel_dist 10).
    fn profile() -> MotionProfile {
        MotionProfile {
            speed: 10.0,
            accel: 5.0,
            accel_time: 2.0,
            accel_dist: 10.0,
        }
    }

    /// A 40-yard segment: 2s accelerating (10 yd), 2s cruising (20 yd), 2s
    /// decelerating (10 yd), 6s total.
    fn frame() -> SegmentFrame {
        SegmentFrame {
            time_from: 0.0,
            time_to: 6.0,
            dist_since_stop: 0.0,
            dist_until_stop: 40.0,
            next_dist_from_prev: 40.0,
            departure_time_ms: 0,
        }
    }

    #[test]
    fn a_transport_sits_at_the_segment_start_at_time_zero() {
        assert!((calculate_segment_pos(&profile(), &frame(), 0.0)).abs() < 1e-4);
    }

    #[test]
    fn the_first_accelerating_second_covers_the_quadratic_distance() {
        // 0.5 * 5 * 1^2 = 2.5 yards of 40 => 0.0625.
        let pos = calculate_segment_pos(&profile(), &frame(), 1.0);
        assert!((pos - 2.5 / 40.0).abs() < 1e-4, "got {pos}");
    }

    #[test]
    fn the_cruise_phase_is_linear_and_symmetric() {
        // At t=3s (mid-segment) the transport is exactly halfway.
        let pos = calculate_segment_pos(&profile(), &frame(), 3.0);
        assert!((pos - 0.5).abs() < 1e-4, "got {pos}");

        // The deceleration side mirrors the acceleration side: t=5 mirrors t=1.
        let accel_side = calculate_segment_pos(&profile(), &frame(), 1.0);
        let decel_side = calculate_segment_pos(&profile(), &frame(), 5.0);
        assert!((accel_side + decel_side - 1.0).abs() < 1e-4);
    }

    #[test]
    fn the_transport_reaches_the_segment_end() {
        let pos = calculate_segment_pos(&profile(), &frame(), 6.0);
        assert!((pos - 1.0).abs() < 1e-4, "got {pos}");
    }

    #[test]
    fn a_zero_length_segment_does_not_divide_by_zero() {
        let mut frame = frame();
        frame.next_dist_from_prev = 0.0;
        assert_eq!(calculate_segment_pos(&profile(), &frame, 3.0), 0.0);
    }

    #[test]
    fn departure_offset_shifts_the_clock() {
        // Departing 1s into the frame makes t=1 read as the segment start.
        let mut frame = frame();
        frame.departure_time_ms = 1_000;
        assert!((calculate_segment_pos(&profile(), &frame, 1.0)).abs() < 1e-4);
    }
}
