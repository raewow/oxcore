//! MotionMaster - manages movement generators for a creature

use super::generator::{MovementGenerator, MovementUpdate};
use super::generators::{
    ChaseMovementGenerator, FleeMovementGenerator, HomeMovementGenerator, IdleMovementGenerator,
    RandomMovementGenerator, Waypoint, WaypointMovementGenerator,
};
use super::types::MovementGeneratorType;
use crate::shared::protocol::{ObjectGuid, Position};
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
    /// State flags (updating, paused, etc.)
    flags: MotionMasterFlags,
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
    pub fn new() -> Self {
        let mut mm = Self {
            generators: BTreeMap::new(),
            active_type: MovementGeneratorType::Idle,
            current_destination: None,
            moving: false,
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

    /// Stop all movement
    pub fn stop(&mut self, creature_guid: ObjectGuid) {
        self.clear(creature_guid);
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

    /// Called when creature reaches destination
    pub fn movement_complete(&mut self, creature_guid: ObjectGuid) {
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
                    if let Some(random) = gen.as_any_mut().downcast_mut::<RandomMovementGenerator>() {
                        random.on_arrival();
                    }
                }
            }
            MovementGeneratorType::Waypoint => {
                // Notify waypoint generator it arrived at waypoint
                if let Some(gen) = self.generators.get_mut(&MovementGeneratorType::Waypoint) {
                    if let Some(waypoint) = gen.as_any_mut().downcast_mut::<WaypointMovementGenerator>() {
                        waypoint.on_arrival();
                    }
                }
            }
            _ => {}
        }
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
