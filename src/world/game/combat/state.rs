//! Combat State - Per-player combat state embedded in Player struct
//!
//! Contains auto-attack timers, weapon info, combat flags, and combo points.

use crate::shared::protocol::ObjectGuid;
use std::collections::HashSet;

/// Attack outcome from hit table
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackOutcome {
    Miss,
    Dodge,
    Parry,
    Glancing,
    Block,
    CriticalHit,
    CrushingBlow,
    NormalHit,
}

impl AttackOutcome {
    /// Convert outcome to hit info flags for SMSG_ATTACKERSTATEUPDATE
    /// Values from MaNGOS UnitDefines.h (1.12.1 client)
    pub fn to_hit_info(&self) -> u32 {
        use crate::shared::messages::combat::HitInfo;
        let affects = HitInfo::AffectsVictim as u32;
        match self {
            // No AFFECTS_VICTIM for miss/dodge/parry
            AttackOutcome::Miss => HitInfo::Miss as u32, // 0x10
            AttackOutcome::Dodge => HitInfo::NormalSwing as u32, // victim state handles
            AttackOutcome::Parry => HitInfo::NormalSwing as u32, // victim state handles
            // All damage-dealing outcomes include AFFECTS_VICTIM
            AttackOutcome::Glancing => affects | HitInfo::Glancing as u32,
            AttackOutcome::Block => affects,
            AttackOutcome::CriticalHit => affects | HitInfo::CriticalHit as u32,
            AttackOutcome::CrushingBlow => affects | HitInfo::Crushing as u32,
            AttackOutcome::NormalHit => affects,
        }
    }

    /// Check if this outcome deals damage
    pub fn deals_damage(&self) -> bool {
        matches!(
            self,
            AttackOutcome::NormalHit
                | AttackOutcome::Glancing
                | AttackOutcome::Block
                | AttackOutcome::CriticalHit
                | AttackOutcome::CrushingBlow
        )
    }

    /// Get display name for combat log
    pub fn display_name(&self) -> &'static str {
        match self {
            AttackOutcome::Miss => "miss",
            AttackOutcome::Dodge => "dodge",
            AttackOutcome::Parry => "parry",
            AttackOutcome::Glancing => "glancing",
            AttackOutcome::Block => "block",
            AttackOutcome::CriticalHit => "crit",
            AttackOutcome::CrushingBlow => "crushing",
            AttackOutcome::NormalHit => "hit",
        }
    }
}

/// Which hand is attacking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackHand {
    MainHand,
    OffHand,
    Ranged,
}

impl AttackHand {
    /// Get damage multiplier for this hand
    pub fn damage_multiplier(&self) -> f32 {
        match self {
            AttackHand::MainHand => 1.0,
            AttackHand::OffHand => 0.5,
            AttackHand::Ranged => 1.0,
        }
    }

    /// Check if this is a melee attack
    pub fn is_melee(&self) -> bool {
        matches!(self, AttackHand::MainHand | AttackHand::OffHand)
    }

    /// Check if this is ranged attack
    pub fn is_ranged(&self) -> bool {
        matches!(self, AttackHand::Ranged)
    }
}

/// Damage result from combat calculation
#[derive(Debug, Clone)]
pub struct DamageResult {
    pub outcome: AttackOutcome,
    pub damage: u32,
    pub absorbed: u32,
    pub resisted: u32,
    pub blocked: u32,
    pub overkill: u32,
    pub damage_school: u8, // 0 = physical
    pub hand: AttackHand,
}

impl DamageResult {
    /// Create a damage result for a missed/dodged/parried attack
    pub fn no_damage(outcome: AttackOutcome, hand: AttackHand) -> Self {
        Self {
            outcome,
            damage: 0,
            absorbed: 0,
            resisted: 0,
            blocked: 0,
            overkill: 0,
            damage_school: 0,
            hand,
        }
    }

    /// Get effective damage (after absorbs/resists)
    pub fn effective_damage(&self) -> u32 {
        self.damage
            .saturating_sub(self.absorbed)
            .saturating_sub(self.resisted)
    }
}

/// Per-player combat state (embedded in Player struct)
#[derive(Debug, Clone)]
pub struct CombatState {
    // Combat flags
    pub in_combat: bool,
    pub combat_timer: u32, // ms, decays to 0 when no attackers
    pub attack_target: Option<ObjectGuid>,
    pub attackers: HashSet<ObjectGuid>,

    // Attack timers (countdown in ms, attack when reaches 0)
    pub main_hand_timer: u32,
    pub off_hand_timer: u32,
    pub ranged_timer: u32,

    // Attack speeds (ms between swings, from weapon + haste)
    pub main_hand_speed: u32, // Default 2000ms
    pub off_hand_speed: u32,  // Default 2000ms
    pub ranged_speed: u32,    // Default 2800ms

    // Weapon info (cached from equipment)
    pub main_hand_min_dmg: f32,
    pub main_hand_max_dmg: f32,
    pub off_hand_min_dmg: f32,
    pub off_hand_max_dmg: f32,
    pub ranged_min_dmg: f32,
    pub ranged_max_dmg: f32,

    // Combat capabilities
    pub can_parry: bool,
    pub can_block: bool,
    pub can_dual_wield: bool,
    pub has_ranged_weapon: bool,

    // Rogue combo points
    pub combo_target: Option<ObjectGuid>,
    pub combo_points: u8, // 0-5

    // Auto-attack state
    pub is_auto_attacking: bool,
    pub is_auto_shooting: bool,

    /// Last swing error (0=OK, 1=not in range) - prevents packet spam
    pub last_swing_error: u8,

    /// Diminishing returns state — tracks DR levels per group when CC is applied to this player
    pub diminishing: crate::world::game::player::spells::diminishing::DiminishingState,

    /// Recent PvP damage contributors used for honor-award calculation on death.
    /// Updated by `HonorSystem::record_damage`, drained by `reward_honor_on_death`.
    pub honor: crate::world::game::player::honor::ContributorTracker,

    /// In-session accumulator for this week's honorable kill count. Persisted
    /// to `characters.honor_last_week_hk` on logout.
    pub honor_last_week_hk: u32,

    /// In-session accumulator for this week's honor contribution points.
    /// Persisted to `characters.honor_last_week_cp` on logout.
    pub honor_last_week_cp: f32,
}

impl Default for CombatState {
    fn default() -> Self {
        Self {
            in_combat: false,
            combat_timer: 0,
            attack_target: None,
            attackers: HashSet::new(),
            main_hand_timer: 0,
            off_hand_timer: 0,
            ranged_timer: 0,
            main_hand_speed: 2000,
            off_hand_speed: 2000,
            ranged_speed: 2800,
            main_hand_min_dmg: 1.0,
            main_hand_max_dmg: 2.0,
            off_hand_min_dmg: 1.0,
            off_hand_max_dmg: 2.0,
            ranged_min_dmg: 1.0,
            ranged_max_dmg: 2.0,
            can_parry: false,
            can_block: false,
            can_dual_wield: false,
            has_ranged_weapon: false,
            combo_target: None,
            combo_points: 0,
            is_auto_attacking: false,
            is_auto_shooting: false,
            last_swing_error: 0,
            diminishing: Default::default(),
            honor: crate::world::game::player::honor::ContributorTracker::new(),
            honor_last_week_hk: 0,
            honor_last_week_cp: 0.0,
        }
    }
}

impl CombatState {
    /// Start auto-attacking a target
    pub fn start_attack(&mut self, target: ObjectGuid) {
        self.attack_target = Some(target);
        self.is_auto_attacking = true;
        // main_hand_timer stays at 0 so first swing fires on next tick
        // update_auto_attack() will reset it to main_hand_speed after firing
        if self.can_dual_wield && self.off_hand_timer == 0 {
            self.off_hand_timer = self.off_hand_speed / 2;
        }
    }

    /// Stop auto-attacking
    pub fn stop_attack(&mut self) {
        self.is_auto_attacking = false;
        self.attack_target = None;
    }

    /// Start auto-shoot (ranged)
    pub fn start_shoot(&mut self, target: ObjectGuid) {
        self.attack_target = Some(target);
        self.is_auto_shooting = true;
        if self.ranged_timer == 0 {
            self.ranged_timer = self.ranged_speed;
        }
    }

    /// Stop auto-shooting
    pub fn stop_shoot(&mut self) {
        self.is_auto_shooting = false;
        if !self.is_auto_attacking {
            self.attack_target = None;
        }
    }

    /// Enter combat
    pub fn enter_combat(&mut self, attacker: ObjectGuid) {
        self.in_combat = true;
        self.combat_timer = 6000; // 6 seconds
        self.attackers.insert(attacker);
    }

    /// Update combat timer, returns true if combat ended
    pub fn update_combat_timer(&mut self, diff_ms: u32) -> bool {
        if self.combat_timer > 0 {
            self.combat_timer = self.combat_timer.saturating_sub(diff_ms);
            if self.combat_timer == 0 && self.attackers.is_empty() {
                self.in_combat = false;
                return true; // Combat ended
            }
        }
        false
    }

    /// Remove an attacker, returns true if combat ended
    pub fn remove_attacker(&mut self, attacker: ObjectGuid) -> bool {
        self.attackers.remove(&attacker);
        if self.attackers.is_empty() && self.combat_timer == 0 {
            self.in_combat = false;
            return true;
        }
        false
    }

    /// Get current attack speed for a hand
    pub fn get_attack_speed(&self, hand: AttackHand) -> u32 {
        match hand {
            AttackHand::MainHand => self.main_hand_speed,
            AttackHand::OffHand => self.off_hand_speed,
            AttackHand::Ranged => self.ranged_speed,
        }
    }

    /// Get weapon damage range for a hand
    pub fn get_weapon_damage(&self, hand: AttackHand) -> (f32, f32) {
        match hand {
            AttackHand::MainHand => (self.main_hand_min_dmg, self.main_hand_max_dmg),
            AttackHand::OffHand => (self.off_hand_min_dmg, self.off_hand_max_dmg),
            AttackHand::Ranged => (self.ranged_min_dmg, self.ranged_max_dmg),
        }
    }

    /// Add combo points to target
    pub fn add_combo_points(&mut self, target: ObjectGuid, points: u8) {
        if self.combo_target != Some(target) {
            self.combo_target = Some(target);
            self.combo_points = 0;
        }
        self.combo_points = (self.combo_points + points).min(5);
    }

    /// Clear combo points
    pub fn clear_combo_points(&mut self) {
        self.combo_target = None;
        self.combo_points = 0;
    }

    /// Reset all swing timers
    pub fn reset_swing_timers(&mut self) {
        self.main_hand_timer = 0;
        self.off_hand_timer = 0;
        self.ranged_timer = 0;
    }
}
