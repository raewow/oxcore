//! MotionMaster - manages movement generators for a creature

use super::generator::{MovementGenerator, MovementUpdate};
use super::generators::{
    AssistanceDistractMovementGenerator, AssistanceMovementGenerator, ChaseMovementGenerator,
    ConfusedMovementGenerator, DistractMovementGenerator, FearMovementGenerator,
    FleeMovementGenerator, FollowMovementGenerator, HomeMovementGenerator, IdleMovementGenerator,
    PointMovementGenerator, RandomMovementGenerator, TimedFearMovementGenerator, Waypoint,
    WaypointMovementGenerator,
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

        (current_type < MovementGeneratorType::Chase || current_type == MovementGeneratorType::Waypoint)
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
    pub fn update_final_distance_to_target(&mut self, _distance: f32) {}

    /// Remove all generators of the specified type.
    pub fn clear_type(&mut self, move_type: MovementGeneratorType, creature_guid: ObjectGuid) {
        let types: Vec<_> = self
            .generators
            .iter()
            .filter_map(|(ty, _)| if *ty == move_type { Some(*ty) } else { None })
            .collect();

        for ty in types {
            self.remove_generator(ty, creature_guid);
        }
    }

    /// Reinitialize the active patrol movement if one exists.
    pub fn reinitialize_patrol_movement(&mut self, creature_guid: ObjectGuid) {
        if let Some(generator) = self.generators.get_mut(&MovementGeneratorType::Waypoint) {
            generator.reset(creature_guid);
        }
    }

    pub fn new() -> Self {
        let mut mm = Self {
            generators: BTreeMap::new(),
            active_type: MovementGeneratorType::Idle,
            current_destination: None,
            moving: false,
            needs_async_update: false,
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

    /// Initialize motion state and ensure an idle generator exists.
    pub fn initialize(&mut self, creature_guid: ObjectGuid) {
        self.clear(creature_guid);

        if self.generators.is_empty() {
            self.generators.insert(
                MovementGeneratorType::Idle,
                Box::new(IdleMovementGenerator::new()),
            );
        }

        self.active_type = MovementGeneratorType::Idle;
        self.moving = false;
        self.current_destination = None;
        self.needs_async_update = false;
        self.flags = MotionMasterFlags::new();
    }

    /// Re-evaluate the default motion generator without disturbing the active one.
    pub fn initialize_new_default(&mut self, creature_guid: ObjectGuid, always_replace: bool) {
        if always_replace {
            self.initialize(creature_guid);
        } else {
            self.update_active(creature_guid);
        }
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
    pub fn mutate(
        &mut self,
        generator: Box<dyn MovementGenerator>,
        creature_guid: ObjectGuid,
        current_pos: Position,
    ) {
        self.add_generator(generator, creature_guid, current_pos);
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
        // Don't recreate if already chasing same target
        if let Some(gen) = self.generators.get_mut(&MovementGeneratorType::Chase) {
            if let Some(chase) = gen.as_any_mut().downcast_mut::<ChaseMovementGenerator>() {
                if chase.target == target {
                    return;
                }
            }
        }
        let generator = ChaseMovementGenerator::new(target, creature_combat_reach, run_speed);
        self.add_generator(Box::new(generator), creature_guid, current_pos);
    }

    /// Start returning home
    pub fn return_home(
        &mut self,
        home_pos: Position,
        creature_guid: ObjectGuid,
        current_pos: Position,
        run_speed: f32,
    ) {
        // Match the C++ targeted-home flow by dropping any stale movement stack
        // before we push the home generator.
        self.clear(creature_guid);
        let generator = HomeMovementGenerator::new(home_pos, run_speed);
        self.add_generator(Box::new(generator), creature_guid, current_pos);
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
        self.add_generator(Box::new(generator), creature_guid, current_pos);
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
        let origin = if use_current_position { current_pos } else { home_pos };
        let generator = RandomMovementGenerator::new(origin, wander_distance, walk_speed)
            .with_expire_time(expire_time_ms);
        self.add_generator(Box::new(generator), creature_guid, current_pos);
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
        self.add_generator(Box::new(generator), creature_guid, current_pos);
    }

    /// Start waypoint movement as the creature's default path.
    pub fn move_waypoint_as_default(
        &mut self,
        waypoints: Vec<Waypoint>,
        creature_guid: ObjectGuid,
        current_pos: Position,
        walk_speed: f32,
    ) {
        self.clear(creature_guid);
        self.waypoint(waypoints, true, creature_guid, current_pos, walk_speed);
    }

    /// Start waypoint movement.
    pub fn move_waypoint(
        &mut self,
        waypoints: Vec<Waypoint>,
        creature_guid: ObjectGuid,
        current_pos: Position,
        walk_speed: f32,
    ) {
        self.waypoint(waypoints, true, creature_guid, current_pos, walk_speed);
    }

    /// Start cyclic waypoint movement.
    pub fn move_cyclic_waypoint(
        &mut self,
        waypoints: Vec<Waypoint>,
        creature_guid: ObjectGuid,
        current_pos: Position,
        walk_speed: f32,
    ) {
        self.waypoint(waypoints, true, creature_guid, current_pos, walk_speed);
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
        let generator = FleeMovementGenerator::new(flee_from, flee_time_ms, run_speed);
        self.add_generator(Box::new(generator), creature_guid, current_pos);
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
        self.clear(creature_guid);
        let generator = FollowMovementGenerator::new(target, follow_distance, follow_angle, walk_speed);
        self.add_generator(Box::new(generator), creature_guid, current_pos);
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
        let generator = PointMovementGenerator::new(id, destination, speed, is_walking, final_orientation);
        self.add_generator(Box::new(generator), creature_guid, current_pos);
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
        self.add_generator(Box::new(generator), creature_guid, current_pos);
    }

    /// Start the post-assistance distraction timer.
    pub fn move_seek_assistance_distract(&mut self, creature_guid: ObjectGuid, timer_ms: u32) {
        self.add_generator(
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
                if let Some(waypoint) = generator.as_any_mut().downcast_mut::<WaypointMovementGenerator>() {
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
                if let Some(waypoint) = generator.as_any_mut().downcast_mut::<WaypointMovementGenerator>() {
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
                if let Some(waypoint) = generator.as_any_mut().downcast_mut::<WaypointMovementGenerator>() {
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
    pub fn move_confused(
        &mut self,
        creature_guid: ObjectGuid,
        current_pos: Position,
    ) {
        self.clear(creature_guid);
        self.add_generator(
            Box::new(ConfusedMovementGenerator::new()),
            creature_guid,
            current_pos,
        );
    }

    /// Taxi flight is handled on the player movement side in this Rust port.
    pub fn move_taxi_flight(&mut self, _path: u32, _pathnode: u32) {}

    /// Charge movement is not fully modeled yet in the Rust creature MotionMaster.
    pub fn move_charge(
        &mut self,
        _target: ObjectGuid,
        _delay: u32,
        _trigger_auto_attack: bool,
        _use_combat_reach: bool,
    ) {
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
        if duration_ms > 0 {
            self.add_generator(
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
            self.add_generator(
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
        self.add_generator(
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

    /// Clean motion generators immediately.
    pub fn direct_clean(&mut self, creature_guid: ObjectGuid, reset: bool, all: bool) {
        if all {
            self.clear(creature_guid);
        } else {
            self.clear(creature_guid);
            if reset {
                self.initialize(creature_guid);
            }
        }
    }

    /// Deferred clean in the current model maps to the immediate clean path.
    pub fn delayed_clean(&mut self, creature_guid: ObjectGuid, reset: bool, all: bool) {
        self.direct_clean(creature_guid, reset, all);
    }

    /// Expire the current movement generator and optionally reset.
    pub fn direct_expire(&mut self, creature_guid: ObjectGuid, reset: bool) {
        self.clear(creature_guid);
        if reset {
            self.initialize(creature_guid);
        }
    }

    /// Deferred expire in the current model maps to the immediate expire path.
    pub fn delayed_expire(&mut self, creature_guid: ObjectGuid, reset: bool) {
        self.direct_expire(creature_guid, reset);
    }

    /// Ensure the idle generator is the active fallback.
    pub fn move_idle(&mut self) {
        if self.generators.is_empty() {
            self.generators.insert(
                MovementGeneratorType::Idle,
                Box::new(IdleMovementGenerator::new()),
            );
        }

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
                let delay = if let Some(gen) = self.generators.get_mut(&MovementGeneratorType::Point) {
                    if let Some(point) = gen.as_any_mut().downcast_mut::<PointMovementGenerator>() {
                        point.on_arrival();
                        point.assistance_delay_ms()
                    } else if let Some(point) = gen.as_any_mut().downcast_mut::<AssistanceMovementGenerator>() {
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
                    if let Some(fear) = gen.as_any_mut().downcast_mut::<TimedFearMovementGenerator>() {
                        fear.on_arrival();
                    } else if let Some(fear) = gen.as_any_mut().downcast_mut::<FearMovementGenerator>() {
                        fear.on_arrival();
                    } else if let Some(flee) = gen.as_any_mut().downcast_mut::<FleeMovementGenerator>() {
                        flee.on_arrival();
                    }
                }
            }
            MovementGeneratorType::Follow => {
                if let Some(gen) = self.generators.get_mut(&MovementGeneratorType::Follow) {
                    if let Some(follow) = gen.as_any_mut().downcast_mut::<FollowMovementGenerator>() {
                        follow.on_arrival();
                    }
                }
            }
            MovementGeneratorType::Confused => {
                if let Some(gen) = self.generators.get_mut(&MovementGeneratorType::Confused) {
                    if let Some(confused) = gen.as_any_mut().downcast_mut::<ConfusedMovementGenerator>() {
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
        self.generators.clear();
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
