//! MotionMaster - manages movement generators for a creature

use super::generator::{MovementGenerator, MovementUpdate};
use super::generators::{
    AssistanceDistractMovementGenerator, AssistanceMovementGenerator, ChargeMovementGenerator,
    ChaseMovementGenerator, ConfusedMovementGenerator, DistractMovementGenerator,
    FearMovementGenerator, FleeMovementGenerator, FollowMovementGenerator, HomeMovementGenerator,
    IdleMovementGenerator, PointMovementGenerator, RandomMovementGenerator,
    TimedFearMovementGenerator, Waypoint, WaypointMovementGenerator,
};
use super::types::MovementGeneratorType;
use oxcore_shared::protocol::{ObjectGuid, Position};
use std::collections::BTreeMap;

/// MotionMaster flags for state tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionMasterFlags {
    bits: u8,
}

impl MotionMasterFlags {
    pub const NONE: u8 = 0x00;
    pub const UPDATING: u8 = 0x01; // Re-entrant protection
    pub const PAUSED: u8 = 0x02; // Movement paused

    pub fn new() -> Self {
        Self { bits: Self::NONE }
    }

    pub fn contains(&self, flag: u8) -> bool {
        (self.bits & flag) != 0
    }

    pub fn insert(&mut self, flag: u8) {
        self.bits |= flag;
    }

    pub fn remove(&mut self, flag: u8) {
        self.bits &= !flag;
    }
}

/// Manages movement generators for a creature
pub struct MotionMaster {
    /// Generators by type (for quick lookup)
    generators: BTreeMap<MovementGeneratorType, Box<dyn MovementGenerator>>,
    /// Current active generator type
    active_type: MovementGeneratorType,
    /// Current movement destination
    current_destination: Option<Position>,
    /// Movement in progress
    moving: bool,
    /// Whether an async update has been requested
    needs_async_update: bool,
    /// Generators retired by a delayed clean/expire, dropped on the next update.
    ///
    /// `Some` (even when empty) marks that a delayed operation ran this tick, which is
    /// what drives the post-update re-initialize and reset in [`Self::update_motion`].
    expire_list: Option<Vec<Box<dyn MovementGenerator>>>,
    /// A delayed clean/expire asked for the new top generator to be reset.
    pending_reset: bool,
    /// State flags (updating, paused, etc.)
    pub flags: MotionMasterFlags,
}

impl std::fmt::Debug for MotionMaster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MotionMaster")
            .field("active_type", &self.active_type)
            .field("current_destination", &self.current_destination)
            .field("moving", &self.moving)
            .field("flags", &self.flags)
            .field("generator_count", &self.generators.len())
            .finish()
    }
}

impl MotionMaster {
    /// Human-readable movement generator name.
    pub fn get_movement_generator_type_name(generator: MovementGeneratorType) -> &'static str {
        match generator {
            MovementGeneratorType::Idle => "IDLE_MOTION_TYPE",
            MovementGeneratorType::Random => "RANDOM_MOTION_TYPE",
            MovementGeneratorType::Waypoint => "WAYPOINT_MOTION_TYPE",
            MovementGeneratorType::Follow => "FOLLOW_MOTION_TYPE",
            MovementGeneratorType::Distract => "DISTRACT_MOTION_TYPE",
            MovementGeneratorType::Confused => "CONFUSED_MOTION_TYPE",
            MovementGeneratorType::Point => "POINT_MOTION_TYPE",
            MovementGeneratorType::Chase => "CHASE_MOTION_TYPE",
            MovementGeneratorType::Fleeing => "FLEEING_MOTION_TYPE",
            MovementGeneratorType::Home => "HOME_MOTION_TYPE",
            MovementGeneratorType::Effect => "EFFECT_MOTION_TYPE",
            MovementGeneratorType::Taxi => "FLIGHT_MOTION_TYPE",
            MovementGeneratorType::Charge => "CHARGE_MOTION_TYPE",
        }
    }

    /// Current generator type.
    pub fn get_current_movement_generator_type(&self) -> MovementGeneratorType {
        self.active_type
    }

    /// List all movement generator types currently present.
    pub fn get_used_movement_generators_list(&self) -> Vec<MovementGeneratorType> {
        self.generators
            .values()
            .map(|generator| generator.generator_type())
            .collect()
    }

    /// Check if the creature is using only idle/default movement.
    pub fn is_using_idle_or_default_movement(&self) -> bool {
        let current_type = self.get_current_movement_generator_type();
        if current_type == MovementGeneratorType::Idle {
            return true;
        }

        (current_type < MovementGeneratorType::Chase
            || current_type == MovementGeneratorType::Waypoint)
            && self.generators.len() <= 1
    }

    /// Get the current destination if a spline is active.
    pub fn get_destination(&self) -> Option<Position> {
        self.current_destination
    }

    /// Check whether the provided generator is the active top-of-stack generator.
    pub fn is_top_generator(&self, generator: &dyn MovementGenerator) -> bool {
        self.generators
            .get(&self.active_type)
            .is_some_and(|active| std::ptr::eq::<dyn MovementGenerator>(&**active, generator))
    }

    /// Update final distance to the target on the active generator, if supported.
    pub fn update_final_distance_to_target(&mut self, distance: f32) {
        if let Some(top) = self.top_generator_mut() {
            top.update_final_distance(distance);
        }
    }

    /// Remove all generators of the specified type.
    pub fn clear_type(&mut self, move_type: MovementGeneratorType, creature_guid: ObjectGuid) {
        if let Some(mut generator) = self.generators.remove(&move_type) {
            generator.finalize(creature_guid);
        }

        self.update_active(creature_guid);
    }

    /// Reinitialize the active patrol movement if one exists.
    ///
    /// Patrol paths have no dedicated generator type here, so the waypoint generator is the
    /// one restarted.
    pub fn reinitialize_patrol_movement(&mut self, creature_guid: ObjectGuid) {
        if let Some(generator) = self.generators.get_mut(&MovementGeneratorType::Waypoint) {
            generator.reset(creature_guid);
        }
    }

    /// Extend the pause before the next leg of out-of-combat movement.
    ///
    /// Only random and waypoint movement can be paused this way; anything else ignores it.
    /// Never shortens an existing pause.
    pub fn add_pause_time(&mut self, pause_time_ms: u32) -> bool {
        match self.top_type() {
            Some(MovementGeneratorType::Random) => self
                .generators
                .get_mut(&MovementGeneratorType::Random)
                .and_then(|gen| gen.as_any_mut().downcast_mut::<RandomMovementGenerator>())
                .map(|random| {
                    random.add_pause_time(pause_time_ms);
                    true
                })
                .unwrap_or(false),
            Some(MovementGeneratorType::Waypoint) => self
                .generators
                .get_mut(&MovementGeneratorType::Waypoint)
                .and_then(|gen| gen.as_any_mut().downcast_mut::<WaypointMovementGenerator>())
                .map(|waypoint| {
                    waypoint.add_pause_time(pause_time_ms);
                    true
                })
                .unwrap_or(false),
            _ => false,
        }
    }

    pub fn new() -> Self {
        let mut mm = Self {
            generators: BTreeMap::new(),
            active_type: MovementGeneratorType::Idle,
            current_destination: None,
            moving: false,
            needs_async_update: false,
            expire_list: None,
            pending_reset: false,
            flags: MotionMasterFlags::new(), // Initialize flags
        };

        // Always have idle generator
        mm.generators.insert(
            MovementGeneratorType::Idle,
            Box::new(IdleMovementGenerator::new()),
        );

        mm
    }

    /// Get the active generator type
    pub fn active_generator(&self) -> MovementGeneratorType {
        self.active_type
    }

    /// Is creature currently moving
    pub fn is_moving(&self) -> bool {
        self.moving
    }

    /// Type of the top (highest priority) generator, if any.
    fn top_type(&self) -> Option<MovementGeneratorType> {
        self.generators.keys().next_back().copied()
    }

    /// Remove and return the top generator.
    fn pop_top(&mut self) -> Option<(MovementGeneratorType, Box<dyn MovementGenerator>)> {
        let gen_type = self.top_type()?;
        self.generators.remove(&gen_type).map(|gen| (gen_type, gen))
    }

    fn top_generator_mut(&mut self) -> Option<&mut Box<dyn MovementGenerator>> {
        let gen_type = self.top_type()?;
        self.generators.get_mut(&gen_type)
    }

    /// The idle generator is a shared static and is never retired.
    fn is_static(gen_type: MovementGeneratorType) -> bool {
        gen_type == MovementGeneratorType::Idle
    }

    /// Drop every generator, including the default one, without finalizing.
    fn drop_all_generators(&mut self, creature_guid: ObjectGuid) {
        let mut retired: Vec<_> = std::mem::take(&mut self.generators).into_values().collect();
        for gen in retired.iter_mut() {
            gen.finalize(creature_guid);
        }
    }

    /// Initialize motion state and ensure an idle generator exists.
    ///
    /// Stopping the current spline and picking the creature's default generator via the
    /// factory selector belong to the caller, which owns creature state; here the stack is
    /// rebuilt with idle as the default.
    pub fn initialize(&mut self, creature_guid: ObjectGuid) {
        self.drop_all_generators(creature_guid);

        self.generators.insert(
            MovementGeneratorType::Idle,
            Box::new(IdleMovementGenerator::new()),
        );

        self.active_type = MovementGeneratorType::Idle;
        self.moving = false;
        self.current_destination = None;
        self.needs_async_update = false;
        self.expire_list = None;
        self.pending_reset = false;
        self.flags = MotionMasterFlags::new();
    }

    /// Swap in a new default movement generator without interrupting the active one.
    ///
    /// `new_default` is the generator the caller's factory selected; `None` falls back to
    /// idle. The currently active generator is popped, everything else is cleared, and the
    /// active generator is restored on top unless it is already the new default.
    pub fn initialize_new_default(
        &mut self,
        creature_guid: ObjectGuid,
        current_pos: Position,
        new_default: Option<Box<dyn MovementGenerator>>,
        always_replace: bool,
    ) {
        if self.generators.is_empty() {
            self.initialize(creature_guid);
            return;
        }

        let new_default_type = new_default
            .as_ref()
            .map_or(MovementGeneratorType::Idle, |gen| gen.generator_type());

        // Already using the same motion type as default
        if !always_replace
            && self.generators.len() == 1
            && self.top_type() == Some(new_default_type)
        {
            return;
        }

        let Some((curr_type, mut curr)) = self.pop_top() else {
            return;
        };

        // Clear ALL other movement generators
        self.drop_all_generators(creature_guid);

        if always_replace || curr_type != new_default_type {
            let mut default_gen =
                new_default.unwrap_or_else(|| Box::new(IdleMovementGenerator::new()));
            default_gen.initialize(creature_guid, current_pos);
            self.generators.insert(new_default_type, default_gen);

            if curr_type != new_default_type {
                // Restore the previous current generator on top of the new default
                self.generators.insert(curr_type, curr);
            } else {
                // Same as the new default, so it can be retired
                curr.finalize(creature_guid);
                if !Self::is_static(curr_type) {
                    self.expire_list.get_or_insert_with(Vec::new).push(curr);
                }
            }
        } else {
            self.generators.insert(curr_type, curr);
        }

        self.update_active(creature_guid);
    }

    /// Add a movement generator
    pub fn add_generator(
        &mut self,
        generator: Box<dyn MovementGenerator>,
        creature_guid: ObjectGuid,
        current_pos: Position,
    ) {
        let gen_type = generator.generator_type();

        tracing::debug!(
            "[MOTION] Adding {:?} generator for {:?}",
            gen_type,
            creature_guid
        );

        // Remove existing generator of same type
        if let Some(mut old) = self.generators.remove(&gen_type) {
            old.finalize(creature_guid);
        }

        // Initialize and insert new generator
        let mut gen = generator;
        gen.initialize(creature_guid, current_pos);
        self.generators.insert(gen_type, gen);

        // Update active generator (highest priority)
        self.update_active(creature_guid);
    }

    /// Replace current motion with a new generator using MotionMaster mutation semantics.
    ///
    /// Chase, home and distract movement is dropped rather than parked beneath the new
    /// generator; anything else is interrupted and kept.
    pub fn mutate(
        &mut self,
        generator: Box<dyn MovementGenerator>,
        creature_guid: ObjectGuid,
        current_pos: Position,
    ) {
        let gen_type = generator.generator_type();

        if !self.generators.is_empty() {
            if matches!(
                self.top_type(),
                Some(
                    MovementGeneratorType::Chase
                        | MovementGeneratorType::Home
                        | MovementGeneratorType::Distract
                )
            ) {
                self.delayed_expire(creature_guid, false);
            }

            if let Some(top) = self.top_generator_mut() {
                top.interrupt(creature_guid);
            }
        }

        if let Some(mut old) = self.generators.remove(&gen_type) {
            old.finalize(creature_guid);
        }

        let mut generator = generator;
        generator.initialize(creature_guid, current_pos);
        self.generators.insert(gen_type, generator);

        self.update_active(creature_guid);
    }

    /// Remove a generator by type
    pub fn remove_generator(&mut self, gen_type: MovementGeneratorType, creature_guid: ObjectGuid) {
        if let Some(mut gen) = self.generators.remove(&gen_type) {
            gen.finalize(creature_guid);
        }

        self.update_active(creature_guid);
    }

    /// Clear all generators except idle
    pub fn clear(&mut self, creature_guid: ObjectGuid) {
        let types: Vec<_> = self
            .generators
            .keys()
            .filter(|t| **t != MovementGeneratorType::Idle)
            .copied()
            .collect();

        for gen_type in types {
            if let Some(mut gen) = self.generators.remove(&gen_type) {
                gen.finalize(creature_guid);
            }
        }

        self.active_type = MovementGeneratorType::Idle;
        self.moving = false;
        self.current_destination = None;
    }

    /// Start chasing a target
    pub fn chase(
        &mut self,
        target: ObjectGuid,
        creature_guid: ObjectGuid,
        current_pos: Position,
        creature_combat_reach: f32,
        run_speed: f32,
    ) {
        // Ignore the request if the target does not exist
        if target.is_empty() {
            return;
        }

        // Don't recreate if already chasing same target - the generator is rebuilt on
        // every call, which would reset chase state on each AI tick here.
        if let Some(gen) = self.generators.get_mut(&MovementGeneratorType::Chase) {
            if let Some(chase) = gen.as_any_mut().downcast_mut::<ChaseMovementGenerator>() {
                if chase.target == target {
                    return;
                }
            }
        }
        let generator = ChaseMovementGenerator::new(target, creature_combat_reach, run_speed);
        self.mutate(Box::new(generator), creature_guid, current_pos);
    }

    /// Start returning home.
    ///
    /// This is the uncharmed branch of the targeted-home flow. The LOST_CONTROL guard
    /// and the charmed branches (stay put, or follow the owner) need charm state the caller
    /// owns, so they route through [`Self::move_idle`] / [`Self::move_follow`] instead.
    pub fn return_home(
        &mut self,
        home_pos: Position,
        creature_guid: ObjectGuid,
        current_pos: Position,
        run_speed: f32,
    ) {
        // Clear(false): drop the stack down to the default generator
        self.clear(creature_guid);
        let generator = HomeMovementGenerator::new(home_pos, run_speed);
        self.mutate(Box::new(generator), creature_guid, current_pos);
    }

    /// Start random wandering around a position
    pub fn random_wander(
        &mut self,
        home_pos: Position,
        wander_distance: f32,
        creature_guid: ObjectGuid,
        current_pos: Position,
        walk_speed: f32,
    ) {
        let generator = RandomMovementGenerator::new(home_pos, wander_distance, walk_speed);
        self.mutate(Box::new(generator), creature_guid, current_pos);
    }

    /// Start random wandering using the MotionMaster command shape.
    pub fn move_random(
        &mut self,
        use_current_position: bool,
        home_pos: Position,
        creature_guid: ObjectGuid,
        current_pos: Position,
        wander_distance: f32,
        expire_time_ms: u32,
        walk_speed: f32,
    ) {
        let origin = if use_current_position {
            current_pos
        } else {
            home_pos
        };
        let generator = RandomMovementGenerator::new(origin, wander_distance, walk_speed)
            .with_expire_time(expire_time_ms);
        self.mutate(Box::new(generator), creature_guid, current_pos);
    }

    /// Start waypoint movement (patrol path)
    pub fn waypoint(
        &mut self,
        waypoints: Vec<Waypoint>,
        repeating: bool,
        creature_guid: ObjectGuid,
        current_pos: Position,
        walk_speed: f32,
    ) {
        let generator = WaypointMovementGenerator::new(waypoints, repeating, walk_speed);
        self.mutate(Box::new(generator), creature_guid, current_pos);
    }

    /// Start waypoint movement as the creature's default path.
    ///
    /// Unlike [`Self::move_waypoint`] this installs the path *beneath* the active
    /// generator, which keeps running until it finishes.
    pub fn move_waypoint_as_default(
        &mut self,
        waypoints: Vec<Waypoint>,
        creature_guid: ObjectGuid,
        current_pos: Position,
        walk_speed: f32,
    ) {
        if self.get_current_movement_generator_type() == MovementGeneratorType::Waypoint {
            tracing::error!(
                "[MOTION] {:?} attempted move_waypoint_as_default while already on a waypoint path",
                creature_guid
            );
            return;
        }

        let mut generator: Box<dyn MovementGenerator> =
            Box::new(WaypointMovementGenerator::new(waypoints, true, walk_speed));

        if self.generators.len() > 1 {
            // Eject the active generator, wipe the rest, then rebuild with the path as the
            // new default and the active generator back on top.
            let curr = self.pop_top();
            self.drop_all_generators(creature_guid);
            generator.initialize(creature_guid, current_pos);
            self.generators
                .insert(MovementGeneratorType::Waypoint, generator);
            if let Some((curr_type, curr)) = curr {
                self.generators.insert(curr_type, curr);
            }
        } else {
            self.drop_all_generators(creature_guid);
            generator.initialize(creature_guid, current_pos);
            self.generators
                .insert(MovementGeneratorType::Waypoint, generator);
        }

        self.update_active(creature_guid);
    }

    /// Start waypoint movement.
    pub fn move_waypoint(
        &mut self,
        waypoints: Vec<Waypoint>,
        creature_guid: ObjectGuid,
        current_pos: Position,
        walk_speed: f32,
    ) {
        if self.get_current_movement_generator_type() == MovementGeneratorType::Waypoint {
            tracing::error!(
                "[MOTION] {:?} attempted move_waypoint while already on a waypoint path",
                creature_guid
            );
            return;
        }

        self.waypoint(waypoints, true, creature_guid, current_pos, walk_speed);
    }

    /// Start cyclic waypoint movement.
    ///
    /// Cyclic paths have no dedicated generator type here, so they run as a repeating
    /// waypoint path and share its already-patrolling guard.
    pub fn move_cyclic_waypoint(
        &mut self,
        waypoints: Vec<Waypoint>,
        creature_guid: ObjectGuid,
        current_pos: Position,
        walk_speed: f32,
    ) {
        self.move_waypoint(waypoints, creature_guid, current_pos, walk_speed);
    }

    /// Start fleeing from a target
    pub fn flee(
        &mut self,
        flee_from: ObjectGuid,
        flee_time_ms: u32,
        creature_guid: ObjectGuid,
        current_pos: Position,
        run_speed: f32,
    ) {
        // Ignore the request if the enemy does not exist
        if flee_from.is_empty() {
            return;
        }

        let generator = FleeMovementGenerator::new(flee_from, flee_time_ms, run_speed);
        self.mutate(Box::new(generator), creature_guid, current_pos);
    }

    /// Start following a target at an offset.
    pub fn move_follow(
        &mut self,
        target: ObjectGuid,
        follow_distance: f32,
        follow_angle: f32,
        creature_guid: ObjectGuid,
        current_pos: Position,
        walk_speed: f32,
    ) {
        // The stack is dropped even when the target turns out to be invalid.
        self.clear(creature_guid);

        if target.is_empty() {
            return;
        }

        let generator =
            FollowMovementGenerator::new(target, follow_distance, follow_angle, walk_speed);
        self.mutate(Box::new(generator), creature_guid, current_pos);
    }

    /// Start a one-shot movement to a specific point.
    pub fn move_point(
        &mut self,
        id: u32,
        destination: Position,
        options: u32,
        speed: f32,
        final_orientation: f32,
        creature_guid: ObjectGuid,
        current_pos: Position,
    ) {
        let is_walking = (options & 0x1) != 0;
        let generator =
            PointMovementGenerator::new(id, destination, speed, is_walking, final_orientation);
        self.mutate(Box::new(generator), creature_guid, current_pos);
    }

    /// Start a one-shot point movement that should call for help on arrival.
    pub fn move_seek_assistance(
        &mut self,
        destination: Position,
        creature_guid: ObjectGuid,
        current_pos: Position,
        delay_ms: u32,
        speed: f32,
    ) {
        let generator = AssistanceMovementGenerator::new(0, destination, speed, delay_ms);
        self.mutate(Box::new(generator), creature_guid, current_pos);
    }

    /// Start the post-assistance distraction timer.
    pub fn move_seek_assistance_distract(&mut self, creature_guid: ObjectGuid, timer_ms: u32) {
        self.mutate(
            Box::new(AssistanceDistractMovementGenerator::new(timer_ms)),
            creature_guid,
            Position::default(),
        );
    }

    /// Pass a speed change notification to all generators.
    pub fn propagate_speed_change(&mut self) {
        for generator in self.generators.values_mut() {
            generator.unit_speed_changed();
        }
    }

    /// Update the next waypoint id on the active waypoint generator.
    pub fn set_next_waypoint(&mut self, point_id: u32) -> bool {
        for generator in self.generators.values_mut().rev() {
            if generator.generator_type() == MovementGeneratorType::Waypoint {
                if let Some(waypoint) = generator
                    .as_any_mut()
                    .downcast_mut::<WaypointMovementGenerator>()
                {
                    return waypoint.set_next_waypoint(point_id);
                }
            }
        }

        false
    }

    /// Get the last reached waypoint id from the active waypoint generator.
    pub fn get_last_reached_waypoint(&mut self) -> u32 {
        for generator in self.generators.values_mut().rev() {
            if generator.generator_type() == MovementGeneratorType::Waypoint {
                if let Some(waypoint) = generator
                    .as_any_mut()
                    .downcast_mut::<WaypointMovementGenerator>()
                {
                    return waypoint.get_last_reached_waypoint();
                }
            }
        }

        0
    }

    /// Return a human-readable waypoint summary for debugging.
    pub fn get_waypoint_path_information(&mut self) -> Option<String> {
        for generator in self.generators.values_mut().rev() {
            if generator.generator_type() == MovementGeneratorType::Waypoint {
                if let Some(waypoint) = generator
                    .as_any_mut()
                    .downcast_mut::<WaypointMovementGenerator>()
                {
                    return Some(format!(
                        "waypoints={}, last_reached={}",
                        waypoint.waypoint_count(),
                        waypoint.get_last_reached_waypoint()
                    ));
                }
            }
        }

        None
    }

    /// Start confused movement.
    pub fn move_confused(&mut self, creature_guid: ObjectGuid, current_pos: Position) {
        self.mutate(
            Box::new(ConfusedMovementGenerator::new()),
            creature_guid,
            current_pos,
        );
    }

    /// Taxi flight is player-only; creatures have no flight path generator.
    pub fn move_taxi_flight(&mut self, path: u32, pathnode: u32) {
        tracing::error!("[MOTION] creature attempted taxi flight (path {path} node {pathnode})");
    }

    /// Jump movement does not exist in 1.12 - the reference body is commented out for this core.
    pub fn move_jump(&mut self) {}

    /// Charge movement toward a target unit, used by SPELL_EFFECT_CHARGE.
    ///
    /// `delay` is a spell-batching arrival delay (ms); it is currently unmodelled.
    /// `trigger_auto_attack` flags the creature to begin attacking the target on arrival.
    /// `use_combat_reach` is preserved for parity with the reference signature but ignored here.
    pub fn move_charge(
        &mut self,
        target: ObjectGuid,
        _delay: u32,
        trigger_auto_attack: bool,
        _use_combat_reach: bool,
        creature_guid: ObjectGuid,
        current_pos: Position,
        run_speed: f32,
    ) {
        if target.is_empty() {
            return;
        }
        let generator = ChargeMovementGenerator::new(target, trigger_auto_attack, run_speed);
        self.mutate(Box::new(generator), creature_guid, current_pos);
    }

    /// Distance-based movement is not fully modeled yet in the Rust creature MotionMaster.
    pub fn move_distance(&mut self, _target: ObjectGuid, _distance: f32) -> bool {
        false
    }

    /// Start feared movement away from a target.
    pub fn fear(
        &mut self,
        fright: ObjectGuid,
        duration_ms: u32,
        flee_distance: f32,
        creature_guid: ObjectGuid,
        current_pos: Position,
        run_speed: f32,
    ) {
        // Ignore the request if the enemy does not exist
        if fright.is_empty() {
            return;
        }

        if duration_ms > 0 {
            self.mutate(
                Box::new(TimedFearMovementGenerator::new(
                    fright,
                    duration_ms,
                    flee_distance,
                    run_speed,
                )),
                creature_guid,
                current_pos,
            );
        } else {
            self.mutate(
                Box::new(
                    FearMovementGenerator::new(fright, flee_distance)
                        .with_timing(2000, false, run_speed),
                ),
                creature_guid,
                current_pos,
            );
        }
    }

    /// Stop all movement
    pub fn stop(&mut self, creature_guid: ObjectGuid) {
        self.clear(creature_guid);
    }

    /// Start a short distraction movement.
    pub fn move_distract(&mut self, creature_guid: ObjectGuid, timer_ms: u32) {
        self.mutate(
            Box::new(DistractMovementGenerator::new(timer_ms)),
            creature_guid,
            Position::default(),
        );
    }

    /// Start an assistance distraction movement.
    pub fn move_assistance_distract(&mut self, creature_guid: ObjectGuid, timer_ms: u32) {
        self.add_generator(
            Box::new(AssistanceDistractMovementGenerator::new(timer_ms)),
            creature_guid,
            Position::default(),
        );
    }

    /// Pop the generators a clean would retire: everything, or everything above the default.
    fn take_generators_for_clean(
        &mut self,
        all: bool,
    ) -> Vec<(MovementGeneratorType, Box<dyn MovementGenerator>)> {
        let mut retired = Vec::new();
        while if all {
            !self.generators.is_empty()
        } else {
            self.generators.len() > 1
        } {
            match self.pop_top() {
                Some(entry) => retired.push(entry),
                None => break,
            }
        }
        retired
    }

    /// Pop the chase/follow generators parked beneath an expiring generator.
    ///
    /// The guard also skips this when the expiring generator is a distancing one; that
    /// type has no equivalent here, so the guard is always satisfied.
    fn take_stored_targeted_generators(
        &mut self,
    ) -> Vec<(MovementGeneratorType, Box<dyn MovementGenerator>)> {
        let mut retired = Vec::new();
        while matches!(
            self.top_type(),
            Some(MovementGeneratorType::Chase | MovementGeneratorType::Follow)
        ) {
            match self.pop_top() {
                Some(entry) => retired.push(entry),
                None => break,
            }
        }
        retired
    }

    /// Clean motion generators immediately.
    ///
    /// Generators are finalized only after the stack has been rebuilt, because finalizing
    /// can push new movement (via movement-inform callbacks in the AI).
    pub fn direct_clean(&mut self, creature_guid: ObjectGuid, reset: bool, all: bool) {
        let mut retired = self.take_generators_for_clean(all);

        if !all && reset {
            if self.generators.is_empty() {
                self.initialize(creature_guid);
            }
            if let Some(top) = self.top_generator_mut() {
                top.reset(creature_guid);
            }
        }

        for (_, gen) in retired.iter_mut() {
            gen.finalize(creature_guid);
        }

        self.moving = false;
        self.current_destination = None;
        self.update_active(creature_guid);
    }

    /// Clean motion generators, deferring their disposal to the next update.
    pub fn delayed_clean(&mut self, creature_guid: ObjectGuid, reset: bool, all: bool) {
        self.pending_reset = reset;

        if self.generators.is_empty() || (!all && self.generators.len() == 1) {
            return;
        }

        let mut retired = self.take_generators_for_clean(all);
        for (_, gen) in retired.iter_mut() {
            gen.finalize(creature_guid);
        }

        let expire_list = self.expire_list.get_or_insert_with(Vec::new);
        expire_list.extend(
            retired
                .into_iter()
                .filter(|(gen_type, _)| !Self::is_static(*gen_type))
                .map(|(_, gen)| gen),
        );

        self.moving = false;
        self.current_destination = None;
        self.update_active(creature_guid);
    }

    /// Expire the current movement generator and optionally reset the one beneath it.
    pub fn direct_expire(&mut self, creature_guid: ObjectGuid, reset: bool) {
        if self.generators.len() <= 1 {
            return;
        }

        let Some((_, mut curr)) = self.pop_top() else {
            return;
        };

        let mut retired = self.take_stored_targeted_generators();
        for (_, gen) in retired.iter_mut() {
            gen.finalize(creature_guid);
        }

        // Remember the top before finalizing, since finalizing may push new movement.
        let now_top = self.top_type();
        curr.finalize(creature_guid);
        drop(curr);

        if self.generators.is_empty() {
            self.initialize(creature_guid);
        }

        // Don't reset a generator that finalization just pushed.
        if reset && self.top_type() == now_top {
            if let Some(top) = self.top_generator_mut() {
                top.reset(creature_guid);
            }
        }

        self.moving = false;
        self.current_destination = None;
        self.update_active(creature_guid);
    }

    /// Expire the current movement generator, deferring disposal to the next update.
    pub fn delayed_expire(&mut self, creature_guid: ObjectGuid, reset: bool) {
        self.pending_reset = reset;

        if self.generators.len() <= 1 {
            return;
        }

        let Some((curr_type, mut curr)) = self.pop_top() else {
            return;
        };

        let mut retired = self.take_stored_targeted_generators();
        for (_, gen) in retired.iter_mut() {
            gen.finalize(creature_guid);
        }
        curr.finalize(creature_guid);

        let expire_list = self.expire_list.get_or_insert_with(Vec::new);
        expire_list.extend(retired.into_iter().map(|(_, gen)| gen));
        if !Self::is_static(curr_type) {
            expire_list.push(curr);
        }

        self.moving = false;
        self.current_destination = None;
        self.update_active(creature_guid);
    }

    /// Make idle the active movement.
    ///
    /// The shared idle generator is pushed on top of the stack; idle is the lowest
    /// priority in this type-keyed model, so the equivalent is to drop everything above it.
    pub fn move_idle(&mut self, creature_guid: ObjectGuid) {
        if self.top_type() != Some(MovementGeneratorType::Idle) {
            self.drop_all_generators(creature_guid);
        }

        self.generators
            .entry(MovementGeneratorType::Idle)
            .or_insert_with(|| Box::new(IdleMovementGenerator::new()));

        self.active_type = MovementGeneratorType::Idle;
        self.moving = false;
        self.current_destination = None;
    }

    /// Update movement - called each tick
    ///
    /// Uses re-entrant protection to prevent nested updates
    pub fn update(
        &mut self,
        creature_guid: ObjectGuid,
        current_pos: Position,
        diff_ms: u32,
    ) -> Option<MovementUpdate> {
        self.update_motion(creature_guid, current_pos, diff_ms)
    }

    /// Update motion state for the current tick.
    pub fn update_motion(
        &mut self,
        creature_guid: ObjectGuid,
        current_pos: Position,
        diff_ms: u32,
    ) -> Option<MovementUpdate> {
        // Re-entrant protection - prevent nested updates
        if self.flags.contains(MotionMasterFlags::UPDATING) {
            tracing::warn!(
                "[MOTION] Re-entrant update detected for {:?}, skipping",
                creature_guid
            );
            return None;
        }

        // Check if movement is paused
        if self.flags.contains(MotionMasterFlags::PAUSED) {
            return None;
        }

        // Set updating flag
        self.flags.insert(MotionMasterFlags::UPDATING);

        // Get active generator and update it
        let update = if let Some(gen) = self.generators.get_mut(&self.active_type) {
            gen.update(creature_guid, diff_ms)
        } else {
            // No active generator, add idle
            self.add_generator(
                Box::new(IdleMovementGenerator::new()),
                creature_guid,
                current_pos,
            );
            MovementUpdate::Continue
        };

        // Handle update result
        match &update {
            MovementUpdate::Finished => {
                // Remove finished generator
                let gen_type = self.active_type;
                self.remove_generator(gen_type, creature_guid);
                self.moving = false;
                self.current_destination = None;

                // If no generators left, add idle
                if self.generators.is_empty() {
                    self.add_generator(
                        Box::new(IdleMovementGenerator::new()),
                        creature_guid,
                        current_pos,
                    );
                }
            }
            MovementUpdate::NewDestination { destination, .. } => {
                self.current_destination = Some(*destination);
                self.moving = true;
            }
            MovementUpdate::Arrived => {
                self.moving = false;
            }
            MovementUpdate::Continue => {}
        }

        // Clear updating flag
        self.flags.remove(MotionMasterFlags::UPDATING);

        // Dispose of anything a delayed clean/expire retired this tick, then honour the
        // reset it deferred.
        if let Some(expire_list) = self.expire_list.take() {
            drop(expire_list);

            if self.generators.is_empty() {
                self.initialize(creature_guid);
            }

            if self.pending_reset {
                if let Some(top) = self.top_generator_mut() {
                    top.reset(creature_guid);
                }
                self.pending_reset = false;
            }
        }

        Some(update)
    }

    /// Run the async update path. The current world model resolves movement synchronously,
    /// so this mirrors the synchronous update path while clearing the async request flag.
    pub fn update_motion_async(
        &mut self,
        creature_guid: ObjectGuid,
        current_pos: Position,
        diff_ms: u32,
    ) -> Option<MovementUpdate> {
        self.needs_async_update = false;
        self.update_motion(creature_guid, current_pos, diff_ms)
    }

    /// Called when creature reaches destination
    pub fn movement_complete(&mut self, creature_guid: ObjectGuid) -> Option<u32> {
        self.moving = false;
        self.current_destination = None;

        // Notify active generator based on type
        match self.active_type {
            MovementGeneratorType::Home => {
                self.remove_generator(MovementGeneratorType::Home, creature_guid);
            }
            MovementGeneratorType::Random => {
                // Notify random generator it arrived
                if let Some(gen) = self.generators.get_mut(&MovementGeneratorType::Random) {
                    if let Some(random) = gen.as_any_mut().downcast_mut::<RandomMovementGenerator>()
                    {
                        random.on_arrival();
                    }
                }
            }
            MovementGeneratorType::Waypoint => {
                // Notify waypoint generator it arrived at waypoint
                if let Some(gen) = self.generators.get_mut(&MovementGeneratorType::Waypoint) {
                    if let Some(waypoint) =
                        gen.as_any_mut().downcast_mut::<WaypointMovementGenerator>()
                    {
                        waypoint.on_arrival();
                    }
                }
            }
            MovementGeneratorType::Point => {
                let delay = if let Some(gen) =
                    self.generators.get_mut(&MovementGeneratorType::Point)
                {
                    if let Some(point) = gen.as_any_mut().downcast_mut::<PointMovementGenerator>() {
                        point.on_arrival();
                        point.assistance_delay_ms()
                    } else if let Some(point) = gen
                        .as_any_mut()
                        .downcast_mut::<AssistanceMovementGenerator>()
                    {
                        point.assistance_delay_ms()
                    } else {
                        None
                    }
                } else {
                    None
                };

                if delay.is_some() {
                    self.remove_generator(MovementGeneratorType::Point, creature_guid);
                }
                return delay;
            }
            MovementGeneratorType::Fleeing => {
                if let Some(gen) = self.generators.get_mut(&MovementGeneratorType::Fleeing) {
                    if let Some(fear) = gen
                        .as_any_mut()
                        .downcast_mut::<TimedFearMovementGenerator>()
                    {
                        fear.on_arrival();
                    } else if let Some(fear) =
                        gen.as_any_mut().downcast_mut::<FearMovementGenerator>()
                    {
                        fear.on_arrival();
                    } else if let Some(flee) =
                        gen.as_any_mut().downcast_mut::<FleeMovementGenerator>()
                    {
                        flee.on_arrival();
                    }
                }
            }
            MovementGeneratorType::Follow => {
                if let Some(gen) = self.generators.get_mut(&MovementGeneratorType::Follow) {
                    if let Some(follow) = gen.as_any_mut().downcast_mut::<FollowMovementGenerator>()
                    {
                        follow.on_arrival();
                    }
                }
            }
            MovementGeneratorType::Confused => {
                if let Some(gen) = self.generators.get_mut(&MovementGeneratorType::Confused) {
                    if let Some(confused) =
                        gen.as_any_mut().downcast_mut::<ConfusedMovementGenerator>()
                    {
                        confused.on_arrival();
                    }
                }
            }
            _ => {}
        }

        None
    }

    /// Update the active generator to highest priority
    fn update_active(&mut self, creature_guid: ObjectGuid) {
        // BTreeMap is ordered by key, last() gives highest
        if let Some((&gen_type, _)) = self.generators.iter().next_back() {
            if self.active_type != gen_type {
                tracing::debug!(
                    "[MOTION] Active generator changed: {:?} -> {:?} for {:?}",
                    self.active_type,
                    gen_type,
                    creature_guid
                );
                self.active_type = gen_type;
            }
        } else {
            self.active_type = MovementGeneratorType::Idle;
        }
    }

    /// Get mutable reference to a generator by type
    /// This is needed for updating target positions in ChaseMovementGenerator
    pub fn get_generator_mut(
        &mut self,
        gen_type: MovementGeneratorType,
    ) -> Option<&mut Box<dyn MovementGenerator>> {
        self.generators.get_mut(&gen_type)
    }
}

impl Drop for MotionMaster {
    fn drop(&mut self) {
        // Deallocate generators without finalizing them: finalization reaches back into the
        // owner, which is already going away.
        self.generators.clear();
        self.expire_list = None;
    }
}

impl Default for MotionMaster {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MotionMaster {
    fn clone(&self) -> Self {
        // MotionMaster cannot be truly cloned because it contains trait objects.
        // For creature cloning (e.g., for templates), we create a fresh MotionMaster.
        // The generators will be re-added as needed when the creature spawns.
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generator_type_name_matches_cpp_names() {
        let cases = [
            (MovementGeneratorType::Idle, "IDLE_MOTION_TYPE"),
            (MovementGeneratorType::Random, "RANDOM_MOTION_TYPE"),
            (MovementGeneratorType::Waypoint, "WAYPOINT_MOTION_TYPE"),
            (MovementGeneratorType::Follow, "FOLLOW_MOTION_TYPE"),
            (MovementGeneratorType::Distract, "DISTRACT_MOTION_TYPE"),
            (MovementGeneratorType::Point, "POINT_MOTION_TYPE"),
            (MovementGeneratorType::Confused, "CONFUSED_MOTION_TYPE"),
            (MovementGeneratorType::Chase, "CHASE_MOTION_TYPE"),
            (MovementGeneratorType::Fleeing, "FLEEING_MOTION_TYPE"),
            (MovementGeneratorType::Home, "HOME_MOTION_TYPE"),
            (MovementGeneratorType::Effect, "EFFECT_MOTION_TYPE"),
            (MovementGeneratorType::Taxi, "FLIGHT_MOTION_TYPE"),
            (MovementGeneratorType::Charge, "CHARGE_MOTION_TYPE"),
        ];

        for (generator_type, name) in cases {
            assert_eq!(
                MotionMaster::get_movement_generator_type_name(generator_type),
                name
            );
        }
    }

    #[test]
    fn new_motion_master_starts_with_idle_as_current_and_used_generator() {
        let motion_master = MotionMaster::new();

        assert_eq!(
            motion_master.get_current_movement_generator_type(),
            MovementGeneratorType::Idle
        );
        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![MovementGeneratorType::Idle]
        );
        assert!(motion_master.is_using_idle_or_default_movement());
        assert_eq!(motion_master.get_destination(), None);
        assert!(!motion_master.is_moving());
    }

    #[test]
    fn used_generator_list_reports_inserted_generator_types_in_order() {
        let mut motion_master = MotionMaster::new();
        let creature = ObjectGuid::from_raw(1);

        motion_master.move_point(
            7,
            Position {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                o: 4.0,
            },
            0,
            7.0,
            4.0,
            creature,
            Position::default(),
        );

        assert_eq!(
            motion_master.get_current_movement_generator_type(),
            MovementGeneratorType::Point
        );
        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![MovementGeneratorType::Idle, MovementGeneratorType::Point]
        );
        assert!(!motion_master.is_using_idle_or_default_movement());
    }

    fn creature() -> ObjectGuid {
        ObjectGuid::from_raw(1)
    }

    /// Idle (default) + Random + Chase, with Chase on top.
    fn stacked_motion_master() -> MotionMaster {
        let mut motion_master = MotionMaster::new();
        let guid = creature();

        motion_master.random_wander(Position::default(), 5.0, guid, Position::default(), 2.5);
        motion_master.chase(ObjectGuid::from_raw(2), guid, Position::default(), 1.0, 7.0);

        motion_master
    }

    #[test]
    fn direct_clean_keeps_the_default_generator_unless_all_is_requested() {
        let mut motion_master = stacked_motion_master();

        motion_master.direct_clean(creature(), true, false);

        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![MovementGeneratorType::Idle]
        );
        assert_eq!(
            motion_master.get_current_movement_generator_type(),
            MovementGeneratorType::Idle
        );
        assert!(!motion_master.is_moving());
    }

    #[test]
    fn direct_clean_with_all_reinstates_idle_only_through_initialize() {
        let mut motion_master = stacked_motion_master();

        motion_master.direct_clean(creature(), false, true);

        assert!(motion_master.generators.is_empty());
        assert_eq!(
            motion_master.get_current_movement_generator_type(),
            MovementGeneratorType::Idle
        );
    }

    #[test]
    fn delayed_clean_parks_generators_until_the_next_update() {
        let mut motion_master = stacked_motion_master();

        motion_master.delayed_clean(creature(), true, false);

        assert_eq!(motion_master.expire_list.as_ref().map(Vec::len), Some(2));
        assert!(motion_master.pending_reset);

        motion_master.update_motion(creature(), Position::default(), 100);

        assert!(motion_master.expire_list.is_none());
        assert!(!motion_master.pending_reset);
        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![MovementGeneratorType::Idle]
        );
    }

    #[test]
    fn delayed_clean_on_a_single_generator_only_records_the_reset_request() {
        let mut motion_master = MotionMaster::new();

        motion_master.delayed_clean(creature(), true, false);

        assert!(motion_master.expire_list.is_none());
        assert!(motion_master.pending_reset);
    }

    #[test]
    fn direct_expire_pops_only_the_top_generator() {
        let mut motion_master = stacked_motion_master();

        motion_master.direct_expire(creature(), true);

        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![MovementGeneratorType::Idle, MovementGeneratorType::Random]
        );
        assert_eq!(
            motion_master.get_current_movement_generator_type(),
            MovementGeneratorType::Random
        );
    }

    #[test]
    fn direct_expire_also_drops_targeted_generators_stored_underneath() {
        let mut motion_master = MotionMaster::new();
        let guid = creature();

        motion_master.chase(ObjectGuid::from_raw(2), guid, Position::default(), 1.0, 7.0);
        motion_master.fear(
            ObjectGuid::from_raw(3),
            5_000,
            20.0,
            guid,
            Position::default(),
            7.0,
        );

        // Fleeing sits above Chase, so expiring it takes the parked chase with it.
        motion_master.direct_expire(guid, true);

        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![MovementGeneratorType::Idle]
        );
    }

    #[test]
    fn expire_is_a_no_op_with_only_the_default_generator() {
        let mut motion_master = MotionMaster::new();

        motion_master.direct_expire(creature(), true);
        motion_master.delayed_expire(creature(), true);

        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![MovementGeneratorType::Idle]
        );
        assert!(motion_master.expire_list.is_none());
    }

    #[test]
    fn delayed_expire_defers_disposal_to_the_next_update() {
        let mut motion_master = stacked_motion_master();

        motion_master.delayed_expire(creature(), false);

        assert_eq!(motion_master.expire_list.as_ref().map(Vec::len), Some(1));
        assert!(!motion_master.pending_reset);
        assert_eq!(
            motion_master.get_current_movement_generator_type(),
            MovementGeneratorType::Random
        );

        motion_master.update_motion(creature(), Position::default(), 100);

        assert!(motion_master.expire_list.is_none());
    }

    #[test]
    fn initialize_new_default_on_an_empty_stack_falls_back_to_initialize() {
        let mut motion_master = MotionMaster::new();
        motion_master.direct_clean(creature(), false, true);
        assert!(motion_master.generators.is_empty());

        motion_master.initialize_new_default(creature(), Position::default(), None, false);

        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![MovementGeneratorType::Idle]
        );
    }

    #[test]
    fn initialize_new_default_keeps_a_stack_already_on_the_new_default() {
        let mut motion_master = MotionMaster::new();

        motion_master.initialize_new_default(
            creature(),
            Position::default(),
            Some(Box::new(IdleMovementGenerator::new())),
            false,
        );

        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![MovementGeneratorType::Idle]
        );
        assert!(motion_master.expire_list.is_none());
    }

    #[test]
    fn initialize_new_default_swaps_the_default_and_restores_the_active_generator() {
        let mut motion_master = stacked_motion_master();

        let new_default = RandomMovementGenerator::new(Position::default(), 3.0, 2.5);
        motion_master.initialize_new_default(
            creature(),
            Position::default(),
            Some(Box::new(new_default)),
            false,
        );

        // Random becomes the default, Chase stays on top, everything else is gone.
        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![MovementGeneratorType::Random, MovementGeneratorType::Chase]
        );
        assert_eq!(
            motion_master.get_current_movement_generator_type(),
            MovementGeneratorType::Chase
        );
    }

    #[test]
    fn initialize_new_default_retires_an_active_generator_equal_to_the_new_default() {
        let mut motion_master = MotionMaster::new();
        let guid = creature();
        motion_master.random_wander(Position::default(), 5.0, guid, Position::default(), 2.5);

        motion_master.initialize_new_default(
            guid,
            Position::default(),
            Some(Box::new(RandomMovementGenerator::new(
                Position::default(),
                3.0,
                2.5,
            ))),
            true,
        );

        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![MovementGeneratorType::Random]
        );
        assert_eq!(motion_master.expire_list.as_ref().map(Vec::len), Some(1));
    }

    fn waypoints() -> Vec<Waypoint> {
        vec![Waypoint {
            point_id: 1,
            position: Position::default(),
            wait_time: 0,
            wander_distance: 0.0,
            script_id: 0,
            orientation: None,
        }]
    }

    #[test]
    fn mutate_drops_a_home_generator_instead_of_parking_it() {
        let mut motion_master = MotionMaster::new();
        let guid = creature();
        motion_master.return_home(Position::default(), guid, Position::default(), 7.0);
        assert_eq!(
            motion_master.get_current_movement_generator_type(),
            MovementGeneratorType::Home
        );

        motion_master.random_wander(Position::default(), 5.0, guid, Position::default(), 2.5);

        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![MovementGeneratorType::Idle, MovementGeneratorType::Random]
        );
        assert_eq!(motion_master.expire_list.as_ref().map(Vec::len), Some(1));
    }

    #[test]
    fn mutate_drops_a_distract_generator_instead_of_parking_it() {
        let mut motion_master = MotionMaster::new();
        let guid = creature();
        motion_master.move_distract(guid, 3_000);

        motion_master.move_point(
            7,
            Position::default(),
            0,
            7.0,
            0.0,
            guid,
            Position::default(),
        );

        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![MovementGeneratorType::Idle, MovementGeneratorType::Point]
        );
    }

    #[test]
    fn mutate_parks_generators_that_are_not_chase_home_or_distract() {
        let mut motion_master = MotionMaster::new();
        let guid = creature();
        motion_master.random_wander(Position::default(), 5.0, guid, Position::default(), 2.5);

        motion_master.move_confused(guid, Position::default());

        // Random survives beneath the confused movement rather than being cleared.
        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![
                MovementGeneratorType::Idle,
                MovementGeneratorType::Random,
                MovementGeneratorType::Confused
            ]
        );
        assert!(motion_master.expire_list.is_none());
    }

    #[test]
    fn move_commands_ignore_a_missing_target() {
        let mut motion_master = MotionMaster::new();
        let guid = creature();

        motion_master.chase(ObjectGuid::default(), guid, Position::default(), 1.0, 7.0);
        motion_master.flee(ObjectGuid::default(), 5_000, guid, Position::default(), 7.0);
        motion_master.fear(
            ObjectGuid::default(),
            5_000,
            20.0,
            guid,
            Position::default(),
            7.0,
        );

        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![MovementGeneratorType::Idle]
        );
    }

    #[test]
    fn move_charge_ignores_an_empty_target_and_installs_charge_for_a_unit() {
        let mut motion_master = MotionMaster::new();
        let guid = creature();

        motion_master.move_charge(
            ObjectGuid::default(),
            0,
            false,
            true,
            guid,
            Position::default(),
            7.0,
        );
        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![MovementGeneratorType::Idle]
        );

        motion_master.move_charge(
            ObjectGuid::from_raw(2),
            0,
            true,
            true,
            guid,
            Position::default(),
            7.0,
        );

        assert_eq!(
            motion_master.get_current_movement_generator_type(),
            MovementGeneratorType::Charge
        );
        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![MovementGeneratorType::Idle, MovementGeneratorType::Charge]
        );
    }

    #[test]
    fn move_follow_clears_the_stack_even_when_the_target_is_missing() {
        let mut motion_master = stacked_motion_master();

        motion_master.move_follow(
            ObjectGuid::default(),
            2.0,
            0.0,
            creature(),
            Position::default(),
            2.5,
        );

        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![MovementGeneratorType::Idle]
        );
    }

    #[test]
    fn move_waypoint_refuses_to_restart_an_active_patrol() {
        let mut motion_master = MotionMaster::new();
        let guid = creature();
        motion_master.move_waypoint(waypoints(), guid, Position::default(), 2.5);

        let path = motion_master.get_waypoint_path_information();
        assert_eq!(path.as_deref(), Some("waypoints=1, last_reached=0"));

        // A second call while the patrol is active is rejected, leaving the path in place.
        motion_master.move_waypoint(Vec::new(), guid, Position::default(), 2.5);

        assert_eq!(
            motion_master.get_current_movement_generator_type(),
            MovementGeneratorType::Waypoint
        );
        assert_eq!(motion_master.get_waypoint_path_information(), path);
    }

    #[test]
    fn move_waypoint_as_default_installs_the_path_beneath_the_active_generator() {
        let mut motion_master = MotionMaster::new();
        let guid = creature();
        motion_master.chase(ObjectGuid::from_raw(2), guid, Position::default(), 1.0, 7.0);

        motion_master.move_waypoint_as_default(waypoints(), guid, Position::default(), 2.5);

        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![
                MovementGeneratorType::Waypoint,
                MovementGeneratorType::Chase
            ]
        );
        assert_eq!(
            motion_master.get_current_movement_generator_type(),
            MovementGeneratorType::Chase
        );
    }

    #[test]
    fn move_idle_drops_everything_above_the_idle_generator() {
        let mut motion_master = stacked_motion_master();

        motion_master.move_idle(creature());

        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![MovementGeneratorType::Idle]
        );
        assert_eq!(
            motion_master.get_current_movement_generator_type(),
            MovementGeneratorType::Idle
        );
        assert!(!motion_master.is_moving());
    }

    #[test]
    fn add_pause_time_extends_random_and_waypoint_movement_only() {
        let guid = creature();

        let mut random = MotionMaster::new();
        random.random_wander(Position::default(), 5.0, guid, Position::default(), 2.5);
        assert!(random.add_pause_time(60_000));

        let mut patrol = MotionMaster::new();
        patrol.move_waypoint(waypoints(), guid, Position::default(), 2.5);
        assert!(patrol.add_pause_time(60_000));

        // Anything else - here a chase - cannot be paused this way.
        let mut chasing = MotionMaster::new();
        chasing.chase(ObjectGuid::from_raw(2), guid, Position::default(), 1.0, 7.0);
        assert!(!chasing.add_pause_time(60_000));

        // Idle only, nothing to pause.
        assert!(!MotionMaster::new().add_pause_time(60_000));
    }

    #[test]
    fn paused_random_movement_waits_out_the_added_time() {
        let guid = creature();
        let mut motion_master = MotionMaster::new();
        motion_master.random_wander(Position::default(), 5.0, guid, Position::default(), 2.5);

        motion_master.add_pause_time(60_000);

        // The initial 1s wander delay is replaced by the much longer pause.
        assert!(matches!(
            motion_master.update_motion(guid, Position::default(), 1_000),
            Some(MovementUpdate::Continue)
        ));
    }

    #[test]
    fn clear_type_removes_just_that_generator() {
        let mut motion_master = stacked_motion_master();

        motion_master.clear_type(MovementGeneratorType::Chase, creature());

        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![MovementGeneratorType::Idle, MovementGeneratorType::Random]
        );
        assert_eq!(
            motion_master.get_current_movement_generator_type(),
            MovementGeneratorType::Random
        );
    }

    #[test]
    fn set_next_waypoint_and_last_reached_report_from_the_waypoint_generator() {
        let guid = creature();
        let mut motion_master = MotionMaster::new();

        // No waypoint generator on the stack yet.
        assert!(!motion_master.set_next_waypoint(1));
        assert_eq!(motion_master.get_last_reached_waypoint(), 0);
        assert_eq!(motion_master.get_waypoint_path_information(), None);

        motion_master.move_waypoint(waypoints(), guid, Position::default(), 2.5);

        assert!(motion_master.set_next_waypoint(1));
        assert!(!motion_master.set_next_waypoint(99));
        assert_eq!(motion_master.get_last_reached_waypoint(), 0);
    }

    #[test]
    fn propagate_speed_change_and_final_distance_reach_the_stack_without_panicking() {
        let guid = creature();
        let mut motion_master = stacked_motion_master();

        motion_master.propagate_speed_change();
        motion_master.update_final_distance_to_target(12.5);

        assert_eq!(
            motion_master.get_current_movement_generator_type(),
            MovementGeneratorType::Chase
        );
    }

    #[test]
    fn initialize_rebuilds_the_stack_and_clears_pending_state() {
        let mut motion_master = stacked_motion_master();
        motion_master.delayed_clean(creature(), true, false);
        motion_master.flags.insert(MotionMasterFlags::PAUSED);

        motion_master.initialize(creature());

        assert_eq!(
            motion_master.get_used_movement_generators_list(),
            vec![MovementGeneratorType::Idle]
        );
        assert!(motion_master.expire_list.is_none());
        assert!(!motion_master.pending_reset);
        assert!(!motion_master.flags.contains(MotionMasterFlags::PAUSED));
        assert!(!motion_master.is_moving());
        assert_eq!(motion_master.get_destination(), None);
    }
}
