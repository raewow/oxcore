//! AI Types - State machine, events, and actions
//!
//! This module defines the core AI types:
//! - AIState: The 7-state AI state machine
//! - AIType: Which AI behavior to use (Basic, Passive, Critter, etc.)
//! - AIEvent: Events that trigger AI decisions
//! - AIAction: Actions that AI decisions produce
//!
//! Following world patterns: Pure data, no side effects.

use crate::shared::protocol::{ObjectGuid, Position};
use std::collections::HashMap;

/// AI state machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AIState {
    /// Not doing anything, waiting for stimulus
    #[default]
    Idle,
    /// In active combat with targets
    Combat,
    /// Evading - leaving combat and preparing to return home
    Evading,
    /// Returning to spawn position
    Returning,
    /// Fleeing from a threat (CritterAI, fear effects)
    Fleeing,
    /// Following another unit (PetAI)
    Following,
    /// Creature is dead
    Dead,
}

/// AI type enum - selects which pure decision function to use
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AIType {
    /// No AI - does nothing (used for possessable creatures)
    Null,
    /// Passive - only returns to spawn if too far, never fights back
    Passive,
    /// Critter - flees when damaged, random wandering
    Critter,
    /// Basic - standard combat AI with threat, spells, melee
    /// This is the default AI for most hostile creatures
    #[default]
    Basic,
    /// Totem - stationary spell caster, doesn't move
    Totem,
    /// Pet - follows owner, attacks owner's target, can be commanded
    Pet,
    /// Guard - like basic but calls for help, summons guards
    Guard,
    /// Event - database-driven scripted AI from creature_ai_scripts table
    Event,
    /// Lua - Lua-scripted AI from /scripts/ directory
    Lua,
}

impl AIType {
    /// Determine AI type from creature template
    /// Phase 4: Simple mapping based on creature type and flags
    pub fn from_creature_template(creature_type: u32, _static_flags: u32) -> Self {
        match creature_type {
            8 => AIType::Critter, // CRITTER type
            // TODO: Add more mappings based on flags and other criteria
            _ => AIType::Basic,
        }
    }
}

/// Reason why combat ended
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatEndReason {
    /// All targets are dead
    AllTargetsDead,
    /// Creature evaded
    Evaded,
    /// Creature died
    CreatureDied,
    /// All targets out of range/unreachable
    TargetsOutOfRange,
}

/// Events that trigger AI decisions
///
/// Events replace the old callback system (on_damage_taken, on_enter_combat, etc.)
/// that was called while holding locks. Instead, events are queued and processed
/// at the start of AI update when no locks are held.
#[derive(Debug, Clone)]
pub enum AIEvent {
    // =========================================================================
    // DAMAGE EVENTS
    // =========================================================================
    /// Damage was taken from an attacker
    DamageTaken {
        attacker_guid: ObjectGuid,
        damage: u32,
        spell_id: Option<u32>,
        school: u8,
    },

    /// Damage was dealt to a target
    DamageDealt {
        target_guid: ObjectGuid,
        damage: u32,
        spell_id: Option<u32>,
    },

    /// Healing was received (generates assist threat for healer)
    HealingReceived {
        healer_guid: ObjectGuid,
        amount: u32,
        spell_id: Option<u32>,
    },

    // =========================================================================
    // SPELL EVENTS
    // =========================================================================
    /// A spell hit this creature
    SpellHit {
        caster_guid: ObjectGuid,
        spell_id: u32,
    },

    /// A spell we cast hit a target
    SpellHitTarget {
        target_guid: ObjectGuid,
        spell_id: u32,
    },

    /// Spell cast was interrupted
    SpellInterrupted { spell_id: u32 },

    // =========================================================================
    // DETECTION EVENTS
    // =========================================================================
    /// A unit entered detection/aggro range
    UnitInRange {
        unit_guid: ObjectGuid,
        distance: f32,
        is_hostile: bool,
        is_player: bool,
    },

    /// Line of sight to a unit was established
    /// Used for proximity aggro in BasicAI
    UnitInLineOfSight {
        unit_guid: ObjectGuid,
        is_hostile: bool,
    },

    // =========================================================================
    // COMBAT STATE EVENTS
    // =========================================================================
    /// Combat has started (first threat was added)
    CombatStarted { initial_aggressor: ObjectGuid },

    /// Combat has ended
    CombatEnded { reason: CombatEndReason },

    /// A target was killed
    TargetKilled { victim_guid: ObjectGuid },

    // =========================================================================
    // MOVEMENT EVENTS
    // =========================================================================
    /// Movement to a point was completed
    MovementComplete { point_id: u32 },

    /// Reached home position after evade
    ReachedHome,

    // =========================================================================
    // TIMER EVENTS
    // =========================================================================
    /// A timer expired (for custom scripting)
    TimerExpired { timer_id: u32 },

    /// Regular update tick (called each AI update interval)
    UpdateTick { diff_ms: u32 },

    // =========================================================================
    // LIFECYCLE EVENTS
    // =========================================================================
    /// Creature just spawned
    Spawned,

    /// Creature just died
    Died { killer_guid: Option<ObjectGuid> },

    /// Creature just respawned
    Respawned,

    // =========================================================================
    // GROUP/ASSISTANCE EVENTS
    // =========================================================================
    /// Another creature called for help
    AssistanceRequested {
        caller_guid: ObjectGuid,
        target_guid: ObjectGuid,
    },

    // =========================================================================
    // SUMMONING EVENTS (for NPCs that summon)
    // =========================================================================
    /// This creature summoned another creature
    SummonedCreature {
        summoned_guid: ObjectGuid,
        entry: u32,
    },

    /// A summoned creature died
    SummonedCreatureDied { summoned_guid: ObjectGuid },

    /// A summoned creature despawned
    SummonedCreatureDespawned { summoned_guid: ObjectGuid },
}

/// Actions that AI decisions can produce
///
/// These are PURE DATA - no side effects when created.
/// The AIActionExecutor applies these to the game world with proper locking.
#[derive(Debug, Clone)]
pub enum AIAction {
    // =========================================================================
    // MOVEMENT ACTIONS
    // =========================================================================
    /// Move to a specific position
    MoveTo {
        position: Position,
        movement_type: MovementType,
    },

    /// Move toward a target (chase)
    MoveToTarget {
        target_guid: ObjectGuid,
        /// Stop when within this distance
        min_distance: f32,
    },

    /// Stop all movement
    StopMovement,

    /// Return to spawn point
    ReturnToSpawn,

    /// Flee from a target
    FleeFrom {
        flee_from_guid: ObjectGuid,
        /// How far to flee
        distance: f32,
        /// How long to flee in milliseconds
        duration_ms: u32,
    },

    /// Face a specific direction
    FaceTarget { target_guid: ObjectGuid },

    /// Random wandering movement
    RandomMovement {
        /// Maximum distance from spawn to wander
        wander_distance: f32,
    },

    // =========================================================================
    // COMBAT ACTIONS
    // =========================================================================
    /// Set the current attack target
    SetAttackTarget { target_guid: ObjectGuid },

    /// Clear the current attack target
    ClearAttackTarget,

    /// Perform a melee attack on target
    MeleeAttack { target_guid: ObjectGuid },

    /// Cast a spell
    CastSpell {
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
        target_position: Option<Position>,
        /// Whether this is a triggered spell (no cast time, no cost)
        triggered: bool,
    },

    // =========================================================================
    // COMBAT STATE CHANGES
    // =========================================================================
    /// Enter combat with a target
    EnterCombat { target_guid: ObjectGuid },

    /// Enter evade mode (leave combat, prepare to return home)
    EnterEvadeMode,

    /// Leave combat
    LeaveCombat,

    /// Modify threat on a target
    ModifyThreat {
        target_guid: ObjectGuid,
        amount: f32,
        /// If true, amount is a percentage modifier
        is_percent: bool,
    },

    /// Clear the entire threat list
    ClearThreatList,

    /// Add threat to a target (typically from damage taken event)
    AddThreat {
        target_guid: ObjectGuid,
        amount: f32,
    },

    // =========================================================================
    // AI CONFIGURATION
    // =========================================================================
    /// Enable or disable melee attacks
    SetMeleeAttack { enabled: bool },

    /// Enable or disable combat movement (chasing)
    SetCombatMovement { enabled: bool },

    /// Set react state (passive/defensive/aggressive)
    SetReactState { react_state: ReactState },

    // =========================================================================
    // ASSISTANCE
    // =========================================================================
    /// Call for help (alert nearby creatures)
    CallForHelp { radius: f32 },

    /// Summon guards (GuardAI)
    SummonGuards { guard_entry: u32, count: u32 },

    // =========================================================================
    // CHAT / EMOTE ACTIONS
    // =========================================================================
    /// Say text (normal chat bubble, 25 yard range)
    Say { text: String },

    /// Yell text (zone-wide)
    Yell { text: String },

    /// Play an emote animation
    PlayEmote { emote_id: u32 },

    /// Text emote (e.g. "%s laughs at you")
    TextEmote { text: String },

    /// Play a sound
    PlaySound { sound_id: u32, zone_wide: bool },

    // =========================================================================
    // SPECIAL
    // =========================================================================
    // =========================================================================
    // CREATURE STATE (Lua scripting)
    // =========================================================================
    /// Interrupt current spell cast
    InterruptSpell,

    /// Kill this creature
    KillSelf,

    /// Change creature faction
    SetFaction { faction_id: u32 },

    /// Set immunity flags
    SetImmune { physical: bool, spell: bool },

    /// Root/unroot creature
    SetRoot { rooted: bool },

    /// Set health to a percentage of max
    SetHealthPercent { percent: f32 },

    /// Change display model
    Morph { display_id: u32 },

    /// Reset display model to default
    Demorph,

    // =========================================================================
    // SPAWNING (Lua scripting)
    // =========================================================================
    /// Spawn a creature
    SpawnCreature {
        entry: u32,
        position: Position,
        summon_type: u8,
        duration_ms: u32,
    },

    /// Despawn a specific creature
    DespawnCreature { guid: ObjectGuid },

    /// Despawn all creatures with entry
    DespawnCreaturesByEntry { entry: u32 },

    /// Spawn a game object
    SpawnGameObject {
        entry: u32,
        position: Position,
        duration_secs: u32,
    },

    /// Respawn a game object
    RespawnGameObject {
        guid: ObjectGuid,
        duration_secs: u32,
    },

    // =========================================================================
    // INSTANCE (Lua scripting)
    // =========================================================================
    /// Set instance encounter data
    SetInstanceData { data_id: u32, value: u32 },

    /// Store a GUID in instance data
    SetInstanceGuid { data_id: u32, guid: ObjectGuid },

    /// Open a door by GUID
    OpenDoor { guid: ObjectGuid },

    /// Open a door by data ID
    OpenDoorByData { data_id: u32 },

    /// Close a door by GUID
    CloseDoor { guid: ObjectGuid },

    /// Close a door by data ID
    CloseDoorByData { data_id: u32 },

    // =========================================================================
    // PHASE 5: NEW ACTIONS
    // =========================================================================
    /// Remove an aura from the creature
    RemoveAura { spell_id: u32 },

    /// Set a unit flag on the creature
    SetUnitFlag { flag: u32 },

    /// Remove a unit flag from the creature
    RemoveUnitFlag { flag: u32 },

    /// Set combat with all players in the zone/instance
    SetCombatWithZone,

    /// Play script text (lookup from DB)
    ScriptText { text_id: i32 },

    /// Set stand/animation state (0=stand, 1=sit, 3=sleep, 4=kneel, 7=dead)
    SetStandState { state: u8 },

    /// Set dynamic flags (UNIT_DYNFLAG_DEAD = 4, etc.)
    SetDynFlag { flag: u32 },

    /// Remove dynamic flags
    RemoveDynFlag { flag: u32 },

    // =========================================================================
    // PHASE 6: PATH MOVEMENT
    // =========================================================================
    /// Move along a path of waypoints
    MoveAlongPath {
        waypoints: Vec<Position>,
        movement_type: MovementType,
        repeating: bool,
    },

    /// Do nothing (explicit no-op)
    None,
}

/// Movement type for MoveTo action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MovementType {
    /// Normal walking speed
    Walk,
    /// Running speed
    #[default]
    Run,
    /// Sprint (fastest)
    Sprint,
    /// Charge movement (used for charges)
    Charge,
}

/// React state - how creature responds to threats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReactState {
    /// Does not react to anything
    Passive,
    /// Only reacts when attacked or helped
    Defensive,
    /// Automatically aggros on hostile units in range
    #[default]
    Aggressive,
}

/// AI state data - persisted across updates
#[derive(Debug, Clone, Default)]
pub struct AIStateData {
    /// Spell cooldowns by spell_id -> remaining milliseconds
    pub spell_cooldowns: HashMap<u32, u32>,
    /// Custom timers for scripting
    pub timers: HashMap<u32, u32>,
    /// Whether melee attack is enabled
    pub melee_enabled: bool,
    /// Whether combat movement (chasing) is enabled
    pub combat_movement_enabled: bool,
}

impl AIStateData {
    /// Create new AI state data with default values
    pub fn new() -> Self {
        Self {
            spell_cooldowns: HashMap::new(),
            timers: HashMap::new(),
            melee_enabled: true,
            combat_movement_enabled: true,
        }
    }

    /// Check if a spell is ready to cast
    pub fn is_spell_ready(&self, spell_id: u32) -> bool {
        self.spell_cooldowns
            .get(&spell_id)
            .map(|&cd| cd == 0)
            .unwrap_or(true)
    }

    /// Set a spell on cooldown
    pub fn set_spell_cooldown(&mut self, spell_id: u32, duration_ms: u32) {
        self.spell_cooldowns.insert(spell_id, duration_ms);
    }

    /// Update cooldowns (called each AI update tick)
    pub fn tick(&mut self, diff_ms: u32) {
        // Reduce spell cooldowns
        for cooldown in self.spell_cooldowns.values_mut() {
            *cooldown = cooldown.saturating_sub(diff_ms);
        }

        // Remove expired cooldowns
        self.spell_cooldowns.retain(|_, &mut cd| cd > 0);

        // Reduce custom timers
        for timer in self.timers.values_mut() {
            *timer = timer.saturating_sub(diff_ms);
        }
    }

    /// Clear all cooldowns (called on evade)
    pub fn reset_cooldowns(&mut self) {
        self.spell_cooldowns.clear();
    }
}

/// Thread-safe queue for AI events
pub struct AIEventQueue {
    /// Events queued per creature GUID
    events: std::sync::Mutex<HashMap<ObjectGuid, Vec<AIEvent>>>,
}

impl AIEventQueue {
    pub fn new() -> Self {
        Self {
            events: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Queue an event for a creature
    pub fn push(&self, creature_guid: ObjectGuid, event: AIEvent) {
        if let Ok(mut events) = self.events.lock() {
            events.entry(creature_guid).or_default().push(event);
        }
    }

    /// Take all pending events for a specific creature
    pub fn take_for(&self, creature_guid: ObjectGuid) -> Vec<AIEvent> {
        if let Ok(mut events) = self.events.lock() {
            events.remove(&creature_guid).unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    /// Check if a creature has pending events
    pub fn has_events(&self, creature_guid: ObjectGuid) -> bool {
        if let Ok(events) = self.events.lock() {
            events
                .get(&creature_guid)
                .map(|v| !v.is_empty())
                .unwrap_or(false)
        } else {
            false
        }
    }
}

impl Default for AIEventQueue {
    fn default() -> Self {
        Self::new()
    }
}
