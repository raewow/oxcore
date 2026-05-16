//! Slim Creature object - only runtime data
//!
//! Template data lives in CreatureManager.

use super::ai::{AIState, AIStateData};
use super::combat::{CombatState, ThreatManager};
use super::death::DeathState;
use super::manager::{ClassLevelStats, CreatureTemplate};
use super::movement::{MotionMaster, MoveSpline};
use crate::shared::protocol::{ObjectGuid, Position};
use rand::Rng;

/// Slim creature object
#[derive(Debug, Clone)]
pub struct Creature {
    // ========== Identity ==========
    /// Unique spawn GUID
    pub guid: ObjectGuid,
    /// Entry ID (links to creature_template)
    pub entry: u32,
    /// Spawn ID (database reference)
    pub spawn_id: u32,

    // ========== Location ==========
    /// Current position
    pub position: Position,
    /// Spawn position (home location)
    pub home_position: Position,
    /// Current map ID
    pub map_id: u32,
    /// Instance ID (0 for continents, >0 for instances)
    pub instance_id: u32,

    // ========== Display (cached from template) ==========
    /// Current model ID
    pub display_id: u32,
    /// Original model ID
    pub native_display_id: u32,
    /// Model scale (1.0 = normal size)
    pub scale: f32,
    /// Bounding radius for collision (from creature_display_info_addon, default 0.5)
    pub bounding_radius: f32,
    /// Combat reach for melee range calculation (from creature_display_info_addon, default 1.5)
    pub combat_reach: f32,

    // ========== Stats (cached from template + classlevelstats) ==========
    /// Creature level
    pub level: u8,
    /// Maximum HP
    pub max_health: u32,
    /// Current HP
    pub current_health: u32,
    /// Maximum mana (0 for non-casters)
    pub max_mana: u32,
    /// Current mana
    pub current_mana: u32,
    /// Faction template ID
    pub faction: u32,
    /// Unit flags (combat, etc.)
    pub unit_flags: u32,
    /// Dynamic flags (lootable, dead, etc.)
    pub dynamic_flags: u32,
    /// Stand/animation state (0=stand, 1=sit, 3=sleep, 4=kneel, 7=dead)
    pub stand_state: u8,
    /// NPC flags (vendor, trainer, etc.)
    pub npc_flags: u32,
    /// Armor value (from classlevelstats * armor_multiplier)
    pub armor: u32,
    /// Minimum melee damage (from classlevelstats * damage_multiplier)
    pub damage_min: u32,
    /// Maximum melee damage
    pub damage_max: u32,
    /// Base attack power (from classlevelstats, used for UNIT_FIELD_ATTACK_POWER in create packet)
    pub attack_power: i32,

    // ========== Metadata ==========
    /// Creature name (cached from template)
    pub name: String,
    /// Creature type (cached from template) - beast, humanoid, etc.
    pub creature_type: u8,
    /// Server-side static flags from DB (includes VISIBLE_TO_GHOSTS for spirit healers)
    pub static_flags1: u32,
    /// Spell IDs from creature_template (spell1-4), cached for AI spell selection
    pub spells: [u32; 4],

    // ========== World State ==========
    /// Phase mask for visibility (bitfield)
    pub phase_mask: u32,
    /// Whether spawned in world
    pub in_world: bool,

    // ========== Combat (embedded per Option 3 architecture decision) ==========
    /// Combat state for threat tracking and combat status
    pub combat: CombatState,

    /// Threat manager for sophisticated threat handling (Phase 5)
    pub threat_manager: ThreatManager,

    /// Attack timer for auto-attack (milliseconds until next attack)
    /// Counts down each update, when reaches 0 → attack ready
    /// Reset to base_attack_time after each attack
    pub attack_timer: u32,

    /// Base attack speed in milliseconds (from creature_template)
    pub base_attack_time: u32,

    // ========== Regeneration ==========
    /// Timer accumulator for regen ticks (fires every 2000ms)
    pub regen_timer: u32,

    // ========== Death ==========
    /// Death state machine
    pub death_state: DeathState,
    /// Corpse decay timer (milliseconds remaining)
    pub corpse_decay_timer: u32,
    /// When creature should respawn (unix timestamp ms)
    pub respawn_time: u64,
    /// Who can loot this corpse
    pub loot_recipient: Option<ObjectGuid>,
    /// Whether this creature has loot available for players
    pub has_loot: bool,

    // ========== AI ==========
    /// AI state machine state
    pub ai_state: AIState,
    /// AI state data (cooldowns, timers, etc.)
    pub ai_state_data: AIStateData,

    // ========== Auras (simplified for AI scripting) ==========
    /// Active auras on this creature: (spell_id, remaining_ms, stacks)
    /// 0 remaining = permanent. Updated by aura apply/remove calls.
    pub auras: Vec<(u32, u32, u8)>,

    // ========== Movement ==========
    /// MotionMaster for movement generator stack
    pub motion_master: MotionMaster,
    /// Current movement spline for smooth interpolation
    pub move_spline: MoveSpline,
    /// Wander distance from spawn data (0 = no wander), used to restore wander after combat
    pub wander_distance: f32,
    /// Walk speed rate multiplier from DB (actual walk speed = rate * 2.5)
    pub speed_walk: f32,
    /// Run speed rate multiplier from DB (actual run speed = rate * 7.0)
    pub speed_run: f32,
}

impl Creature {
    /// Create a new creature from spawn data, template, and class level stats
    pub fn new(
        guid: ObjectGuid,
        entry: u32,
        spawn_id: u32,
        position: Position,
        map_id: u32,
        instance_id: u32,
        template: &CreatureTemplate,
        phase_mask: u32,
        class_stats: Option<&ClassLevelStats>,
    ) -> Self {
        let level = template.min_level; // TODO: Random between min/max in later phase
        let health = template.calculate_health(level, class_stats);
        let mana = template.calculate_mana(level, class_stats);
        let armor = template.calculate_armor(level, class_stats);
        let (damage_min, damage_max) = template.calculate_damage(level, class_stats);
        let attack_power = class_stats.map(|s| s.attack_power).unwrap_or(0);

        // Defensive validation: Ensure the entry in the GUID matches the creature's entry
        // This catches bugs where GUID and creature entry diverge (which causes invisibility)
        debug_assert_eq!(
            guid.entry(),
            entry,
            "Creature entry mismatch: GUID has entry {}, but creature.entry is {}",
            guid.entry(),
            entry
        );

        Self {
            // Identity
            guid,
            entry,
            spawn_id,

            // Location
            position,
            home_position: position,
            map_id,
            instance_id,

            // Display
            display_id: template.get_display_id(),
            native_display_id: template.get_display_id(),
            scale: template.scale,
            bounding_radius: 0.5,
            combat_reach: 1.5,

            // Stats (from template + classlevelstats)
            level,
            max_health: health,
            current_health: health,
            max_mana: mana,
            current_mana: mana,
            faction: template.faction,
            unit_flags: template.unit_flags,
            dynamic_flags: 0,
            stand_state: 0,
            npc_flags: template.npc_flags,
            armor,
            damage_min,
            damage_max,
            attack_power,

            // Metadata
            name: template.name.clone(),
            creature_type: template.creature_type,
            static_flags1: template.static_flags1,
            spells: template.spells,

            // World state
            phase_mask,
            in_world: false,

            // Combat
            combat: CombatState::new(),
            threat_manager: ThreatManager::new(guid),
            attack_timer: 0,
            base_attack_time: template.attack_time,

            // Regeneration
            regen_timer: 0,

            // Death
            death_state: DeathState::Alive,
            corpse_decay_timer: 0,
            respawn_time: 0,
            loot_recipient: None,
            has_loot: false,

            // AI
            ai_state: AIState::Idle,
            ai_state_data: AIStateData::new(),

            // Movement
            auras: Vec::new(),
            motion_master: MotionMaster::new(),
            move_spline: MoveSpline::default(),
            wander_distance: 0.0,
            speed_walk: 1.0,    // Default rate, overridden by model_info
            speed_run: 1.14286, // Default rate (vmangos DEFAULT_NPC_RUN_SPEED_RATE)
        }
    }

    /// Get actual walk speed in yards/sec (rate * base walk speed)
    pub fn walk_speed(&self) -> f32 {
        self.speed_walk * 2.5
    }

    /// Get actual run speed in yards/sec (rate * base run speed)
    pub fn run_speed(&self) -> f32 {
        self.speed_run * 7.0
    }

    /// Check if creature is alive
    pub fn is_alive(&self) -> bool {
        self.current_health > 0
    }

    /// Apply damage to creature, returns actual damage dealt
    pub fn take_damage(&mut self, damage: u32) -> u32 {
        let actual_damage = damage.min(self.current_health);
        self.current_health = self.current_health.saturating_sub(damage);
        actual_damage
    }

    /// Check if creature just died (health reached 0)
    pub fn is_dead(&self) -> bool {
        self.current_health == 0
    }

    /// Update attack timer, returns true if attack is ready
    /// Called each game tick with time delta in milliseconds
    pub fn update_attack_timer(&mut self, diff_ms: u32) -> bool {
        if self.attack_timer > 0 {
            self.attack_timer = self.attack_timer.saturating_sub(diff_ms);
            self.attack_timer == 0
        } else {
            false
        }
    }

    /// Reset attack timer to weapon speed after performing attack
    pub fn reset_attack_timer(&mut self, attack_time_ms: u32) {
        self.attack_timer = attack_time_ms;
    }

    /// Check if attack timer is ready (0)
    pub fn is_attack_ready(&self) -> bool {
        self.attack_timer == 0
    }

    /// Called when health reaches 0
    pub fn kill(&mut self, killer: Option<ObjectGuid>) {
        if self.death_state != DeathState::Alive {
            return;
        }

        self.death_state = DeathState::JustDied;
        self.current_health = 0;
        self.combat.leave_combat();
        self.threat_manager.clear();

        // Stop movement immediately on death (vmangos: MotionMaster.Clear + StopMoving in SetDeathState)
        // Snap position to current spline location so the stop packet doesn't teleport the corpse
        if self.move_spline.is_active() {
            self.position = self.move_spline.get_position();
        }
        self.move_spline.stop();
        self.motion_master.clear(self.guid);

        // Set loot recipient to first attacker or killer (tapping)
        self.loot_recipient = killer.or_else(|| self.combat.attackers.iter().next().copied());
    }

    /// Set loot recipient (tapping mechanics)
    pub fn set_loot_recipient(&mut self, recipient: ObjectGuid) {
        if self.loot_recipient.is_none() {
            self.loot_recipient = Some(recipient);
        }
    }

    /// Check if a player can loot this corpse (tapping check)
    pub fn can_loot(&self, player_guid: ObjectGuid) -> bool {
        match self.loot_recipient {
            Some(recipient) => recipient == player_guid,
            None => false,
        }
    }

    /// Get loot recipient for UI display (gray nameplate for others)
    pub fn get_loot_recipient(&self) -> Option<ObjectGuid> {
        self.loot_recipient
    }

    /// Transition from JustDied to Corpse state
    pub fn set_corpse_state(&mut self, decay_time_ms: u32) {
        self.death_state = DeathState::Corpse;
        self.corpse_decay_timer = decay_time_ms;
    }

    /// Update corpse timer, returns true if corpse should be removed
    pub fn update_corpse_timer(&mut self, diff_ms: u32) -> bool {
        if self.death_state != DeathState::Corpse {
            return false;
        }

        self.corpse_decay_timer = self.corpse_decay_timer.saturating_sub(diff_ms);
        self.corpse_decay_timer == 0
    }

    /// Transition from Corpse to Dead (no corpse visible)
    pub fn remove_corpse(&mut self) {
        self.death_state = DeathState::Dead;
        self.corpse_decay_timer = 0;
    }

    /// Calculate respawn time with spawn flags
    pub fn calculate_respawn_time_with_flags(
        &self,
        base_time_secs: u32,
        spawn_flags: u32,
        nearby_player_count: u32,
    ) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};

        let mut time = base_time_secs;

        if spawn_flags & super::spawn::spawn_flags::RANDOM_RESPAWN_TIME != 0 {
            let variance = (time as f32 * 0.1 * rand::random::<f32>()) as u32;
            let min = time - time / 10;
            time = min + variance;
        }

        if spawn_flags & super::spawn::spawn_flags::DYNAMIC_RESPAWN_TIME != 0 {
            let scale = match nearby_player_count {
                0..=1 => 1.0,
                2..=5 => 0.8,
                6..=10 => 0.6,
                _ => 0.5,
            };
            time = (time as f32 * scale) as u32;
        }

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        now_ms + (time as u64 * 1000)
    }

    /// Set respawn time based on calculated delay
    pub fn set_respawn_timer(&mut self, delay_secs: u32) {
        use std::time::{SystemTime, UNIX_EPOCH};

        let respawn_delay_ms = delay_secs as u64 * 1000;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        self.respawn_time = now + respawn_delay_ms;
    }

    /// Check if respawn time has passed
    pub fn should_respawn(&self, current_time: u64) -> bool {
        self.death_state == DeathState::Dead && current_time >= self.respawn_time
    }

    /// Reset creature to alive state at home position
    pub fn respawn(&mut self) {
        self.death_state = DeathState::Alive;
        self.position = self.home_position;
        self.current_health = self.max_health;
        self.current_mana = self.max_mana;
        self.ai_state = AIState::Idle;
        self.combat.leave_combat();
        self.threat_manager.clear();
        self.corpse_decay_timer = 0;
        self.respawn_time = 0;
        self.loot_recipient = None;
        self.has_loot = false;
    }

    /// Mark creature as having loot available
    pub fn set_has_loot(&mut self, has_loot: bool) {
        self.has_loot = has_loot;
    }

    /// Check if creature has loot available
    pub fn has_loot(&self) -> bool {
        self.has_loot
    }

    // ========== Aura helpers ==========

    /// Check if creature has a specific aura by spell ID
    pub fn has_aura(&self, spell_id: u32) -> bool {
        self.auras.iter().any(|(id, _, _)| *id == spell_id)
    }

    /// Add an aura to the creature (simplified tracking for AI scripting)
    pub fn add_aura(&mut self, spell_id: u32, duration_ms: u32, stacks: u8) {
        // Update existing aura if present
        if let Some(existing) = self.auras.iter_mut().find(|(id, _, _)| *id == spell_id) {
            existing.1 = duration_ms;
            existing.2 = stacks;
        } else {
            self.auras.push((spell_id, duration_ms, stacks));
        }
    }

    /// Remove an aura by spell ID
    pub fn remove_aura(&mut self, spell_id: u32) {
        self.auras.retain(|(id, _, _)| *id != spell_id);
    }

    /// Update aura durations, removing expired ones
    pub fn update_auras(&mut self, diff_ms: u32) {
        for aura in &mut self.auras {
            if aura.1 > 0 {
                aura.1 = aura.1.saturating_sub(diff_ms);
            }
        }
        // Remove expired non-permanent auras (duration was >0 and is now 0)
        // Permanent auras have duration 0 from the start (tracked via initial value)
        self.auras.retain(|(_, dur, _)| *dur > 0);
    }

    /// Clear all auras (e.g., on death)
    pub fn clear_auras(&mut self) {
        self.auras.clear();
    }
}
