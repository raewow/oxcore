//! Server-side spline state: where a unit is along its path at any moment.
//!
//! This is the faithful port of the reference `MoveSpline`, built on the geometry primitives in
//! [`super::spline_base`]. It is not yet wired into creature movement, which still uses the
//! simpler linear [`super::spline::MoveSpline`].

use super::spline_base::{EvaluationMode, Spline, Vec3};
use std::fmt;

/// Shortest duration a spline may have, in milliseconds.
const MINIMAL_DURATION: i32 = 1;

/// Fall physics constants shared with the client.
const GRAVITY: f32 = 19.291_105;
const TERMINAL_VELOCITY: f32 = 60.148_003;
const TERMINAL_SAFE_FALL_VELOCITY: f32 = 7.0;

/// Distance after which a fall reaches terminal velocity.
fn terminal_length() -> f32 {
    TERMINAL_VELOCITY * TERMINAL_VELOCITY / (2.0 * GRAVITY)
}

fn terminal_safe_fall_length() -> f32 {
    TERMINAL_SAFE_FALL_VELOCITY * TERMINAL_SAFE_FALL_VELOCITY / (2.0 * GRAVITY)
}

fn terminal_fall_time() -> f32 {
    TERMINAL_VELOCITY / GRAVITY
}

/// Seconds needed to fall `path_length` yards.
pub fn compute_fall_time(path_length: f32, is_safe_fall: bool) -> f32 {
    if path_length < 0.0 {
        return 0.0;
    }

    if is_safe_fall {
        if path_length >= terminal_safe_fall_length() {
            (path_length - terminal_safe_fall_length()) / TERMINAL_SAFE_FALL_VELOCITY
                + TERMINAL_SAFE_FALL_VELOCITY / GRAVITY
        } else {
            (2.0 * path_length / GRAVITY).sqrt()
        }
    } else if path_length >= terminal_length() {
        (path_length - terminal_length()) / TERMINAL_VELOCITY + terminal_fall_time()
    } else {
        (2.0 * path_length / GRAVITY).sqrt()
    }
}

/// Yards fallen after `time_passed` seconds.
pub fn compute_fall_elevation(time_passed: f32, is_safe_fall: bool, start_velocity: f32) -> f32 {
    let terminal_velocity = if is_safe_fall {
        TERMINAL_SAFE_FALL_VELOCITY
    } else {
        TERMINAL_VELOCITY
    };
    let start_velocity = start_velocity.min(terminal_velocity);
    let terminal_time = terminal_fall_time() - start_velocity / GRAVITY;

    if time_passed > terminal_time {
        TERMINAL_VELOCITY * (time_passed - terminal_time)
            + start_velocity * terminal_time
            + GRAVITY * terminal_time * terminal_time * 0.5
    } else {
        time_passed * (start_velocity + time_passed * GRAVITY * 0.5)
    }
}

/// What the unit should face when the spline completes.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum SplineFacing {
    #[default]
    None,
    Angle(f32),
    Point(Vec3),
    Target(u64),
}

/// Behaviour flags carried by a spline.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MoveSplineFlags {
    pub done: bool,
    pub falling: bool,
    pub cyclic: bool,
    pub catmullrom: bool,
    pub walkmode: bool,
}

impl MoveSplineFlags {
    /// Smooth splines interpolate through their points instead of between them.
    pub fn is_smooth(&self) -> bool {
        self.catmullrom
    }

    /// Whether the spline ends facing something in particular.
    pub fn is_facing(&self, facing: &SplineFacing) -> bool {
        !matches!(facing, SplineFacing::None)
    }

    fn evaluation_mode(&self) -> EvaluationMode {
        if self.is_smooth() {
            EvaluationMode::CatmullRom
        } else {
            EvaluationMode::Linear
        }
    }
}

impl fmt::Display for MoveSplineFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut names = Vec::new();
        if self.done {
            names.push("Done");
        }
        if self.falling {
            names.push("Falling");
        }
        if self.cyclic {
            names.push("Cyclic");
        }
        if self.catmullrom {
            names.push("Catmullrom");
        }
        if self.walkmode {
            names.push("Walkmode");
        }

        if names.is_empty() {
            f.write_str("None")
        } else {
            f.write_str(&names.join(", "))
        }
    }
}

/// Names of the 32 raw spline flag bits, indexed by bit position.
///
/// Bits with no defined meaning keep their `Unknown<n>` label so the rendered string still
/// accounts for every set bit, exactly as the reference debug output does.
const SPLINE_FLAG_NAMES: [&str; 32] = [
    "Done",
    "Falling",
    "Unknown3",
    "Unknown4",
    "Unknown5",
    "Unknown6",
    "Unknown7",
    "Unknown8",
    "Runmode",
    "Flying",
    "No_Spline",
    "Unknown12",
    "Unknown13",
    "Unknown14",
    "Unknown15",
    "Unknown16",
    "Final_Point",
    "Final_Target",
    "Final_Angle",
    "Unknown19",
    "Cyclic",
    "Enter_Cycle",
    "Frozen",
    "Unknown23",
    "Unknown24",
    "Unknown25",
    "Unknown26",
    "Unknown27",
    "Unknown28",
    "Unknown29",
    "Unknown30",
    "Unknown31",
];

/// Render the raw spline flag word as its set bit names.
///
/// Each set bit contributes a leading space then its name, low bit first, so the result is
/// `" Done Falling"` for the two lowest bits and empty when nothing is set - matching the
/// reference `print_flags` output byte for byte.
pub fn format_move_spline_flags(raw: u32) -> String {
    let mut out = String::new();
    for (bit, name) in SPLINE_FLAG_NAMES.iter().enumerate() {
        if raw & (1 << bit) != 0 {
            out.push(' ');
            out.push_str(name);
        }
    }
    out
}

/// Everything needed to start a spline.
#[derive(Debug, Default, Clone)]
pub struct MoveSplineInitArgs {
    pub path: Vec<Vec3>,
    pub path_idx_offset: i32,
    pub velocity: f32,
    pub facing: SplineFacing,
    pub flags: MoveSplineFlags,
    pub spline_id: u32,
    pub transport_guid: u64,
    pub uninterruptible: bool,
}

impl MoveSplineInitArgs {
    /// Whether these arguments describe a usable spline.
    pub fn validate(&self) -> bool {
        self.path.len() > 1 && self.velocity > 0.0
    }

    /// Whether every intermediate point fits the MONSTER_MOVE packet's 11-bit offsets.
    ///
    /// Catmull-Rom paths send absolute points, so they are exempt. The reference core has this
    /// check commented out of `Validate`; it is kept callable here for the same reason it
    /// exists - the packet writer needs it.
    pub fn check_path_bounds(&self) -> bool {
        const MAX_OFFSET: f32 = ((1 << 11) / 2) as f32;

        if self.flags.catmullrom || self.path.len() <= 2 {
            return true;
        }

        let front = self.path[0];
        let back = self.path[self.path.len() - 1];
        let middle = Vec3::new(
            (front.x + back.x) / 2.0,
            (front.y + back.y) / 2.0,
            (front.z + back.z) / 2.0,
        );

        self.path[1..self.path.len() - 1].iter().all(|point| {
            (point.x - middle.x).abs() < MAX_OFFSET
                && (point.y - middle.y).abs() < MAX_OFFSET
                && (point.z - middle.z).abs() < MAX_OFFSET
        })
    }
}

/// A position along a spline, with the facing the unit should hold there.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct SplineLocation {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: f32,
}

/// Outcome of advancing a spline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateResult {
    None,
    Arrived,
    NextSegment,
}

/// Where a unit is along its current path.
#[derive(Debug, Default, Clone)]
pub struct MoveSpline {
    spline: Spline,
    facing: SplineFacing,
    flags: MoveSplineFlags,
    id: u32,
    transport_guid: u64,
    time_passed: i32,
    point_idx: usize,
    point_idx_offset: i32,
    last_point_sent_idx: i32,
    uninterruptible: bool,
}

impl MoveSpline {
    /// A spline that has already finished, which is the resting state of a unit.
    pub fn new() -> Self {
        Self {
            last_point_sent_idx: -1,
            flags: MoveSplineFlags {
                done: true,
                ..MoveSplineFlags::default()
            },
            ..Self::default()
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn flags(&self) -> MoveSplineFlags {
        self.flags
    }

    pub fn transport_guid(&self) -> u64 {
        self.transport_guid
    }

    pub fn is_uninterruptible(&self) -> bool {
        self.uninterruptible
    }

    pub fn is_cyclic(&self) -> bool {
        self.flags.cyclic
    }

    /// A spline is initialized once it has at least one evaluable segment.
    pub fn initialized(&self) -> bool {
        !self.spline.is_empty()
    }

    pub fn finalized(&self) -> bool {
        self.flags.done
    }

    /// Total duration of the spline, in milliseconds.
    pub fn duration(&self) -> i32 {
        self.spline.length()
    }

    pub fn time_passed(&self) -> i32 {
        self.time_passed
    }

    /// Timestamp at which the current segment ends.
    fn next_timestamp(&self) -> i32 {
        self.spline.length_at(self.point_idx + 1)
    }

    /// Milliseconds left in the current segment.
    fn segment_time_elapsed(&self) -> i32 {
        self.next_timestamp() - self.time_passed
    }

    pub fn final_destination(&self) -> Vec3 {
        self.spline.point(self.spline.last()).unwrap_or_default()
    }

    pub fn facing(&self) -> SplineFacing {
        self.facing
    }

    /// First point of the curve, where the unit started.
    pub fn start_point(&self) -> Option<Vec3> {
        self.spline.point(self.spline.first())
    }

    /// Every point held by the curve, including the virtual ones.
    pub fn curve_points(&self) -> &[Vec3] {
        self.spline.base().points()
    }

    /// The path the caller supplied, without the leading virtual point.
    ///
    /// Linear splines keep one virtual point at the front; the packet builder walks
    /// this slice when packing intermediate offsets.
    pub fn real_path(&self) -> &[Vec3] {
        let points = self.curve_points();
        if points.len() <= 1 {
            return &[];
        }
        &points[1..]
    }

    /// Start a new spline from `args`, replacing any current one.
    pub fn initialize(&mut self, args: &MoveSplineInitArgs) {
        self.flags = args.flags;
        self.facing = args.facing;
        self.id = args.spline_id;
        self.point_idx_offset = args.path_idx_offset;
        self.time_passed = 0;
        self.transport_guid = args.transport_guid;
        self.last_point_sent_idx = -1;
        self.uninterruptible = args.uninterruptible;

        self.init_spline(args);
    }

    /// Build the curve and its per-point timestamps.
    fn init_spline(&mut self, args: &MoveSplineInitArgs) {
        let mode = args.flags.evaluation_mode();

        if args.flags.cyclic {
            // Entering a cycle at a later point is not supported by this client.
            self.spline.init_cyclic_spline(&args.path, mode, 0);
        } else {
            self.spline.init_spline(&args.path, mode);
        }

        if self.flags.falling {
            // Falling splines are timed by gravity rather than by velocity.
            let start_elevation = self.spline.point(self.spline.first()).unwrap_or_default().z;
            self.spline.init_lengths_with(|spline, index| {
                let target_z = spline.point(index + 1).unwrap_or_default().z;
                (compute_fall_time(start_elevation - target_z, false) * 1000.0) as i32
            });
        } else {
            let velocity_inv = 1000.0 / args.velocity;
            let mut time = MINIMAL_DURATION;
            self.spline.init_lengths_with(move |spline, index| {
                let seg_length = spline.base().seg_length(index).unwrap_or(0.0);
                time += (seg_length * velocity_inv) as i32;
                time
            });
        }

        // All points at the same coordinates would otherwise yield a zero-length spline.
        if self.spline.length() < MINIMAL_DURATION {
            tracing::error!("MoveSpline::init_spline: zero length spline, wrong input data?");
            let fallback = if self.spline.is_cyclic() { 1000 } else { 1 };
            self.spline.set_length(self.spline.last(), fallback);
        }

        self.point_idx = self.spline.first();
    }

    /// Position at the current time.
    pub fn compute_position(&self) -> Option<SplineLocation> {
        if !self.initialized() {
            return None;
        }

        self.compute_position_at(self.point_idx, self.time_passed)
    }

    /// Position the unit will hold `duration` milliseconds from now.
    pub fn compute_position_after_time(&self, duration: i32) -> Option<SplineLocation> {
        if !self.initialized() {
            return None;
        }

        let mut last_index = self.point_idx;
        let time_passed = self.time_passed.max(self.spline.length_at(last_index));

        for index in (self.point_idx + 1)..=self.spline.last() {
            if self.spline.length_at(index) - time_passed > duration {
                break;
            }
            last_index = index;
        }

        if last_index == self.spline.last() {
            let destination = self.final_destination();
            return Some(SplineLocation {
                x: destination.x,
                y: destination.y,
                z: destination.z,
                orientation: 0.0,
            });
        }

        self.compute_position_at(last_index, duration + time_passed)
    }

    fn compute_position_at(&self, index: usize, desired_time: i32) -> Option<SplineLocation> {
        let segment_time = self.spline.length_between(index, index + 1);
        let u = if segment_time > 0 {
            (desired_time - self.spline.length_at(index)) as f32 / segment_time as f32
        } else {
            1.0
        };

        let point = self.spline.evaluate_percent_in_segment(index, u)?;
        let mut location = SplineLocation {
            x: point.x,
            y: point.y,
            z: point.z,
            orientation: 0.0,
        };

        if self.flags.falling {
            location.z = self.compute_fall_elevation_at(location.z);
        }

        if self.flags.done && self.flags.is_facing(&self.facing) {
            match self.facing {
                SplineFacing::Angle(angle) => location.orientation = angle,
                SplineFacing::Point(target) => {
                    location.orientation = (target.y - location.y).atan2(target.x - location.x);
                }
                // A facing target is resolved by the caller, which can look the unit up.
                SplineFacing::Target(_) | SplineFacing::None => {}
            }
        } else if let Some(tangent) = self.spline.evaluate_derivative_in_segment(index, u) {
            location.orientation = tangent.y.atan2(tangent.x);
        }

        Some(location)
    }

    /// Height of a falling unit, floored at the destination height.
    fn compute_fall_elevation_at(&self, current_z: f32) -> f32 {
        let start_z = self.spline.point(self.spline.first()).unwrap_or_default().z;
        let fallen = compute_fall_elevation(self.time_passed as f32 / 1000.0, false, 0.0);
        let z_now = start_z - fallen;
        let final_z = self.final_destination().z;

        let _ = current_z;
        if z_now < final_z {
            final_z
        } else {
            z_now
        }
    }

    /// Advance the spline, consuming from `ms_time_diff` what this segment can absorb.
    pub fn update_state(&mut self, ms_time_diff: &mut i32) -> UpdateResult {
        if self.finalized() {
            *ms_time_diff = 0;
            return UpdateResult::Arrived;
        }

        let mut result = UpdateResult::None;

        let minimal_diff = (*ms_time_diff).min(self.segment_time_elapsed()).max(0);
        self.time_passed += minimal_diff;
        *ms_time_diff -= minimal_diff;

        if self.time_passed >= self.next_timestamp() {
            self.point_idx += 1;
            if self.point_idx < self.spline.last() {
                result = UpdateResult::NextSegment;
            } else if self.spline.is_cyclic() {
                self.point_idx = self.spline.first();
                let duration = self.duration();
                if duration > 0 {
                    self.time_passed %= duration;
                }
                result = UpdateResult::NextSegment;
            } else {
                self.finalize();
                *ms_time_diff = 0;
                result = UpdateResult::Arrived;
            }
        }

        result
    }

    /// Park the spline at its destination.
    fn finalize(&mut self) {
        self.flags.done = true;
        self.point_idx = self.spline.last().saturating_sub(1);
        self.time_passed = self.duration();
    }

    /// Index into the caller's original path, rather than into the spline's points.
    pub fn current_path_idx(&self) -> i32 {
        let mut point = self.point_idx_offset + self.point_idx as i32 - self.spline.first() as i32
            + i32::from(self.finalized());

        if self.is_cyclic() {
            let span = self.spline.last() as i32 - self.spline.first() as i32;
            if span > 0 {
                point %= span;
            }
        }

        point
    }
}

impl fmt::Display for MoveSpline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "MoveSpline")?;
        writeln!(f, "spline Id: {}", self.id)?;
        writeln!(f, "flags: {}", self.flags)?;
        match self.facing {
            SplineFacing::Angle(angle) => writeln!(f, "facing  angle: {angle}")?,
            SplineFacing::Target(target) => writeln!(f, "facing target: {target}")?,
            SplineFacing::Point(point) => {
                writeln!(f, "facing  point: {} {} {}", point.x, point.y, point.z)?
            }
            SplineFacing::None => writeln!(f)?,
        }
        writeln!(f, "time passed: {}", self.time_passed)?;
        writeln!(f, "total  time: {}", self.duration())?;
        writeln!(f, "spline point Id: {}", self.point_idx)?;
        writeln!(f, "path  point  Id: {}", self.current_path_idx())?;
        write!(f, "{}", self.spline.base())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3::new(x, y, z)
    }

    #[test]
    fn raw_spline_flags_render_low_bit_first_with_a_leading_space() {
        // Done (bit 0) and Falling (bit 1).
        assert_eq!(format_move_spline_flags(0x3), " Done Falling");
        // A single high bit: Cyclic is bit 20 (0x00100000).
        assert_eq!(format_move_spline_flags(0x0010_0000), " Cyclic");
        // Runmode (bit 8) and Flying (bit 9), rendered in bit order.
        assert_eq!(format_move_spline_flags(0x0000_0300), " Runmode Flying");
        // Reserved bits still account for themselves so nothing is silently dropped.
        assert_eq!(format_move_spline_flags(0x4), " Unknown3");
        // No bits set renders empty, not "None".
        assert_eq!(format_move_spline_flags(0), "");
    }

    /// A straight 30-yard path at 10 yards/sec: 3 seconds end to end.
    fn straight_args() -> MoveSplineInitArgs {
        MoveSplineInitArgs {
            path: vec![v(0.0, 0.0, 0.0), v(10.0, 0.0, 0.0), v(30.0, 0.0, 0.0)],
            velocity: 10.0,
            spline_id: 7,
            ..MoveSplineInitArgs::default()
        }
    }

    #[test]
    fn a_fresh_spline_is_finished_and_uninitialized() {
        let spline = MoveSpline::new();

        assert!(spline.finalized());
        assert!(!spline.initialized());
        assert!(spline.compute_position().is_none());
    }

    #[test]
    fn validate_requires_a_real_path_and_a_positive_velocity() {
        assert!(straight_args().validate());

        let mut args = straight_args();
        args.path.truncate(1);
        assert!(!args.validate());

        let mut args = straight_args();
        args.velocity = 0.0;
        assert!(!args.validate());
    }

    #[test]
    fn path_bounds_reject_points_too_far_from_the_midpoint() {
        let mut args = straight_args();
        assert!(args.check_path_bounds());

        // A mid-point far outside the 11-bit packed offset range.
        args.path[1] = v(5_000.0, 0.0, 0.0);
        assert!(!args.check_path_bounds());

        // Catmull-Rom paths send absolute points, so they are exempt.
        args.flags.catmullrom = true;
        assert!(args.check_path_bounds());

        // Two-point paths carry no intermediate offsets.
        let mut args = straight_args();
        args.path = vec![v(0.0, 0.0, 0.0), v(9_000.0, 0.0, 0.0)];
        assert!(args.check_path_bounds());
    }

    #[test]
    fn initialize_times_the_path_by_velocity() {
        let mut spline = MoveSpline::new();
        spline.initialize(&straight_args());

        assert!(spline.initialized());
        assert!(!spline.finalized());
        assert_eq!(spline.id(), 7);
        // 30 yards at 10 yards/sec, plus the 1ms minimum.
        assert_eq!(spline.duration(), 3_001);
    }

    #[test]
    fn position_tracks_time_along_the_path() {
        let mut spline = MoveSpline::new();
        spline.initialize(&straight_args());

        let start = spline.compute_position().unwrap();
        assert!((start.x - 0.0).abs() < 0.01);

        // Half a second in: 5 yards along the first segment.
        let mut diff = 500;
        spline.update_state(&mut diff);
        let moved = spline.compute_position().unwrap();
        assert!((moved.x - 5.0).abs() < 0.1, "got {}", moved.x);
        // Facing follows the direction of travel, which is +x here.
        assert!(moved.orientation.abs() < 0.01);
    }

    #[test]
    fn update_state_reports_segment_changes_then_arrival() {
        let mut spline = MoveSpline::new();
        spline.initialize(&straight_args());

        // Cross into the second segment.
        let mut diff = 1_200;
        assert_eq!(spline.update_state(&mut diff), UpdateResult::NextSegment);

        // Run well past the end.
        let mut diff = 10_000;
        let mut result = spline.update_state(&mut diff);
        while result == UpdateResult::NextSegment {
            result = spline.update_state(&mut diff);
        }

        assert_eq!(result, UpdateResult::Arrived);
        assert!(spline.finalized());
        assert_eq!(diff, 0);
        assert_eq!(spline.time_passed(), spline.duration());
    }

    #[test]
    fn an_already_finished_spline_absorbs_no_time() {
        let mut spline = MoveSpline::new();

        let mut diff = 500;
        assert_eq!(spline.update_state(&mut diff), UpdateResult::Arrived);
        assert_eq!(diff, 0);
    }

    #[test]
    fn cyclic_splines_wrap_instead_of_finishing() {
        let mut args = straight_args();
        args.flags.cyclic = true;
        let mut spline = MoveSpline::new();
        spline.initialize(&args);

        let mut diff = 10_000;
        let mut result = spline.update_state(&mut diff);
        while result == UpdateResult::NextSegment && diff > 0 {
            result = spline.update_state(&mut diff);
        }

        // A loop never arrives.
        assert!(!spline.finalized());
    }

    #[test]
    fn a_degenerate_path_still_gets_a_usable_duration() {
        let mut spline = MoveSpline::new();
        spline.initialize(&MoveSplineInitArgs {
            // Every point at the same place: zero length.
            path: vec![v(5.0, 5.0, 5.0), v(5.0, 5.0, 5.0)],
            velocity: 10.0,
            ..MoveSplineInitArgs::default()
        });

        assert!(spline.duration() >= 1);
    }

    #[test]
    fn position_after_time_looks_ahead_without_advancing() {
        let mut spline = MoveSpline::new();
        spline.initialize(&straight_args());

        let ahead = spline.compute_position_after_time(1_500).unwrap();
        assert!(ahead.x > 10.0, "expected to be past the first point");
        // Looking ahead must not consume time.
        assert_eq!(spline.time_passed(), 0);

        // Looking beyond the end yields the destination.
        let past_end = spline.compute_position_after_time(60_000).unwrap();
        assert!((past_end.x - 30.0).abs() < 0.01);
    }

    #[test]
    fn final_facing_is_applied_once_the_spline_is_done() {
        let mut args = straight_args();
        args.facing = SplineFacing::Angle(1.25);
        let mut spline = MoveSpline::new();
        spline.initialize(&args);

        let mut diff = 10_000;
        while spline.update_state(&mut diff) == UpdateResult::NextSegment {}

        let end = spline.compute_position().unwrap();
        assert!((end.orientation - 1.25).abs() < 0.001);
    }

    #[test]
    fn fall_timing_matches_the_client_curve() {
        // Below terminal velocity the fall is the usual sqrt(2h/g).
        let short = compute_fall_time(10.0, false);
        assert!((short - (2.0f32 * 10.0 / GRAVITY).sqrt()).abs() < 0.001);

        // A negative drop takes no time at all.
        assert_eq!(compute_fall_time(-5.0, false), 0.0);

        // Elevation grows with time.
        assert!(compute_fall_elevation(2.0, false, 0.0) > compute_fall_elevation(1.0, false, 0.0));

        // The safe-fall flag only clamps the starting velocity: past terminal time the
        // formula uses the unsafe terminal velocity either way, so a fall starting
        // from rest is identical with and without it.
        assert_eq!(
            compute_fall_elevation(5.0, true, 0.0),
            compute_fall_elevation(5.0, false, 0.0)
        );

        // With a starting velocity above the safe cap, the clamp does show up.
        assert!(compute_fall_elevation(1.0, true, 20.0) < compute_fall_elevation(1.0, false, 20.0));
    }

    #[test]
    fn current_path_idx_follows_the_callers_offset() {
        let mut args = straight_args();
        args.path_idx_offset = 10;
        let mut spline = MoveSpline::new();
        spline.initialize(&args);

        assert_eq!(spline.current_path_idx(), 10);

        let mut diff = 1_200;
        spline.update_state(&mut diff);
        assert_eq!(spline.current_path_idx(), 11);
    }

    #[test]
    fn display_reports_the_spline_state() {
        let mut spline = MoveSpline::new();
        spline.initialize(&straight_args());
        let text = spline.to_string();

        assert!(text.starts_with("MoveSpline\nspline Id: 7\n"));
        assert!(text.contains("total  time: 3001"));
        assert!(text.contains("mode: Linear"));
        assert_eq!(
            MoveSplineFlags {
                done: true,
                cyclic: true,
                ..MoveSplineFlags::default()
            }
            .to_string(),
            "Done, Cyclic"
        );
    }
}
