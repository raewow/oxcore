//! The keyframe schedule of a transport path: when it arrives at and departs each node.
//!
//! `TransportMgr::GenerateWaypoints` builds a transport's path in two halves. The first
//! half turns DBC taxi-path nodes into keyframes and measures the distance between them
//! with Catmull-Rom splines - that needs the DBC store and the spline geometry and is not
//! ported here. The second half is pure kinematics over those distances: it works out how
//! far each keyframe sits from the nearest stop, how long the transport takes to travel
//! each leg while accelerating, cruising and braking, and from that the absolute arrive and
//! departure times along the path. That schedule is what this module computes, and its
//! per-keyframe distances and times are exactly the inputs [`super::segment`] reads at
//! runtime to place the transport.

/// A transport's move speed and acceleration, from its game-object template.
#[derive(Debug, Clone, Copy)]
pub struct ScheduleProfile {
    /// Cruise speed in yards per second.
    pub speed: f32,
    /// Acceleration in yards per second squared.
    pub accel: f32,
}

impl ScheduleProfile {
    /// Distance covered while accelerating from a stop to cruise speed.
    pub fn accel_dist(&self) -> f32 {
        0.5 * self.speed * self.speed / self.accel
    }

    /// Time taken to accelerate from a stop to cruise speed.
    pub fn accel_time(&self) -> f32 {
        self.speed / self.accel
    }
}

/// One node of a transport path (`KeyFrame`).
///
/// The `dist_from_prev` and stop/delay fields are the inputs the schedule is computed from;
/// everything else is filled in by [`compute_schedule`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyFrame {
    /// Spline distance from the previous keyframe (input; the first keyframe's is 0).
    pub dist_from_prev: f32,
    /// Whether the transport halts here (`Node->actionFlag == 2`).
    pub is_stop_frame: bool,
    /// How long the transport waits at this node, in seconds (`Node->delay`).
    pub delay_secs: u32,

    /// Distance from this keyframe back to the next stop already passed.
    pub dist_since_stop: f32,
    /// Distance from this keyframe forward to the next stop.
    pub dist_until_stop: f32,
    /// Distance to the following keyframe (this keyframe's `NextDistFromPrev`).
    pub next_dist_from_prev: f32,
    /// Seconds from this keyframe forward to the next stop.
    pub time_to: f32,
    /// Seconds from the previous stop forward to this keyframe.
    pub time_from: f32,
    /// Absolute time the transport reaches this keyframe, in milliseconds.
    pub arrive_time: u32,
    /// Absolute time the transport leaves this keyframe, in milliseconds.
    pub departure_time: u32,
    /// The following keyframe's `arrive_time` (this keyframe's `NextArriveTime`).
    pub next_arrive_time: u32,

    /// Whether the path teleports away from this keyframe rather than moving on (`Teleport`).
    pub teleport: bool,
    /// Whether the client must be sent a fresh create block here (`Update`).
    pub update: bool,
    /// Facing the transport holds at this node, from the orientation spline
    /// (`InitialOrientation`).
    pub initial_orientation: f32,
    /// 1-based index of this keyframe within its spline segment (`Index`).
    pub index: u32,
    /// Which segment spline this keyframe's leg is evaluated on (the C++ `Spline` pointer,
    /// as an index into the path's segment splines).
    pub spline_id: usize,
    /// Map this keyframe's node is on.
    pub map_id: u32,
    /// World position of this keyframe's node, used as a teleport target.
    pub node_x: f32,
    pub node_y: f32,
    pub node_z: f32,
}

impl KeyFrame {
    /// A keyframe with its inputs set and its computed fields left at the C++ defaults.
    pub fn new(dist_from_prev: f32, is_stop_frame: bool, delay_secs: u32) -> Self {
        Self {
            dist_from_prev,
            is_stop_frame,
            delay_secs,
            dist_since_stop: -1.0,
            dist_until_stop: -1.0,
            next_dist_from_prev: 0.0,
            time_to: 0.0,
            time_from: 0.0,
            arrive_time: 0,
            departure_time: 0,
            next_arrive_time: 0,
            teleport: false,
            update: false,
            initial_orientation: 0.0,
            index: 0,
            spline_id: 0,
            map_id: 0,
            node_x: 0.0,
            node_y: 0.0,
            node_z: 0.0,
        }
    }
}

/// Milliseconds per second (`IN_MILLISECONDS`).
const IN_MILLISECONDS: f32 = 1000.0;

/// Fill in the timing schedule of `keyframes` from their distances and the transport's
/// `profile`, and return the total path time in milliseconds (`transportTemplate.pathTime`).
///
/// This is the second half of `TransportMgr::GenerateWaypoints`, from the point where the
/// spline distances are known. Keyframes form a closed loop - the last returns to the first
/// by teleport - so every accumulation wraps modulo the keyframe count.
pub fn compute_schedule(profile: &ScheduleProfile, keyframes: &mut [KeyFrame]) -> u32 {
    let n = keyframes.len();
    if n == 0 {
        return 0;
    }

    // Distance to the following keyframe, wrapping the last one back to the first.
    for i in 0..n {
        keyframes[i].next_dist_from_prev = keyframes[(i + 1) % n].dist_from_prev;
    }

    let (first_stop, last_stop) = find_stops(keyframes);

    // At each stop distSinceStop is 0; between stops it accumulates the distance travelled.
    // Measured from lastStop so a run of two stops in a row still resets cleanly.
    let mut tmp_dist = 0.0;
    for i in 0..n {
        let j = (i + last_stop) % n;
        if keyframes[j].is_stop_frame || j == last_stop {
            tmp_dist = 0.0;
        } else {
            tmp_dist += keyframes[j].dist_from_prev;
        }
        keyframes[j].dist_since_stop = tmp_dist;
    }

    // distUntilStop is the mirror image, accumulated backwards from firstStop.
    tmp_dist = 0.0;
    for i in (0..n).rev() {
        let j = (i + first_stop) % n;
        tmp_dist += keyframes[(j + 1) % n].dist_from_prev;
        keyframes[j].dist_until_stop = tmp_dist;
        if keyframes[j].is_stop_frame || j == first_stop {
            tmp_dist = 0.0;
        }
    }

    let accel = profile.accel;
    let speed = profile.speed;
    let accel_dist = profile.accel_dist();
    for k in keyframes.iter_mut() {
        k.time_to = time_to(
            k.dist_since_stop,
            k.dist_until_stop,
            speed,
            accel,
            accel_dist,
        );
    }

    // timeFrom is measured forward from the previous stop, so it needs the leg's total time.
    let mut segment_time = 0.0;
    for i in 0..n {
        let j = (i + last_stop) % n;
        if keyframes[j].is_stop_frame || j == last_stop {
            segment_time = keyframes[j].time_to;
        }
        keyframes[j].time_from = segment_time - keyframes[j].time_to;
    }

    compute_path_times(keyframes);

    keyframes[n - 1].departure_time
}

/// Index of the first and last stop keyframes, defaulting both to 0 when there are none.
///
/// Matches `GenerateWaypoints`: the first keyframe counts as a stop up front, and a path
/// with no stop frames at all collapses both to keyframe 0.
fn find_stops(keyframes: &[KeyFrame]) -> (usize, usize) {
    let mut first_stop: i32 = -1;
    let mut last_stop: i32 = -1;

    if keyframes[0].is_stop_frame {
        first_stop = 0;
        last_stop = 0;
    }
    for (i, k) in keyframes.iter().enumerate().skip(1) {
        if k.is_stop_frame {
            if first_stop == -1 {
                first_stop = i as i32;
            }
            last_stop = i as i32;
        }
    }
    if first_stop == -1 || last_stop == -1 {
        (0, 0)
    } else {
        (first_stop as usize, last_stop as usize)
    }
}

/// Seconds to travel from a keyframe to the next stop, given how far it lies between the
/// bracketing stops. Splits into whether the segment is too short to reach cruise speed and
/// which side of it the keyframe is on.
fn time_to(
    dist_since_stop: f32,
    dist_until_stop: f32,
    speed: f32,
    accel: f32,
    accel_dist: f32,
) -> f32 {
    let total_dist = dist_since_stop + dist_until_stop;
    if total_dist < 2.0 * accel_dist {
        // Too short to reach full speed: accelerate then brake, no cruise.
        if dist_since_stop < dist_until_stop {
            let segment_time = 2.0 * ((dist_until_stop + dist_since_stop) / accel).sqrt();
            segment_time - (2.0 * dist_since_stop / accel).sqrt()
        } else {
            (2.0 * dist_until_stop / accel).sqrt()
        }
    } else if dist_since_stop < accel_dist {
        // Still accelerating, but the segment is long enough to reach cruise speed.
        let segment_time = (dist_until_stop + dist_since_stop) / speed + speed / accel;
        segment_time - (2.0 * dist_since_stop / accel).sqrt()
    } else if dist_until_stop < accel_dist {
        // Already braking after having reached cruise speed.
        (2.0 * dist_until_stop / accel).sqrt()
    } else {
        // Cruising at full speed.
        dist_until_stop / speed + 0.5 * speed / accel
    }
}

/// Fill in each keyframe's absolute arrive and departure times from the per-leg `time_to`.
fn compute_path_times(keyframes: &mut [KeyFrame]) {
    let n = keyframes.len();

    keyframes[0].arrive_time = 0;
    let mut cur_path_time = 0.0;
    if keyframes[0].is_stop_frame {
        cur_path_time = keyframes[0].delay_secs as f32;
        keyframes[0].departure_time = (cur_path_time * IN_MILLISECONDS) as u32;
    }

    for i in 1..n {
        cur_path_time += keyframes[i - 1].time_to;
        if keyframes[i].is_stop_frame {
            let arrive = (cur_path_time * IN_MILLISECONDS) as u32;
            keyframes[i].arrive_time = arrive;
            keyframes[i - 1].next_arrive_time = arrive;
            cur_path_time += keyframes[i].delay_secs as f32;
            keyframes[i].departure_time = (cur_path_time * IN_MILLISECONDS) as u32;
        } else {
            // A pass-through keyframe departs the instant it arrives.
            cur_path_time -= keyframes[i].time_to;
            let arrive = (cur_path_time * IN_MILLISECONDS) as u32;
            keyframes[i].arrive_time = arrive;
            keyframes[i - 1].next_arrive_time = arrive;
            keyframes[i].departure_time = arrive;
        }
    }

    keyframes[n - 1].next_arrive_time = keyframes[n - 1].departure_time;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cruise 10 yd/s, accel 5 yd/s^2, so accel_dist 10 yd over accel_time 2 s.
    fn profile() -> ScheduleProfile {
        ScheduleProfile {
            speed: 10.0,
            accel: 5.0,
        }
    }

    /// A single-stop cyclic path: keyframe 0 is the stop, three pass-through nodes each
    /// 10 yards apart, returning to the stop by teleport.
    fn single_stop_loop() -> Vec<KeyFrame> {
        vec![
            KeyFrame::new(0.0, true, 0),
            KeyFrame::new(10.0, false, 0),
            KeyFrame::new(10.0, false, 0),
            KeyFrame::new(10.0, false, 0),
        ]
    }

    #[test]
    fn profile_derives_accel_dist_and_time() {
        let p = profile();
        assert!((p.accel_dist() - 10.0).abs() < 1e-4);
        assert!((p.accel_time() - 2.0).abs() < 1e-4);
    }

    #[test]
    fn next_dist_from_prev_wraps_the_loop() {
        let mut kf = single_stop_loop();
        compute_schedule(&profile(), &mut kf);
        let next: Vec<f32> = kf.iter().map(|k| k.next_dist_from_prev).collect();
        // Each points to the following keyframe's dist_from_prev; the last wraps to the first.
        assert_eq!(next, vec![10.0, 10.0, 10.0, 0.0]);
    }

    #[test]
    fn distances_from_the_stop_mirror_each_other() {
        let mut kf = single_stop_loop();
        compute_schedule(&profile(), &mut kf);

        let since: Vec<f32> = kf.iter().map(|k| k.dist_since_stop).collect();
        let until: Vec<f32> = kf.iter().map(|k| k.dist_until_stop).collect();
        // Leaving the stop distSinceStop climbs 0..30; distUntilStop is its mirror.
        assert_eq!(since, vec![0.0, 10.0, 20.0, 30.0]);
        assert_eq!(until, vec![30.0, 20.0, 10.0, 0.0]);
    }

    #[test]
    fn leg_times_follow_the_acceleration_profile() {
        let mut kf = single_stop_loop();
        compute_schedule(&profile(), &mut kf);

        // At the stop, the whole 30-yard run lies ahead: accelerate (2s over 10yd) then
        // cruise the remaining 20yd at 10yd/s -> 5s.
        assert!((kf[0].time_to - 5.0).abs() < 1e-4, "got {}", kf[0].time_to);
        // Mid-run, at cruise, 20yd ahead: 20/10 + half the accel time = 3s.
        assert!((kf[1].time_to - 3.0).abs() < 1e-4, "got {}", kf[1].time_to);
        // The farthest node coincides with the next stop, so nothing is left to travel.
        assert!(kf[3].time_to.abs() < 1e-4, "got {}", kf[3].time_to);
    }

    #[test]
    fn arrive_times_accumulate_along_the_path() {
        let mut kf = single_stop_loop();
        let path_time = compute_schedule(&profile(), &mut kf);

        let arrive: Vec<u32> = kf.iter().map(|k| k.arrive_time).collect();
        assert_eq!(arrive, vec![0, 2000, 3000, 5000]);
        // The last keyframe's departure is the total path time.
        assert_eq!(path_time, 5000);
        assert_eq!(kf[3].departure_time, 5000);
        // A pass-through keyframe leaves the moment it arrives.
        assert_eq!(kf[1].arrive_time, kf[1].departure_time);
    }

    #[test]
    fn a_stop_delay_pushes_back_every_later_arrival() {
        let mut kf = single_stop_loop();
        kf[0].delay_secs = 4;
        compute_schedule(&profile(), &mut kf);

        // Waiting 4s at the start departs at 4s and shifts each later arrival by the same.
        assert_eq!(kf[0].departure_time, 4000);
        assert_eq!(kf[1].arrive_time, 6000);
        assert_eq!(kf[3].arrive_time, 9000);
    }

    #[test]
    fn a_path_with_no_stops_still_schedules() {
        // No stop frames: firstStop/lastStop both collapse to keyframe 0.
        let mut kf = vec![
            KeyFrame::new(0.0, false, 0),
            KeyFrame::new(10.0, false, 0),
            KeyFrame::new(10.0, false, 0),
        ];
        let path_time = compute_schedule(&profile(), &mut kf);
        assert!(path_time > 0);
        assert_eq!(kf[0].arrive_time, 0);
    }

    #[test]
    fn an_empty_path_has_zero_path_time() {
        assert_eq!(compute_schedule(&profile(), &mut []), 0);
    }
}
