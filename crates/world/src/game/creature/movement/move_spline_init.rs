//! Building and launching a spline.
//!
//! Collects path, speed and facing into [`MoveSplineInitArgs`], then works out the
//! movement-flag transition a launch implies. The world-facing half of the C++ `Launch`
//! (transport passenger moves, the MONSTER_MOVE broadcast) is not ported - see the module
//! notes on [`LaunchPlan`].

use super::move_spline::{MoveSplineFlags, MoveSplineInitArgs, SplineFacing};
use super::spline_base::Vec3;
use super::types::MoveType;
use crate::core::common::MoveFlags;
use std::sync::atomic::{AtomicU32, Ordering};

/// Movement flags that mean the unit is under its own power.
const MASK_MOVING: u32 = 0x0000_0001 // forward
    | 0x0000_0002 // backward
    | 0x0000_0004 // strafe left
    | 0x0000_0008 // strafe right
    | 0x0000_0010 // turn left
    | 0x0000_0020 // turn right
    | 0x0000_0040 // pitch up
    | 0x0000_4000; // falling far

/// Source of spline ids. C++ uses a thread-local counter starting at 1.
static SPLINE_COUNTER: AtomicU32 = AtomicU32::new(1);

fn next_spline_id() -> u32 {
    SPLINE_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Which speed applies to a unit moving with these flags (`SelectSpeedType`).
pub fn select_speed_type(move_flags: u32) -> MoveType {
    let flags = MoveFlags::from(move_flags);

    if flags.has_flag(MoveFlags::SWIMMING) {
        if flags.has_flag(MoveFlags::BACKWARD) {
            MoveType::SwimBack
        } else {
            MoveType::Swim
        }
    } else if flags.has_flag(MoveFlags::WALK_MODE) {
        MoveType::Walk
    } else if flags.has_flag(MoveFlags::BACKWARD) {
        MoveType::RunBack
    } else {
        MoveType::Run
    }
}

/// Wrap an angle into [0, 2π), as `G3D::wrap` does.
fn wrap_angle(angle: f32) -> f32 {
    let two_pi = std::f32::consts::TAU;
    let wrapped = angle % two_pi;
    if wrapped < 0.0 {
        wrapped + two_pi
    } else {
        wrapped
    }
}

/// What a launch decided, for the caller to act on.
///
/// The caller still owns everything that touches the world: moving the unit between
/// transports, sending SMSG_MONSTER_MOVE (or the stop packet), the root and run/walk
/// broadcasts, and marking a spline-done ack pending.
#[derive(Debug, Clone, PartialEq)]
pub struct LaunchPlan {
    /// Movement flags the unit should now carry.
    pub move_flags: u32,
    /// Flags the unit carried before, for deciding which broadcasts to send.
    pub old_move_flags: u32,
    /// A stop packet is wanted rather than a spline.
    pub is_stop: bool,
}

impl LaunchPlan {
    /// Whether the launch released a root, which observers must be told about.
    pub fn releases_root(&self) -> bool {
        MoveFlags::from(self.old_move_flags).has_flag(MoveFlags::ROOT) && !self.is_stop
    }

    /// Whether the unit switched between walking and running.
    pub fn walk_mode_changed(&self) -> bool {
        let was_walking = MoveFlags::from(self.old_move_flags).has_flag(MoveFlags::WALK_MODE);
        let is_walking = MoveFlags::from(self.move_flags).has_flag(MoveFlags::WALK_MODE);
        was_walking != is_walking
    }

    /// True when the unit is now running, for the toggle broadcast.
    pub fn is_running(&self) -> bool {
        !MoveFlags::from(self.move_flags).has_flag(MoveFlags::WALK_MODE)
    }
}

/// Builder for a spline launch.
#[derive(Debug, Default, Clone)]
pub struct MoveSplineInit {
    pub args: MoveSplineInitArgs,
}

impl MoveSplineInit {
    /// Start from the unit's current movement flags, mixing existing state into the new
    /// spline the way the C++ constructor does.
    pub fn new(current_move_flags: u32) -> Self {
        let flags = MoveFlags::from(current_move_flags);
        let walking = flags.has_flag(MoveFlags::WALK_MODE);

        Self {
            args: MoveSplineInitArgs {
                flags: MoveSplineFlags {
                    walkmode: walking,
                    ..MoveSplineFlags::default()
                },
                ..MoveSplineInitArgs::default()
            },
        }
    }

    /// Move along an explicit path.
    pub fn move_by_path(&mut self, path: Vec<Vec3>) -> &mut Self {
        self.args.path = path;
        self
    }

    /// Move to a single destination. The start point is filled in at launch.
    pub fn move_to(&mut self, destination: Vec3) -> &mut Self {
        self.args.path = vec![Vec3::default(), destination];
        self
    }

    pub fn set_velocity(&mut self, velocity: f32) -> &mut Self {
        self.args.velocity = velocity;
        self
    }

    pub fn set_walk(&mut self, walk: bool) -> &mut Self {
        self.args.flags.walkmode = walk;
        self
    }

    pub fn set_cyclic(&mut self) -> &mut Self {
        self.args.flags.cyclic = true;
        self
    }

    pub fn set_smooth(&mut self) -> &mut Self {
        self.args.flags.catmullrom = true;
        self
    }

    pub fn set_falling(&mut self) -> &mut Self {
        self.args.flags.falling = true;
        self
    }

    pub fn set_first_point_id(&mut self, id: i32) -> &mut Self {
        self.args.path_idx_offset = id;
        self
    }

    pub fn set_transport(&mut self, transport_guid: u64) -> &mut Self {
        self.args.transport_guid = transport_guid;
        self
    }

    /// Face a unit for the rest of the spline.
    pub fn set_facing_guid(&mut self, guid: u64) -> &mut Self {
        self.args.facing = SplineFacing::Target(guid);
        self
    }

    /// Face a fixed angle, wrapped into [0, 2π).
    pub fn set_facing(&mut self, angle: f32) -> &mut Self {
        self.args.facing = SplineFacing::Angle(wrap_angle(angle));
        self
    }

    /// Face a fixed point.
    pub fn set_facing_point(&mut self, point: Vec3) -> &mut Self {
        self.args.facing = SplineFacing::Point(point);
        self
    }

    /// Stop the unit where it stands rather than moving it.
    pub fn set_done(&mut self) -> &mut Self {
        self.args.flags.done = true;
        self
    }

    /// Work out the flag transition and finish the args, ready for
    /// `MoveSpline::initialize`.
    ///
    /// `real_position` is where the unit actually is now - the caller resolves that,
    /// since mid-spline it has to be computed rather than read. Returns `None` when the
    /// args don't describe a usable spline, matching the C++ `Validate` early-out.
    pub fn prepare(
        &mut self,
        real_position: Vec3,
        current_move_flags: u32,
        speed_for: impl Fn(MoveType) -> f32,
        on_transport: bool,
    ) -> Option<LaunchPlan> {
        if self.args.path.is_empty() {
            self.move_to(real_position);
        }
        // The first vertex is always where the unit is standing.
        self.args.path[0] = real_position;

        let old_move_flags = current_move_flags;
        let mut move_flags = current_move_flags;

        if self.args.flags.done {
            self.args.flags = MoveSplineFlags {
                done: true,
                ..MoveSplineFlags::default()
            };
            move_flags &= !(MoveFlags::SPLINE_ENABLED.value() | MASK_MOVING);
        } else {
            move_flags |= MoveFlags::SPLINE_ENABLED.value() | MoveFlags::FORWARD.value();

            if self.args.flags.walkmode {
                move_flags |= MoveFlags::WALK_MODE.value();
            } else {
                move_flags &= !MoveFlags::WALK_MODE.value();
            }
        }

        if on_transport {
            move_flags |= MoveFlags::ONTRANSPORT.value();
        } else {
            move_flags &= !MoveFlags::ONTRANSPORT.value();
        }

        if self.args.velocity == 0.0 {
            self.args.velocity = speed_for(select_speed_type(move_flags));
        }

        if !self.args.validate() {
            return None;
        }

        self.args.spline_id = next_spline_id();

        Some(LaunchPlan {
            move_flags,
            old_move_flags,
            is_stop: self.args.flags.done,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3::new(x, y, z)
    }

    fn speeds(move_type: MoveType) -> f32 {
        match move_type {
            MoveType::Walk => 2.5,
            MoveType::Run => 7.0,
            MoveType::RunBack => 4.5,
            MoveType::Swim => 4.72,
            MoveType::SwimBack => 2.5,
            _ => 1.0,
        }
    }

    #[test]
    fn speed_type_follows_the_movement_flags() {
        assert_eq!(select_speed_type(0), MoveType::Run);
        assert_eq!(
            select_speed_type(MoveFlags::WALK_MODE.value()),
            MoveType::Walk
        );
        assert_eq!(
            select_speed_type(MoveFlags::BACKWARD.value()),
            MoveType::RunBack
        );
        assert_eq!(
            select_speed_type(MoveFlags::SWIMMING.value()),
            MoveType::Swim
        );
        assert_eq!(
            select_speed_type(MoveFlags::SWIMMING.value() | MoveFlags::BACKWARD.value()),
            MoveType::SwimBack
        );
        // Swimming outranks walk mode.
        assert_eq!(
            select_speed_type(MoveFlags::SWIMMING.value() | MoveFlags::WALK_MODE.value()),
            MoveType::Swim
        );
    }

    #[test]
    fn constructor_inherits_walk_mode_from_the_unit() {
        assert!(!MoveSplineInit::new(0).args.flags.walkmode);
        assert!(
            MoveSplineInit::new(MoveFlags::WALK_MODE.value())
                .args
                .flags
                .walkmode
        );
    }

    #[test]
    fn facing_setters_pick_the_matching_variant() {
        let mut init = MoveSplineInit::new(0);

        init.set_facing_guid(0xDEAD);
        assert_eq!(init.args.facing, SplineFacing::Target(0xDEAD));

        init.set_facing_point(v(1.0, 2.0, 3.0));
        assert_eq!(init.args.facing, SplineFacing::Point(v(1.0, 2.0, 3.0)));

        init.set_facing(1.25);
        assert_eq!(init.args.facing, SplineFacing::Angle(1.25));
    }

    #[test]
    fn facing_angles_wrap_into_a_single_turn() {
        let mut init = MoveSplineInit::new(0);

        init.set_facing(-std::f32::consts::FRAC_PI_2);
        match init.args.facing {
            SplineFacing::Angle(angle) => {
                assert!((angle - (std::f32::consts::TAU - std::f32::consts::FRAC_PI_2)).abs() < 1e-5)
            }
            other => panic!("expected an angle, got {other:?}"),
        }

        init.set_facing(std::f32::consts::TAU + 1.0);
        match init.args.facing {
            SplineFacing::Angle(angle) => assert!((angle - 1.0).abs() < 1e-5),
            other => panic!("expected an angle, got {other:?}"),
        }
    }

    #[test]
    fn prepare_fills_the_start_point_and_enables_the_spline() {
        let mut init = MoveSplineInit::new(0);
        init.move_to(v(30.0, 0.0, 0.0));

        let plan = init
            .prepare(v(5.0, 5.0, 5.0), 0, speeds, false)
            .expect("valid spline");

        assert_eq!(init.args.path[0], v(5.0, 5.0, 5.0));
        assert!(MoveFlags::from(plan.move_flags).has_flag(MoveFlags::SPLINE_ENABLED));
        assert!(MoveFlags::from(plan.move_flags).has_flag(MoveFlags::FORWARD));
        assert!(!plan.is_stop);
        // Run speed was filled in from the speed lookup.
        assert_eq!(init.args.velocity, 7.0);
        assert!(init.args.spline_id > 0);
    }

    #[test]
    fn a_done_spline_clears_the_moving_flags() {
        let mut init = MoveSplineInit::new(0);
        init.move_to(v(30.0, 0.0, 0.0));
        init.set_done();

        let moving = MoveFlags::SPLINE_ENABLED.value() | MoveFlags::FORWARD.value();
        let plan = init
            .prepare(v(0.0, 0.0, 0.0), moving, speeds, false)
            .expect("valid spline");

        assert!(plan.is_stop);
        assert!(!MoveFlags::from(plan.move_flags).has_flag(MoveFlags::SPLINE_ENABLED));
        assert!(!MoveFlags::from(plan.move_flags).has_flag(MoveFlags::FORWARD));
    }

    #[test]
    fn walk_mode_is_applied_from_the_spline_flags() {
        let mut init = MoveSplineInit::new(MoveFlags::WALK_MODE.value());
        init.move_to(v(30.0, 0.0, 0.0));

        let plan = init
            .prepare(
                v(0.0, 0.0, 0.0),
                MoveFlags::WALK_MODE.value(),
                speeds,
                false,
            )
            .expect("valid spline");

        assert!(MoveFlags::from(plan.move_flags).has_flag(MoveFlags::WALK_MODE));
        assert!(!plan.walk_mode_changed());
        assert!(!plan.is_running());
        // Walk speed, not run speed.
        assert_eq!(init.args.velocity, 2.5);

        // Switching to running clears the flag and reports the change.
        let mut init = MoveSplineInit::new(MoveFlags::WALK_MODE.value());
        init.move_to(v(30.0, 0.0, 0.0));
        init.set_walk(false);
        let plan = init
            .prepare(
                v(0.0, 0.0, 0.0),
                MoveFlags::WALK_MODE.value(),
                speeds,
                false,
            )
            .expect("valid spline");
        assert!(plan.walk_mode_changed());
        assert!(plan.is_running());
    }

    #[test]
    fn transport_state_toggles_the_ontransport_flag() {
        let mut init = MoveSplineInit::new(0);
        init.move_to(v(30.0, 0.0, 0.0));
        let plan = init
            .prepare(v(0.0, 0.0, 0.0), 0, speeds, true)
            .expect("valid spline");
        assert!(MoveFlags::from(plan.move_flags).has_flag(MoveFlags::ONTRANSPORT));

        let mut init = MoveSplineInit::new(0);
        init.move_to(v(30.0, 0.0, 0.0));
        let plan = init
            .prepare(
                v(0.0, 0.0, 0.0),
                MoveFlags::ONTRANSPORT.value(),
                speeds,
                false,
            )
            .expect("valid spline");
        assert!(!MoveFlags::from(plan.move_flags).has_flag(MoveFlags::ONTRANSPORT));
    }

    #[test]
    fn a_rooted_unit_launching_a_spline_reports_the_root_release() {
        let mut init = MoveSplineInit::new(0);
        init.move_to(v(30.0, 0.0, 0.0));

        let plan = init
            .prepare(v(0.0, 0.0, 0.0), MoveFlags::ROOT.value(), speeds, false)
            .expect("valid spline");

        assert!(plan.releases_root());
    }

    #[test]
    fn prepare_rejects_a_spline_with_no_usable_velocity() {
        let mut init = MoveSplineInit::new(0);
        init.move_to(v(30.0, 0.0, 0.0));

        // A speed lookup that returns zero leaves the args invalid.
        assert!(init.prepare(v(0.0, 0.0, 0.0), 0, |_| 0.0, false).is_none());
    }

    #[test]
    fn spline_ids_are_unique_and_increasing() {
        let mut first = MoveSplineInit::new(0);
        first.move_to(v(10.0, 0.0, 0.0));
        first.prepare(v(0.0, 0.0, 0.0), 0, speeds, false).unwrap();

        let mut second = MoveSplineInit::new(0);
        second.move_to(v(10.0, 0.0, 0.0));
        second.prepare(v(0.0, 0.0, 0.0), 0, speeds, false).unwrap();

        assert!(second.args.spline_id > first.args.spline_id);
    }
}
