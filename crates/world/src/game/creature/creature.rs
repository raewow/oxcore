//! Slim Creature object - only runtime data
//!
//! Template data lives in CreatureManager.

use super::ai::{AIState, AIStateData};
use super::combat::{CombatState, ThreatManager};
use super::death::DeathState;
use super::manager::{ClassLevelStats, CreatureTemplate};
use super::movement::{MotionMaster, MoveSpline};
use crate::core::common::movement::MovementInfo;
use crate::game::player::auras::{Aura, AuraContainer};
use oxcore_shared::protocol::{ObjectGuid, Position};
use rand::Rng;

/// Default pause applied to out-of-combat movement on player interaction (ms).
pub const NPC_MOVEMENT_PAUSE_TIME: u32 = 180_000;

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

    /// Player currently charmed by this creature.
    pub charm_guid: Option<ObjectGuid>,
    /// Player that owns this creature when it is a runtime pet.
    pub owner_guid: Option<ObjectGuid>,

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

    // ========== Auras ==========
    /// Active aura effects on this creature.
    ///
    /// Creatures do not use player-visible slots, but they share the same storage
    /// so spell logic can retain caster, effect, duration, stack, and type data.
    pub auras: AuraContainer,

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
    /// Whether out-of-combat movement has been paused (player interaction)
    pub movement_paused: bool,
    /// Movement and transport state (flags, transport GUID and local offset when riding one)
    pub movement_info: MovementInfo,
    /// Current unit this creature is following, if any
    pub following_target: Option<ObjectGuid>,
    /// Units currently following this creature
    pub followers: Vec<ObjectGuid>,
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
            charm_guid: None,
            owner_guid: None,
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
            auras: AuraContainer::new(),
            motion_master: MotionMaster::new(),
            move_spline: MoveSpline::default(),
            wander_distance: 0.0,
            speed_walk: 1.0,    // Default rate, overridden by model_info
            speed_run: 1.14286, // Default rate
            movement_paused: false,
            movement_info: MovementInfo::new(),
            following_target: None,
            followers: Vec::new(),
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

    /// Pause out-of-combat movement when a player interacts with us
    pub fn pause_out_of_combat_movement(&mut self) {
        self.pause_out_of_combat_movement_for(NPC_MOVEMENT_PAUSE_TIME);
    }

    /// Pause out-of-combat movement for a specific duration.
    ///
    /// Creatures in combat ignore this, and only random or waypoint movement can be paused.
    /// The paused flag gates the interaction flows that read [`Self::movement_paused`].
    pub fn pause_out_of_combat_movement_for(&mut self, pause_time_ms: u32) {
        if self.combat.in_combat {
            return;
        }

        // Only random and waypoint movement carries a pause timer; the flag below still
        // applies to any generator, since the interaction flows depend on it.
        self.motion_master.add_pause_time(pause_time_ms);

        self.movement_paused = true;
        self.motion_master.flags.insert(2); // 0x02 = movement paused flag
    }

    /// Resume out-of-combat movement after interaction ends
    pub fn resume_out_of_combat_movement(&mut self) {
        self.movement_paused = false;
        self.motion_master.flags.remove(2); // 0x02 = movement paused flag
    }

    /// Register a follower that is tracking this creature.
    pub fn add_follower(&mut self, follower_guid: ObjectGuid) {
        if !self.followers.contains(&follower_guid) {
            self.followers.push(follower_guid);
        }
    }

    /// Unregister a follower.
    pub fn remove_follower(&mut self, follower_guid: ObjectGuid) {
        self.followers.retain(|guid| *guid != follower_guid);
    }

    /// Stop following the current target.
    pub fn stop_following(&mut self) {
        self.following_target = None;
        self.ai_state = AIState::Idle;
        self.motion_master.stop(self.guid);
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

        // Stop movement immediately on death
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

    /// Respawn delay in seconds, with the spawn flags applied to `spawntimesecs`.
    ///
    /// Returns a *delay*, not a point in time: the flow scales the base
    /// delay by the spawn flags and only then turns it into an absolute respawn
    /// time. Handing back an absolute timestamp here is what let the caller add
    /// "now" a second time, pushing every respawn decades into the future.
    pub fn calculate_respawn_delay_secs(
        &self,
        base_time_secs: u32,
        spawn_flags: u32,
        nearby_player_count: u32,
    ) -> u32 {
        let mut time = base_time_secs as f32;

        // urand(90, 110) / 100
        if spawn_flags & super::spawn::spawn_flags::RANDOM_RESPAWN_TIME != 0 {
            time *= 0.9 + rand::random::<f32>() * 0.2;
        }

        if spawn_flags & super::spawn::spawn_flags::DYNAMIC_RESPAWN_TIME != 0 {
            let scale = match nearby_player_count {
                0..=1 => 1.0,
                2..=5 => 0.8,
                6..=10 => 0.6,
                _ => 0.5,
            };
            time *= scale;
        }

        time as u32
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
        self.in_world = true;
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
        self.auras.has_aura(spell_id)
    }

    /// Add or refresh a creature aura effect.
    pub fn add_aura(&mut self, aura: Aura) -> Option<u8> {
        self.auras.add_aura(aura)
    }

    /// Remove all effects belonging to a spell.
    pub fn remove_aura(&mut self, spell_id: u32) -> Vec<(Aura, u8)> {
        self.auras.remove_spell_auras(spell_id)
    }

    /// Advance aura durations and return expired effect keys for AuraSystem cleanup.
    pub fn update_auras(&mut self, diff_ms: u32) -> Vec<(u32, u8)> {
        self.auras.tick_durations(diff_ms)
    }

    /// Clear all auras (e.g., on death)
    pub fn clear_auras(&mut self) {
        self.auras.remove_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::creature::spawn::spawn_flags;

    fn template() -> CreatureTemplate {
        CreatureTemplate {
            entry: 1,
            name: String::from("Test Creature"),
            subname: None,
            min_level: 1,
            max_level: 1,
            faction: 35,
            model_id_1: 1,
            model_id_2: 0,
            model_id_3: 0,
            model_id_4: 0,
            scale: 1.0,
            npc_flags: 0,
            unit_flags: 0,
            static_flags1: 0,
            flags_extra: 0,
            creature_type: 7,
            unit_class: 1,
            health_multiplier: 1.0,
            power_multiplier: 1.0,
            armor_multiplier: 1.0,
            damage_multiplier: 1.0,
            damage_variance: 0.0,
            attack_time: 2000,
            rank: 0,
            gossip_menu_id: 0,
            vendor_id: 0,
            trainer_id: 0,
            trainer_type: 0,
            spells: [0; 4],
        }
    }

    fn creature() -> Creature {
        Creature::new(
            ObjectGuid::new_creature(1, 1),
            1,
            1,
            Position::default(),
            0,
            0,
            &template(),
            1,
            None,
        )
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    /// The regression: the delay used to come back as an absolute timestamp,
    /// which `set_respawn_timer` then added to "now" a second time — putting
    /// every respawn ~55 years out, so `should_respawn` never fired.
    #[test]
    fn respawn_timer_is_the_spawn_delay_from_now() {
        let mut creature = creature();
        let delay = creature.calculate_respawn_delay_secs(300, 0, 0);
        assert_eq!(delay, 300);

        creature.set_respawn_timer(delay);
        creature.death_state = DeathState::Dead;

        let now = now_ms();
        assert!(!creature.should_respawn(now));
        assert!(!creature.should_respawn(now + 299_000));
        assert!(creature.should_respawn(now + 301_000));
    }

    /// `urand(90, 110) / 100` around the base delay.
    #[test]
    fn random_respawn_time_stays_within_ten_percent() {
        let creature = creature();

        for _ in 0..200 {
            let delay =
                creature.calculate_respawn_delay_secs(300, spawn_flags::RANDOM_RESPAWN_TIME, 0);
            assert!((270..=330).contains(&delay), "delay {delay} out of range");
        }
    }

    /// Death drops the movement generators and clears `in_world`; a respawned
    /// creature has to be back in the world or nothing will tick it.
    #[test]
    fn respawn_puts_the_creature_back_in_the_world() {
        let mut creature = creature();
        creature.in_world = true;
        creature.kill(None);
        creature.in_world = false; // corpse removal

        creature.respawn();

        assert!(creature.in_world);
        assert_eq!(creature.death_state, DeathState::Alive);
        assert_eq!(creature.current_health, creature.max_health);
        assert_eq!(creature.respawn_time, 0);
    }
}
