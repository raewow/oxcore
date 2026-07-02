//! Spell State - SpellsState, ActiveCast, SpellMod
//!
//! All spell-related state for a player is stored here and embedded in the Player struct.

use oxcore_shared::protocol::ObjectGuid;
use std::collections::{BTreeMap, HashMap, HashSet};

/// Number of spell schools in vanilla WoW.
pub const NUM_SPELL_SCHOOLS: usize = 7;

/// Number of concurrent spell slots (matches MaNGOS CURRENT_SPELL_TYPES).
pub const NUM_CURRENT_SPELLS: usize = 4;

/// Which slot a spell occupies. MaNGOS allows concurrent spells in different slots:
/// e.g., Heroic Strike (Melee) while auto-attacking, Auto-Shot (Autorepeat) while casting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CurrentSpellType {
    /// Next-melee abilities: Heroic Strike, Raptor Strike, Cleave
    Melee = 0,
    /// Auto-repeat: Auto-Shot, Wand Shoot
    Autorepeat = 1,
    /// Channeled spells: Mind Flay, Drain Life, Blizzard
    Channeled = 2,
    /// Generic cast-time and instant spells: Fireball, Flash Heal, etc.
    Generic = 3,
}

/// Spell cast state machine (matches MaNGOS SpellState).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellState {
    /// Cast bar running (non-channeled) — waiting for timer to complete
    Preparing,
    /// Channeled spell active — ticking effects over time
    Casting,
    /// Cast complete (effects applied) — cleanup phase
    Finished,
    /// Spell cast but effects delayed (projectile in flight)
    Delayed,
}

/// Per-player spells state, embedded in Player struct.
///
/// This contains ALL spell-related data for a player:
/// - What spells they know
/// - What they're currently casting
/// - Cooldown timers
/// - GCD state
/// - School lockouts
/// - Spell modifiers from talents/auras
#[derive(Debug, Clone)]
pub struct SpellsState {
    // === Spellbook ===
    /// Set of all learned spell IDs (for O(1) knows-spell checks)
    pub learned_spells: HashSet<u32>,

    /// Ordered spellbook for client display
    pub spellbook: Vec<u32>,

    // === Active Casts ===
    /// Concurrent spell slots matching MaNGOS CURRENT_SPELL_TYPES.
    /// Multiple spells can be active simultaneously in different slots:
    /// [0] Melee (Heroic Strike), [1] Autorepeat (Auto-Shot), [2] Channeled (Mind Flay), [3] Generic (Fireball)
    pub current_spells: [Option<ActiveCast>; NUM_CURRENT_SPELLS],

    // === Cooldowns ===
    /// Per-spell cooldowns: spell_id -> cooldown_end_time_ms (game time)
    pub cooldowns: HashMap<u32, u64>,

    /// Category cooldowns: category_id -> cooldown_end_time_ms (game time)
    /// Spells in the same category share a cooldown (e.g., Health Potion + Mana Potion)
    pub category_cooldowns: HashMap<u32, u64>,

    /// Global Cooldown end time (game time ms)
    /// Most spells trigger 1.5s GCD. Some (Fury abilities, some instant casts) trigger 1.0s.
    pub gcd_end: u64,

    // === School Lockout ===
    /// Per-school lockout end times (from spell interrupt).
    /// When a spell is interrupted, that spell's school is locked for N seconds.
    /// Index: 0=Physical, 1=Holy, 2=Fire, 3=Nature, 4=Frost, 5=Shadow, 6=Arcane
    pub school_lockouts: [u64; NUM_SPELL_SCHOOLS],

    // === Spell Modifiers ===
    /// Active spell modifiers from talents and auras.
    /// These modify spell properties: damage, healing, cost, duration, crit, range, etc.
    /// Indexed by SpellModOp (the property being modified).
    pub spell_modifiers: Vec<SpellMod>,

    // === Delayed Effects (Projectile Travel) ===
    /// Pending spell effects waiting for projectile to arrive.
    /// Ticked every world update; effects execute when delivery_time_ms reaches 0.
    pub cast_counter: u32,

    /// Active dynamic objects created by the player's spells (GUID list).
    /// Matches MaNGOS m_spellDynObjects.
    pub spell_dyn_objects: Vec<ObjectGuid>,

    pub delayed_effects: Vec<DelayedSpellEffect>,

    // === Persistence ===
    /// Flag: spellbook has changed since last save
    pub needs_save: bool,
}

impl Default for SpellsState {
    fn default() -> Self {
        Self {
            learned_spells: HashSet::new(),
            spellbook: Vec::new(),
            current_spells: [None, None, None, None],
            cooldowns: HashMap::new(),
            category_cooldowns: HashMap::new(),
            gcd_end: 0,
            school_lockouts: [0; NUM_SPELL_SCHOOLS],
            spell_modifiers: Vec::new(),
            cast_counter: 0,
            spell_dyn_objects: Vec::new(),
            delayed_effects: Vec::new(),
            needs_save: false,
        }
    }
}

impl SpellsState {
    /// Check if the player knows a specific spell
    pub fn knows_spell(&self, spell_id: u32) -> bool {
        self.learned_spells.contains(&spell_id)
    }

    /// Check if a spell is on cooldown
    pub fn is_on_cooldown(&self, spell_id: u32, now: u64) -> bool {
        if let Some(&cd_end) = self.cooldowns.get(&spell_id) {
            if cd_end > now {
                return true;
            }
        }
        false
    }

    /// Check if GCD is active
    pub fn is_on_gcd(&self, now: u64) -> bool {
        self.gcd_end > now
    }

    /// Check if a spell school is locked out
    pub fn is_school_locked(&self, school: u8, now: u64) -> bool {
        if (school as usize) < NUM_SPELL_SCHOOLS {
            self.school_lockouts[school as usize] > now
        } else {
            false
        }
    }

    /// Get remaining cooldown for a spell in milliseconds
    pub fn get_cooldown_remaining(&self, spell_id: u32, now: u64) -> u32 {
        if let Some(&cd_end) = self.cooldowns.get(&spell_id) {
            if cd_end > now {
                return (cd_end - now) as u32;
            }
        }
        0
    }

    /// GetExpireTime: returns the later of spell-level and category-level cooldown expire times.
    /// Returns None if neither cooldown is active. Permanent CDs are not tracked (stub).
    pub fn get_expire_time(&self, spell_id: u32, category: u32, now: u64) -> Option<u64> {
        let spell_cd = self.cooldowns.get(&spell_id).copied();
        let cat_cd = if category > 0 {
            self.category_cooldowns.get(&category).copied()
        } else {
            None
        };
        let expire = match (spell_cd, cat_cd) {
            (Some(sp), Some(cp)) => Some(sp.max(cp)),
            (Some(sp), None) => Some(sp),
            (None, Some(cp)) => Some(cp),
            (None, None) => None,
        };
        expire.filter(|&t| t > now)
    }

    /// Clear expired cooldowns (housekeeping)
    pub fn clear_expired_cooldowns(&mut self, now: u64) {
        self.cooldowns.retain(|_, &mut cd_end| cd_end > now);
        self.category_cooldowns
            .retain(|_, &mut cd_end| cd_end > now);
    }

    /// Learn a new spell
    pub fn learn_spell(&mut self, spell_id: u32) -> bool {
        if self.learned_spells.insert(spell_id) {
            self.spellbook.push(spell_id);
            self.needs_save = true;
            true
        } else {
            false
        }
    }

    /// Unlearn a spell
    pub fn unlearn_spell(&mut self, spell_id: u32) -> bool {
        if self.learned_spells.remove(&spell_id) {
            self.spellbook.retain(|&id| id != spell_id);
            self.needs_save = true;
            true
        } else {
            false
        }
    }

    /// Add a cooldown for a spell
    pub fn add_cooldown(&mut self, spell_id: u32, duration_ms: u32, now: u64) {
        if duration_ms > 0 {
            self.cooldowns.insert(spell_id, now + duration_ms as u64);
        }
    }

    /// Reset a spell's cooldown
    pub fn reset_cooldown(&mut self, spell_id: u32) {
        self.cooldowns.remove(&spell_id);
    }

    /// Reset all cooldowns
    pub fn reset_all_cooldowns(&mut self) {
        self.cooldowns.clear();
        self.category_cooldowns.clear();
    }

    /// Apply school lockout
    pub fn apply_school_lockout(&mut self, school: u8, duration_ms: u32, now: u64) {
        if (school as usize) < NUM_SPELL_SCHOOLS {
            let lockout_end = now + duration_ms as u64;
            // Only extend, never shorten
            if lockout_end > self.school_lockouts[school as usize] {
                self.school_lockouts[school as usize] = lockout_end;
            }
        }
    }

    /// Apply GCD
    pub fn apply_gcd(&mut self, duration_ms: u32, now: u64) {
        self.gcd_end = now + duration_ms as u64;
    }

    /// Reset GCD (MaNGOS ResetGCD without spell entry — clears all GCD).
    pub fn reset_gcd(&mut self) {
        self.gcd_end = 0;
    }

    /// Format all active cooldowns, GCD, and lockouts into human-readable lines.
    /// Faithful `SpellCaster::PrintCooldownList` port.
    ///
    /// Returns a Vec of display strings; callers send them via chat or log.
    /// Permanent cooldowns are not tracked in the Rust state (permanent_cd_count always 0).
    pub fn format_cooldown_list(&self, now: u64) -> Vec<String> {
        const SCHOOL_NAMES: [&str; 7] = [
            "SPELL_SCHOOL_NORMAL",
            "SPELL_SCHOOL_HOLY",
            "SPELL_SCHOOL_FIRE",
            "SPELL_SCHOOL_NATURE",
            "SPELL_SCHOOL_FROST",
            "SPELL_SCHOOL_SHADOW",
            "SPELL_SCHOOL_ARCANE",
        ];

        let mut lines = Vec::new();
        let mut cd_count = 0u32;
        let perm_cd_count = 0u32;

        // GCD (Rust uses a single gcd_end vs C++ per-category map)
        if self.gcd_end > now {
            let remaining_ms = self.gcd_end - now;
            lines.push(format!("GCD: {}ms remaining", remaining_ms));
            cd_count += 1;
        }

        // Per-spell cooldowns
        for (&spell_id, &cd_end) in &self.cooldowns {
            if cd_end > now {
                let remaining_ms = cd_end - now;
                lines.push(format!("Spell({}) RecTime({}ms)", spell_id, remaining_ms));
                cd_count += 1;
            }
        }

        // Category cooldowns
        for (&cat_id, &cd_end) in &self.category_cooldowns {
            if cd_end > now {
                let remaining_ms = cd_end - now;
                lines.push(format!("Category({}) CatRecTime({}ms)", cat_id, remaining_ms));
                cd_count += 1;
            }
        }

        // School lockouts
        for (school, &lockout_end) in self.school_lockouts.iter().enumerate() {
            if lockout_end > now {
                let remaining_ms = lockout_end - now;
                lines.push(format!(
                    "LOCKOUT for {} with {}ms remaining time cd",
                    SCHOOL_NAMES[school], remaining_ms
                ));
                cd_count += 1;
            }
        }

        lines.push(format!(
            "Found {} cooldown{}.",
            cd_count,
            if cd_count != 1 { "s" } else { "" }
        ));
        lines.push(format!(
            "Found {} permanent cooldown{}.",
            perm_cd_count,
            if perm_cd_count != 1 { "s" } else { "" }
        ));

        lines
    }

    /// Check if any school in the given mask is locked out (MaNGOS CheckLockout).
    pub fn check_lockout_by_mask(&self, school_mask: u32, now: u64) -> bool {
        for (i, &lockout_end) in self.school_lockouts.iter().enumerate() {
            if lockout_end > now && (school_mask & (1 << i)) != 0 {
                return true;
            }
        }
        false
    }

    /// Lock out all schools in the given mask (MaNGOS LockOutSpells).
    pub fn lock_out_spells(&mut self, school_mask: u32, duration_ms: u32, now: u64) {
        for school in 0..NUM_SPELL_SCHOOLS {
            if (school_mask & (1 << school)) != 0 {
                self.apply_school_lockout(school as u8, duration_ms, now);
            }
        }
    }

    /// Remove a spell's category cooldown (MaNGOS RemoveSpellCategoryCooldown).
    pub fn remove_spell_category_cooldown(&mut self, category: u32) {
        self.category_cooldowns.remove(&category);
    }

    // =========================================================================
    // Dynamic Object Methods (MaNGOS SpellCaster DynObject helpers)
    // =========================================================================

    /// Add a dynamic object GUID to the list (MaNGOS AddDynObject).
    pub fn add_dyn_object(&mut self, guid: ObjectGuid) {
        self.spell_dyn_objects.push(guid);
    }

    /// Get all GUIDs for dynamic objects matching a spell and effect index (MaNGOS GetDynObjects).
    pub fn get_dyn_objects(&self, spell_id: u32, effect_index: u8) -> Vec<ObjectGuid> {
        self.spell_dyn_objects
            .iter()
            .copied()
            .filter(|_guid| {
                // DynamicObject* lookup via map not available here;
                // filtering by spell_id and effect_index requires a map query.
                // Stub: returns all GUIDs for now.
                true
            })
            .collect()
    }

    /// Get the first dynamic object GUID matching a spell and effect index (MaNGOS GetDynObject).
    /// Returns None if not found.
    pub fn get_dyn_object(&self, spell_id: u32, effect_index: u8) -> Option<ObjectGuid> {
        // Stub: returns first GUID regardless (actual map lookup deferred).
        self.spell_dyn_objects.first().copied()
    }

    /// Get first dynamic object GUID matching a spell (MaNGOS GetDynObject by spellId only).
    pub fn get_dyn_object_by_spell(&self, spell_id: u32) -> Option<ObjectGuid> {
        self.spell_dyn_objects.first().copied()
    }

    /// Remove all dynamic objects matching a spell_id (MaNGOS RemoveDynObject).
    /// Returns the removed GUIDs so the caller can delete the actual DynamicObjects.
    pub fn remove_dyn_object(&mut self, spell_id: u32) -> Vec<ObjectGuid> {
        let (remaining, removed): (Vec<_>, Vec<_>) =
            self.spell_dyn_objects.drain(..).partition(|_guid| {
                // Full filtering needs DynamicObject lookup. Stub: retain all.
                true
            });
        self.spell_dyn_objects = remaining;
        removed
    }

    /// Remove a specific dynamic object by GUID (MaNGOS RemoveDynObjectWithGUID).
    /// Returns true if the GUID was found and removed.
    pub fn remove_dyn_object_with_guid(&mut self, guid: ObjectGuid) -> bool {
        let len_before = self.spell_dyn_objects.len();
        self.spell_dyn_objects.retain(|g| *g != guid);
        self.spell_dyn_objects.len() < len_before
    }

    /// Remove all dynamic objects (MaNGOS RemoveAllDynObjects).
    /// Returns the removed GUIDs so the caller can delete the actual DynamicObjects.
    pub fn remove_all_dyn_objects(&mut self) -> Vec<ObjectGuid> {
        std::mem::take(&mut self.spell_dyn_objects)
    }

    /// Add a spell modifier
    pub fn add_spell_modifier(&mut self, modifier: SpellMod) {
        self.spell_modifiers.push(modifier);
    }

    /// Remove all spell modifiers from a source spell
    pub fn remove_spell_modifiers_from_source(&mut self, source_spell_id: u32) {
        self.spell_modifiers
            .retain(|m| m.source_spell_id != source_spell_id);
    }

    /// Get spell modifiers that apply to a specific operation
    pub fn get_modifiers_for_op(&self, op: SpellModOp) -> Vec<&SpellMod> {
        self.spell_modifiers.iter().filter(|m| m.op == op).collect()
    }

    /// Get reference to the active cast in a specific slot.
    pub fn get_current_spell(&self, slot: CurrentSpellType) -> Option<&ActiveCast> {
        self.current_spells[slot as usize].as_ref()
    }

    /// Get mutable reference to the active cast in a specific slot.
    pub fn get_current_spell_mut(&mut self, slot: CurrentSpellType) -> Option<&mut ActiveCast> {
        self.current_spells[slot as usize].as_mut()
    }

    /// Set the active cast in a specific slot.
    pub fn set_current_spell(&mut self, slot: CurrentSpellType, cast: ActiveCast) {
        self.current_spells[slot as usize] = Some(cast);
    }

    /// Clear the active cast in a specific slot. Returns the old cast if any.
    pub fn clear_current_spell(&mut self, slot: CurrentSpellType) -> Option<ActiveCast> {
        self.current_spells[slot as usize].take()
    }

    /// Check if a specific slot has an active cast.
    pub fn has_current_spell(&self, slot: CurrentSpellType) -> bool {
        self.current_spells[slot as usize].is_some()
    }

    /// Check if the generic or channeled slot is busy (used for "already casting" validation).
    pub fn is_casting(&self) -> bool {
        self.current_spells[CurrentSpellType::Generic as usize].is_some()
            || self.current_spells[CurrentSpellType::Channeled as usize].is_some()
    }

    /// Check if any non-melee spell slot is active (MaNGOS SpellCaster::IsNonMeleeSpellCasted).
    pub fn is_non_melee_spell_casted(
        &self,
        with_delayed: bool,
        skip_channeled: bool,
        skip_autorepeat: bool,
    ) -> bool {
        // Generic: not finished, and (with_delayed or not delayed)
        if let Some(ref cast) = self.current_spells[CurrentSpellType::Generic as usize] {
            if cast.state != SpellState::Finished
                && (with_delayed || cast.state != SpellState::Delayed)
            {
                return true;
            }
        }
        // Channeled: not finished (channeled spells are still casted even if delayed)
        if !skip_channeled {
            if let Some(ref cast) = self.current_spells[CurrentSpellType::Channeled as usize] {
                if cast.state != SpellState::Finished {
                    return true;
                }
            }
        }
        // Autorepeat: always considered casted if present (even if finished/delayed)
        if !skip_autorepeat {
            if self.current_spells[CurrentSpellType::Autorepeat as usize].is_some() {
                return true;
            }
        }
        false
    }

    /// Check if the Melee slot has a pending next-swing spell (MaNGOS SpellCaster::IsNextSwingSpellCasted).
    /// Returns the spell_id if present, caller must check IsNextMeleeSwingSpell on the spell entry.
    pub fn get_next_swing_spell_id(&self) -> Option<u32> {
        self.current_spells[CurrentSpellType::Melee as usize]
            .as_ref()
            .map(|cast| cast.spell_id)
    }

    /// Find which slot contains a spell by spell_id. Returns the first match.
    pub fn find_spell_slot(&self, spell_id: u32) -> Option<CurrentSpellType> {
        for slot in [
            CurrentSpellType::Generic,
            CurrentSpellType::Channeled,
            CurrentSpellType::Melee,
            CurrentSpellType::Autorepeat,
        ] {
            if let Some(ref cast) = self.current_spells[slot as usize] {
                if cast.spell_id == spell_id {
                    return Some(slot);
                }
            }
        }
        None
    }

    /// Find an active cast by spell_id across all slots (MaNGOS SpellCaster::FindCurrentSpellBySpellId).
    pub fn find_spell_by_id(&self, spell_id: u32) -> Option<&ActiveCast> {
        for slot in [
            CurrentSpellType::Generic,
            CurrentSpellType::Channeled,
            CurrentSpellType::Melee,
            CurrentSpellType::Autorepeat,
        ] {
            if let Some(ref cast) = self.current_spells[slot as usize] {
                if cast.spell_id == spell_id {
                    return Some(cast);
                }
            }
        }
        None
    }
}

// === Target Flags (from CMSG_CAST_SPELL) ===
// Matching MaNGOS TARGET_FLAG_* values

/// No explicit target (self-cast)
pub const TARGET_FLAG_SELF: u32 = 0x0000;
/// Unit target (packed GUID follows)
pub const TARGET_FLAG_UNIT: u32 = 0x0002;
/// Item target (packed GUID follows)
pub const TARGET_FLAG_ITEM: u32 = 0x0010;
/// Source location (packed GUID + 3 floats follow)
pub const TARGET_FLAG_SOURCE_LOCATION: u32 = 0x0020;
/// Destination location (packed GUID + 3 floats follow)
pub const TARGET_FLAG_DEST_LOCATION: u32 = 0x0040;
/// GameObject target (packed GUID follows)
pub const TARGET_FLAG_OBJECT: u32 = 0x0800;
/// Trade item target (packed GUID follows)
pub const TARGET_FLAG_TRADE_ITEM: u32 = 0x1000;
/// String target
pub const TARGET_FLAG_STRING: u32 = 0x2000;
/// Corpse target (packed GUID follows)
pub const TARGET_FLAG_CORPSE: u32 = 0x8000;
/// Non-combat pet / dynamic unit target (packed GUID follows)
pub const TARGET_FLAG_UNK2: u32 = 0x0100;
/// PVP corpse / enemy corpse target (packed GUID follows)
pub const TARGET_FLAG_PVP_CORPSE: u32 = 0x0200;
pub const TARGET_FLAG_CORPSE_ENEMY: u32 = 0x0200;
/// Object UNK (packed GUID follows)
pub const TARGET_FLAG_OBJECT_UNK: u32 = 0x0080;
/// Mini-pet unit target (packed GUID follows)
pub const TARGET_FLAG_UNIT_MINIPET: u32 = 0x0400;
/// Locked object (GO or item with a lock) target
pub const TARGET_FLAG_LOCKED: u32 = 0x4000;
/// Friendly corpse target (ally resurrection)
pub const TARGET_FLAG_CORPSE_ALLY: u32 = 0x8000;

/// Parsed spell cast targets from the client packet.
///
/// Contains the target mask and resolved target data sent by the client
/// in CMSG_CAST_SPELL. Follows the MaNGOS SpellCastTargets format.
#[derive(Debug, Clone, Default)]
pub struct SpellCastTargets {
    /// Bitmask of TARGET_FLAG_* values indicating which fields are present
    pub target_flags: u32,
    /// Unit target GUID (when TARGET_FLAG_UNIT is set)
    pub unit_target_guid: Option<ObjectGuid>,
    /// GameObject target GUID (when TARGET_FLAG_OBJECT is set)
    pub gameobject_target_guid: Option<ObjectGuid>,
    /// Item target GUID (when TARGET_FLAG_ITEM or TARGET_FLAG_TRADE_ITEM is set)
    pub item_target_guid: Option<ObjectGuid>,
    /// Corpse target GUID (when TARGET_FLAG_CORPSE or TARGET_FLAG_PVP_CORPSE is set)
    pub corpse_target_guid: Option<ObjectGuid>,
    /// Source location (when TARGET_FLAG_SOURCE_LOCATION is set)
    pub src_position: Option<(f32, f32, f32)>,
    /// Destination location (when TARGET_FLAG_DEST_LOCATION is set)
    pub dst_position: Option<(f32, f32, f32)>,
    /// String target (when TARGET_FLAG_STRING is set)
    pub str_target: Option<String>,
}

impl SpellCastTargets {
    /// Get the primary unit target GUID (convenience for the common case)
    pub fn unit_target(&self) -> Option<ObjectGuid> {
        self.unit_target_guid
    }
}

/// An in-progress spell cast.
///
/// Created when the player starts casting, updated every tick,
/// consumed when the cast completes or is cancelled.
#[derive(Debug, Clone)]
pub struct ActiveCast {
    /// Spell ID being cast
    pub spell_id: u32,

    /// Target GUID (None for self-cast or AoE without explicit target)
    pub target_guid: Option<ObjectGuid>,

    /// Current state of this cast (matches MaNGOS SpellState)
    pub state: SpellState,

    /// Which slot this cast occupies
    pub slot: CurrentSpellType,

    /// Remaining cast time in milliseconds.
    /// When this reaches 0, the spell fires.
    /// Instant spells never create an ActiveCast (they execute immediately).
    pub cast_time_remaining_ms: u32,

    /// Original cast time (for cast bar display and pushback cap)
    pub original_cast_time_ms: u32,

    /// Total pushback accumulated from damage (capped at original_cast_time * some factor)
    pub total_pushback_ms: u32,

    /// Whether this is a channeled spell
    pub is_channeling: bool,

    /// For channeled spells: remaining channel ticks
    pub channel_ticks_remaining: u32,

    /// For channeled spells: time until next channel tick
    pub channel_tick_timer_ms: u32,

    /// For channeled spells: interval between ticks
    pub channel_tick_interval_ms: u32,

    /// Whether this cast was triggered (by a proc, item, etc.)
    /// Triggered casts bypass GCD and some validation checks.
    pub is_triggered: bool,

    /// Whether this cast originated from a client request.
    pub is_client_started: bool,

    /// Caster position at cast start (for movement interrupt check)
    pub start_position_x: f32,
    pub start_position_y: f32,
    pub start_position_z: f32,

    /// Full client-provided targets (unit/GO/item/corpse + source/dest positions).
    /// Carried so ground-targeted AoE and item/GO effects keep their data when the cast
    /// resolves later (cast-time / channel / delayed projectile).
    pub cast_targets: SpellCastTargets,
}

impl ActiveCast {
    /// Create a new active cast for a non-channeled spell.
    pub fn new(
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
        cast_time_ms: u32,
        is_triggered: bool,
        slot: CurrentSpellType,
        x: f32,
        y: f32,
        z: f32,
    ) -> Self {
        Self {
            spell_id,
            target_guid,
            state: SpellState::Preparing,
            slot,
            cast_time_remaining_ms: cast_time_ms,
            original_cast_time_ms: cast_time_ms,
            total_pushback_ms: 0,
            is_channeling: false,
            channel_ticks_remaining: 0,
            channel_tick_timer_ms: 0,
            channel_tick_interval_ms: 0,
            is_triggered,
            is_client_started: false,
            start_position_x: x,
            start_position_y: y,
            start_position_z: z,
            cast_targets: SpellCastTargets::default(),
        }
    }

    /// Create an active cast for a channeled spell.
    pub fn new_channel(
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
        total_duration_ms: u32,
        tick_count: u32,
        is_triggered: bool,
        x: f32,
        y: f32,
        z: f32,
    ) -> Self {
        let tick_interval = if tick_count > 0 {
            total_duration_ms / tick_count
        } else {
            total_duration_ms
        };

        Self {
            spell_id,
            target_guid,
            state: SpellState::Casting,
            slot: CurrentSpellType::Channeled,
            cast_time_remaining_ms: total_duration_ms,
            original_cast_time_ms: total_duration_ms,
            total_pushback_ms: 0,
            is_channeling: true,
            channel_ticks_remaining: tick_count,
            channel_tick_timer_ms: tick_interval,
            channel_tick_interval_ms: tick_interval,
            is_triggered,
            is_client_started: false,
            start_position_x: x,
            start_position_y: y,
            start_position_z: z,
            cast_targets: SpellCastTargets::default(),
        }
    }

    pub fn set_client_started(&mut self, is_client_started: bool) {
        self.is_client_started = is_client_started;
    }

    /// Apply cast time pushback from taking damage
    /// Returns the amount of pushback applied
    pub fn apply_pushback(&mut self, amount_ms: u32, max_total_pushback_ms: u32) -> u32 {
        if self.is_channeling {
            // Channeled: lose percentage of remaining time
            let loss = self.cast_time_remaining_ms / 4;
            self.cast_time_remaining_ms = self.cast_time_remaining_ms.saturating_sub(loss);
            loss
        } else {
            // Non-channeled: add to remaining time, capped
            let remaining_pushback = max_total_pushback_ms.saturating_sub(self.total_pushback_ms);
            let actual_pushback = amount_ms.min(remaining_pushback);
            self.cast_time_remaining_ms += actual_pushback;
            self.total_pushback_ms += actual_pushback;
            actual_pushback
        }
    }

    /// Check if the cast has moved too far from start position (for movement interrupt)
    pub fn has_moved_too_far(&self, current_x: f32, current_y: f32, current_z: f32) -> bool {
        let dx = current_x - self.start_position_x;
        let dy = current_y - self.start_position_y;
        let dz = current_z - self.start_position_z;
        let distance_squared = dx * dx + dy * dy + dz * dz;
        // Allow small movement (0.5 yards squared = 0.25)
        distance_squared > 0.25
    }

    /// Tick the cast timer by delta time
    /// Returns true if the cast is complete
    pub fn tick(&mut self, delta_ms: u32) -> bool {
        if self.cast_time_remaining_ms <= delta_ms {
            self.cast_time_remaining_ms = 0;
            true
        } else {
            self.cast_time_remaining_ms -= delta_ms;
            false
        }
    }

    /// Tick the channel timer
    /// Returns Some(true) if channel tick should fire, Some(false) if just decrementing
    /// Returns None if channel is complete
    pub fn tick_channel(&mut self, delta_ms: u32) -> Option<bool> {
        // Check for channel tick
        let should_tick = if self.channel_tick_timer_ms <= delta_ms {
            self.channel_tick_timer_ms = self.channel_tick_interval_ms;
            if self.channel_ticks_remaining > 0 {
                self.channel_ticks_remaining -= 1;
            }
            true
        } else {
            self.channel_tick_timer_ms -= delta_ms;
            false
        };

        // Check for channel completion
        if self.cast_time_remaining_ms <= delta_ms {
            self.cast_time_remaining_ms = 0;
            None
        } else {
            self.cast_time_remaining_ms -= delta_ms;
            Some(should_tick)
        }
    }
}

/// A spell modifier from a talent or aura.
///
/// Spell modifiers change properties of spells the player casts.
/// They are applied during spell calculation, NOT to the spell DBC data itself.
///
/// Example: Talent "Ice Shards" adds +100% crit bonus damage to Frost spells.
/// This creates a SpellMod { op: CritDamageBonus, value: 100, ... }
#[derive(Debug, Clone)]
pub struct SpellMod {
    /// What property this modifier affects
    pub op: SpellModOp,

    /// Modifier type (flat or percentage)
    pub mod_type: SpellModType,

    /// Modifier value.
    /// For flat: added to the property (e.g., +50 damage)
    /// For pct: multiplier in percentage points (e.g., 10 = +10%)
    pub value: i32,

    /// Spell mask that determines which spells this modifier applies to.
    /// Uses spell_family_flags from the talent/aura's spell entry.
    pub spell_family_mask: u64,

    /// Spell family this modifier applies to (must match spell's spell_family_name)
    pub spell_family_name: u32,

    /// Source spell ID (the talent or aura providing this modifier)
    pub source_spell_id: u32,

    /// Source aura slot (for removal when aura expires)
    pub source_aura_slot: Option<u8>,
}

/// What property a spell modifier affects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SpellModOp {
    Damage = 0,
    Duration = 1,
    Threat = 2,
    Effect1 = 3,
    Charges = 4,
    Range = 5,
    Radius = 6,
    CritChance = 7,
    AllEffects = 8,
    NotLoseCastTime = 9,
    CastTime = 10,
    Cooldown = 11,
    Effect2 = 12,
    IgnoreArmor = 13,
    Cost = 14,
    CritDamageBonus = 15,
    ResistMissChance = 16,
    JumpTargets = 17,
    ChanceOfSuccess = 18,
    ActivationTime = 19,
    EffectPastFirst = 20,
    GlobalCooldown = 21,
    Dot = 22,
    Effect3 = 23,
}

impl SpellModOp {
    /// Convert from u32 (from DBC effect_misc_value)
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(SpellModOp::Damage),
            1 => Some(SpellModOp::Duration),
            2 => Some(SpellModOp::Threat),
            3 => Some(SpellModOp::Effect1),
            4 => Some(SpellModOp::Charges),
            5 => Some(SpellModOp::Range),
            6 => Some(SpellModOp::Radius),
            7 => Some(SpellModOp::CritChance),
            8 => Some(SpellModOp::AllEffects),
            9 => Some(SpellModOp::NotLoseCastTime),
            10 => Some(SpellModOp::CastTime),
            11 => Some(SpellModOp::Cooldown),
            12 => Some(SpellModOp::Effect2),
            13 => Some(SpellModOp::IgnoreArmor),
            14 => Some(SpellModOp::Cost),
            15 => Some(SpellModOp::CritDamageBonus),
            16 => Some(SpellModOp::ResistMissChance),
            17 => Some(SpellModOp::JumpTargets),
            18 => Some(SpellModOp::ChanceOfSuccess),
            19 => Some(SpellModOp::ActivationTime),
            20 => Some(SpellModOp::EffectPastFirst),
            21 => Some(SpellModOp::GlobalCooldown),
            22 => Some(SpellModOp::Dot),
            23 => Some(SpellModOp::Effect3),
            _ => None,
        }
    }
}

/// Type of spell modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellModType {
    /// Flat value added to the property
    Flat,
    /// Percentage modifier (value is in percentage points, e.g., 10 = +10%)
    Pct,
}

/// Spell cast result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellCastError {
    None,
    CasterDead,
    NotEnoughMana,
    NotEnoughRage,
    NotEnoughEnergy,
    SpellOnCooldown,
    NotReady, // GCD active
    SpellNotKnown,
    InvalidTarget,
    TargetOutOfRange,
    TargetNotInLineOfSight,
    NotWhileMoving,
    Stunned,
    Silenced,
    Pacified,
    Confused,
    Fleeing,
    SchoolLockout,
    AlreadyCasting,
    Interrupted,
    /// Wrong shapeshift form (e.g., spell requires Bear Form but in Cat Form)
    WrongShapeshift,
    /// Required aura state not met (e.g., Execute requires target < 20% HP)
    CasterAuraState,
    /// Target does not meet aura state requirement
    TargetAuraState,
    /// Rogue/Druid tried to use a finishing move with no combo points on the target
    NoComboPoints,
    /// Silent failure — cast silently rejected, client shows no error message
    DontReport,
    /// Target or destination is out of spell range
    OutOfRange,
    /// Target is too close (minimum range not met)
    TooClose,
    /// Caster is not facing the target (SPELL_CUSTOM_FACE_TARGET or combat-range spells)
    NotInFront,
    /// Skill/level too low to open the lock or use the ability (SPELL_FAILED_LOW_CASTLEVEL)
    LowCastLevel,
    // ── CheckCast / CheckPetCast errors ───────────────────────────────────
    NotStanding,
    AffectingCombat,
    OnlyStealthed,
    OnlyOutdoors,
    OnlyIndoors,
    AuraBounced,
    TargetNotDead,
    HighLevel,
    TargetAffectingCombat,
    CantCastOnTapped,
    TargetIsPlayer,
    NotBehind,
    NotOnTaxi,
    NoPet,
    TargetsDead,
    DamageImmune,
    AlreadyBeingTamed,
    // ── Item check errors (CheckItems) ─────────────────────────────────────
    AlreadyAtFullHealth,
    AlreadyAtFullPower,
    CantBeDisenchanted,
    EquippedItem,
    EquippedItemClass,
    EquippedItemClassMainhand,
    EquippedItemClassOffhand,
    Error,
    ItemGone,
    ItemNotReady,
    LowLevel,
    MainhandEmpty,
    NeedExoticAmmo,
    NoAmmo,
    NoChargesRemain,
    NotTradeable,
    RequiresSpellFocus,
    TargetNotPlayer,
}

/// Result of a spell cast attempt
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellCastResult {
    Success,
    Failed(SpellCastError),
}

impl SpellCastResult {
    pub fn is_success(&self) -> bool {
        matches!(self, SpellCastResult::Success)
    }

    pub fn is_failure(&self) -> bool {
        !self.is_success()
    }

    pub fn error(&self) -> SpellCastError {
        match self {
            SpellCastResult::Success => SpellCastError::None,
            SpellCastResult::Failed(e) => *e,
        }
    }
}

/// A delayed spell effect awaiting delivery (projectile travel time).
///
/// Created when a spell has `speed > 0` in DBC. The spell's SMSG_SPELL_GO
/// is sent immediately with cast flags, but effects land after travel time.
#[derive(Debug, Clone)]
pub struct DelayedSpellEffect {
    /// Spell ID
    pub spell_id: u32,
    /// Caster GUID
    pub caster_guid: ObjectGuid,
    /// Target GUID
    pub target_guid: Option<ObjectGuid>,
    /// Time remaining until effects land (ms)
    pub delivery_time_ms: u32,
    /// Whether this is a triggered spell
    pub is_triggered: bool,
}

/// Spell school enumeration
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellSchool {
    Physical = 0,
    Holy = 1,
    Fire = 2,
    Nature = 3,
    Frost = 4,
    Shadow = 5,
    Arcane = 6,
}

// =============================================================================
// Spell Event Queue
// =============================================================================

/// A unique ID for each spell event (for removal on cancel/interrupt).
pub type SpellEventId = u64;

/// What a spell event does when it fires.
#[derive(Debug, Clone)]
pub enum SpellEventType {
    /// Cast timer completed — execute spell effects and finish cast
    CastFinish {
        caster_guid: ObjectGuid,
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
        is_triggered: bool,
        slot: CurrentSpellType,
        cast_item_guid: Option<ObjectGuid>,
        cast_targets: SpellCastTargets,
    },
    /// Channel tick — execute one channel tick
    ChannelTick {
        caster_guid: ObjectGuid,
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
        tick_number: u32,
        cast_targets: SpellCastTargets,
    },
    /// Channel complete — finish the channel
    ChannelFinish {
        caster_guid: ObjectGuid,
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
        cast_targets: SpellCastTargets,
    },
    /// Delayed spell effect — projectile arrived
    DelayedEffect {
        caster_guid: ObjectGuid,
        spell_id: u32,
        target_guid: Option<ObjectGuid>,
        is_triggered: bool,
        cast_targets: SpellCastTargets,
    },
    /// Delayed proc resolution — run after the current spell event batch.
    PendingProc {
        caster_guid: ObjectGuid,
        spell_id: u32,
        is_triggered: bool,
    },
}

/// A single scheduled spell event.
#[derive(Debug, Clone)]
pub struct SpellEvent {
    /// Unique ID (for cancellation)
    pub id: SpellEventId,
    /// When this event fires (game time ms)
    pub fire_time_ms: u64,
    /// What to do
    pub event_type: SpellEventType,
}

/// Event-driven spell queue. Events are sorted by fire time.
/// Replaces the per-player polling loop for spell updates.
#[derive(Debug)]
pub struct SpellEventQueue {
    /// Events keyed by fire time for efficient drain
    events: BTreeMap<u64, Vec<SpellEvent>>,
    /// Next unique event ID
    next_id: SpellEventId,
}

impl SpellEventQueue {
    pub fn new() -> Self {
        Self {
            events: BTreeMap::new(),
            next_id: 1,
        }
    }

    /// Schedule a new event. Returns the event ID (for cancellation).
    pub fn schedule(&mut self, fire_time_ms: u64, event_type: SpellEventType) -> SpellEventId {
        let id = self.next_id;
        self.next_id += 1;
        let event = SpellEvent {
            id,
            fire_time_ms,
            event_type,
        };
        self.events.entry(fire_time_ms).or_default().push(event);
        id
    }

    /// Drain all events that should fire at or before `now_ms`.
    pub fn drain_ready(&mut self, now_ms: u64) -> Vec<SpellEvent> {
        let mut ready = Vec::new();

        // Collect all keys <= now_ms
        let keys: Vec<u64> = self.events.range(..=now_ms).map(|(&k, _)| k).collect();
        for key in keys {
            if let Some(events) = self.events.remove(&key) {
                ready.extend(events);
            }
        }

        ready
    }

    /// Remove all events for a specific caster + spell (on cancel/interrupt).
    pub fn cancel_events_for(&mut self, caster_guid: ObjectGuid, spell_id: u32) {
        for events in self.events.values_mut() {
            events.retain(|e| match &e.event_type {
                SpellEventType::CastFinish {
                    caster_guid: c,
                    spell_id: s,
                    ..
                }
                | SpellEventType::ChannelTick {
                    caster_guid: c,
                    spell_id: s,
                    ..
                }
                | SpellEventType::ChannelFinish {
                    caster_guid: c,
                    spell_id: s,
                    ..
                }
                | SpellEventType::DelayedEffect {
                    caster_guid: c,
                    spell_id: s,
                    ..
                }
                | SpellEventType::PendingProc {
                    caster_guid: c,
                    spell_id: s,
                    ..
                } => !(*c == caster_guid && *s == spell_id),
            });
        }
        // Clean up empty entries
        self.events.retain(|_, v| !v.is_empty());
    }

    /// Remove a specific event by ID. Returns true if found and removed.
    pub fn cancel_event(&mut self, event_id: SpellEventId) -> bool {
        for events in self.events.values_mut() {
            if let Some(pos) = events.iter().position(|e| e.id == event_id) {
                events.remove(pos);
                return true;
            }
        }
        false
    }

    /// Reschedule an event (for pushback). Removes old, inserts at new time.
    pub fn reschedule(&mut self, event_id: SpellEventId, new_fire_time_ms: u64) -> bool {
        // Find and remove the event
        let mut found_event: Option<SpellEvent> = None;
        for events in self.events.values_mut() {
            if let Some(pos) = events.iter().position(|e| e.id == event_id) {
                found_event = Some(events.remove(pos));
                break;
            }
        }
        // Clean up empty entries
        self.events.retain(|_, v| !v.is_empty());

        if let Some(mut event) = found_event {
            event.fire_time_ms = new_fire_time_ms;
            self.events.entry(new_fire_time_ms).or_default().push(event);
            true
        } else {
            false
        }
    }

    /// Check if queue is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxcore_shared::protocol::ObjectGuid;

    fn player(id: u32) -> ObjectGuid {
        ObjectGuid::new_player(id)
    }

    fn cast_finish(caster: ObjectGuid, spell_id: u32) -> SpellEventType {
        SpellEventType::CastFinish {
            caster_guid: caster,
            spell_id,
            target_guid: None,
            is_triggered: false,
            slot: CurrentSpellType::Generic,
            cast_item_guid: None,
            cast_targets: SpellCastTargets::default(),
        }
    }

    fn channel_tick(caster: ObjectGuid, spell_id: u32) -> SpellEventType {
        SpellEventType::ChannelTick {
            caster_guid: caster,
            spell_id,
            target_guid: None,
            tick_number: 1,
            cast_targets: SpellCastTargets::default(),
        }
    }

    fn pending_proc(caster: ObjectGuid, spell_id: u32) -> SpellEventType {
        SpellEventType::PendingProc {
            caster_guid: caster,
            spell_id,
            is_triggered: false,
        }
    }

    // === SpellEventQueue ===

    #[test]
    fn test_schedule_and_drain_ready() {
        let mut q = SpellEventQueue::new();
        q.schedule(100, cast_finish(player(1), 1));

        assert!(q.drain_ready(50).is_empty());
        let ready = q.drain_ready(100);
        assert_eq!(ready.len(), 1);
        assert!(q.is_empty());
    }

    #[test]
    fn test_drain_does_not_double_fire() {
        let mut q = SpellEventQueue::new();
        q.schedule(100, cast_finish(player(1), 1));
        q.drain_ready(100);
        assert!(q.drain_ready(200).is_empty());
    }

    #[test]
    fn test_multiple_events_same_timestamp() {
        let mut q = SpellEventQueue::new();
        q.schedule(100, cast_finish(player(1), 1));
        q.schedule(100, cast_finish(player(2), 2));
        q.schedule(100, cast_finish(player(3), 3));

        let ready = q.drain_ready(100);
        assert_eq!(ready.len(), 3);
        assert!(q.is_empty());
    }

    #[test]
    fn test_cancel_events_for_removes_matching() {
        let mut q = SpellEventQueue::new();
        let a = player(1);
        q.schedule(100, cast_finish(a, 10));
        q.schedule(200, cast_finish(a, 10));
        q.schedule(300, cast_finish(a, 20)); // different spell — should survive

        q.cancel_events_for(a, 10);

        let all = q.drain_ready(1000);
        assert_eq!(all.len(), 1);
        match &all[0].event_type {
            SpellEventType::CastFinish { spell_id, .. } => assert_eq!(*spell_id, 20),
            _ => panic!("wrong event type"),
        }
    }

    #[test]
    fn test_cancel_events_for_different_caster_not_removed() {
        let mut q = SpellEventQueue::new();
        let a = player(1);
        let b = player(2);
        q.schedule(100, cast_finish(a, 10));
        q.schedule(200, cast_finish(b, 10)); // same spell, different caster

        q.cancel_events_for(a, 10);

        let all = q.drain_ready(1000);
        assert_eq!(all.len(), 1);
        match &all[0].event_type {
            SpellEventType::CastFinish { caster_guid, .. } => assert_eq!(*caster_guid, b),
            _ => panic!("wrong event type"),
        }
    }

    #[test]
    fn test_cancel_events_for_all_event_types() {
        let mut q = SpellEventQueue::new();
        let a = player(1);
        q.schedule(100, cast_finish(a, 5));
        q.schedule(200, channel_tick(a, 5));
        q.schedule(
            300,
            SpellEventType::ChannelFinish {
                caster_guid: a,
                spell_id: 5,
                target_guid: None,
                cast_targets: SpellCastTargets::default(),
            },
        );
        q.schedule(
            400,
            SpellEventType::DelayedEffect {
                caster_guid: a,
                spell_id: 5,
                target_guid: None,
                is_triggered: false,
                cast_targets: SpellCastTargets::default(),
            },
        );
        q.schedule(500, pending_proc(a, 5));

        q.cancel_events_for(a, 5);
        assert!(q.is_empty());
    }

    #[test]
    fn test_cancel_event_by_id() {
        let mut q = SpellEventQueue::new();
        let id = q.schedule(100, cast_finish(player(1), 1));
        q.schedule(100, cast_finish(player(2), 2)); // second event, same time

        assert!(q.cancel_event(id));
        let ready = q.drain_ready(100);
        assert_eq!(ready.len(), 1);
        match &ready[0].event_type {
            SpellEventType::CastFinish { caster_guid, .. } => {
                assert_eq!(*caster_guid, player(2))
            }
            _ => panic!("wrong event type"),
        }
    }

    #[test]
    fn test_cancel_event_nonexistent_id_returns_false() {
        let mut q = SpellEventQueue::new();
        assert!(!q.cancel_event(999));
    }

    #[test]
    fn test_reschedule_moves_event_to_later_time() {
        let mut q = SpellEventQueue::new();
        let id = q.schedule(100, cast_finish(player(1), 1));

        assert!(q.reschedule(id, 200));
        assert!(q.drain_ready(100).is_empty());
        let ready = q.drain_ready(200);
        assert_eq!(ready.len(), 1);
    }

    #[test]
    fn test_reschedule_nonexistent_id_returns_false() {
        let mut q = SpellEventQueue::new();
        assert!(!q.reschedule(999, 500));
    }

    // === ActiveCast ===

    #[test]
    fn test_active_cast_tick_completes_at_zero() {
        let mut cast = ActiveCast::new(
            1,
            None,
            1000,
            false,
            CurrentSpellType::Generic,
            0.0,
            0.0,
            0.0,
        );
        assert!(!cast.tick(999));
        assert!(cast.tick(1)); // remaining was 1, subtract 1 → 0
    }

    #[test]
    fn test_active_cast_tick_overshoots_completes() {
        let mut cast = ActiveCast::new(
            1,
            None,
            500,
            false,
            CurrentSpellType::Generic,
            0.0,
            0.0,
            0.0,
        );
        // delta > remaining
        assert!(cast.tick(1000));
        assert_eq!(cast.cast_time_remaining_ms, 0);
    }

    #[test]
    fn test_set_client_started_assigns_flag_only() {
        let mut cast = ActiveCast::new(
            1,
            None,
            500,
            false,
            CurrentSpellType::Generic,
            0.0,
            0.0,
            0.0,
        );
        let remaining = cast.cast_time_remaining_ms;
        let state = cast.state;

        assert!(!cast.is_client_started);
        cast.set_client_started(true);
        assert!(cast.is_client_started);
        cast.set_client_started(false);
        assert!(!cast.is_client_started);
        assert_eq!(cast.cast_time_remaining_ms, remaining);
        assert_eq!(cast.state, state);
    }

    #[test]
    fn test_pushback_non_channeled_capped_at_max() {
        let mut cast = ActiveCast::new(
            1,
            None,
            2000,
            false,
            CurrentSpellType::Generic,
            0.0,
            0.0,
            0.0,
        );
        let applied1 = cast.apply_pushback(500, 1000);
        let applied2 = cast.apply_pushback(500, 1000);
        let applied3 = cast.apply_pushback(500, 1000); // already at cap
        assert_eq!(applied1, 500);
        assert_eq!(applied2, 500);
        assert_eq!(applied3, 0); // cap reached
        assert_eq!(cast.total_pushback_ms, 1000);
    }

    #[test]
    fn test_pushback_channeled_loses_25_pct() {
        let mut cast = ActiveCast::new_channel(1, None, 4000, 4, false, 0.0, 0.0, 0.0);
        let remaining_before = cast.cast_time_remaining_ms;
        let loss = cast.apply_pushback(0, 0); // amount_ms ignored for channeled
        assert_eq!(loss, remaining_before / 4);
    }

    #[test]
    fn test_has_moved_too_far_within_threshold() {
        let cast = ActiveCast::new(
            1,
            None,
            1000,
            false,
            CurrentSpellType::Generic,
            0.0,
            0.0,
            0.0,
        );
        assert!(!cast.has_moved_too_far(0.4, 0.0, 0.0)); // 0.16 < 0.25
    }

    #[test]
    fn test_has_moved_too_far_exceeds_threshold() {
        let cast = ActiveCast::new(
            1,
            None,
            1000,
            false,
            CurrentSpellType::Generic,
            0.0,
            0.0,
            0.0,
        );
        assert!(cast.has_moved_too_far(1.0, 0.0, 0.0)); // 1.0 > 0.25
    }

    // === SpellsState ===

    #[test]
    fn test_cooldown_not_set_returns_false() {
        let state = SpellsState::default();
        assert!(!state.is_on_cooldown(100, 1000));
    }

    #[test]
    fn test_cooldown_active_returns_true() {
        let mut state = SpellsState::default();
        state.add_cooldown(100, 5000, 0); // expires at 5000ms
        assert!(state.is_on_cooldown(100, 1000));
        assert!(!state.is_on_cooldown(100, 5000)); // exactly at end = not on cd
    }

    #[test]
    fn test_gcd_active_and_expired() {
        let mut state = SpellsState::default();
        state.apply_gcd(1500, 0);
        assert!(state.is_on_gcd(500));
        assert!(!state.is_on_gcd(1500));
    }

    #[test]
    fn test_school_lockout_active_and_expired() {
        let mut state = SpellsState::default();
        state.apply_school_lockout(2 /* Fire */, 5000, 0);
        assert!(state.is_school_locked(2, 1000));
        assert!(!state.is_school_locked(2, 5000));
        assert!(!state.is_school_locked(3 /* Nature */, 1000)); // other school unaffected
    }

    #[test]
    fn test_school_lockout_only_extends_never_shortens() {
        let mut state = SpellsState::default();
        state.apply_school_lockout(1, 10000, 0); // locked until 10000
        state.apply_school_lockout(1, 3000, 0); // shorter — should not shorten
        assert!(state.is_school_locked(1, 9000));
    }

    #[test]
    fn test_cancel_preparing_cast_does_not_consume_gcd() {
        let mut state = SpellsState::default();
        // GCD is not set (cancel before GCD is applied)
        assert!(!state.is_on_gcd(1000));
    }

    #[test]
    fn test_find_spell_slot_returns_correct_slot() {
        let mut state = SpellsState::default();
        let cast = ActiveCast::new(
            42,
            None,
            1000,
            false,
            CurrentSpellType::Generic,
            0.0,
            0.0,
            0.0,
        );
        state.set_current_spell(CurrentSpellType::Generic, cast);
        assert_eq!(state.find_spell_slot(42), Some(CurrentSpellType::Generic));
        assert_eq!(state.find_spell_slot(99), None);
    }

    #[test]
    fn test_clear_current_spell_returns_and_removes() {
        let mut state = SpellsState::default();
        let cast = ActiveCast::new(
            7,
            None,
            500,
            false,
            CurrentSpellType::Generic,
            0.0,
            0.0,
            0.0,
        );
        state.set_current_spell(CurrentSpellType::Generic, cast);
        let removed = state.clear_current_spell(CurrentSpellType::Generic);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().spell_id, 7);
        assert!(!state.has_current_spell(CurrentSpellType::Generic));
    }

    #[test]
    fn test_is_casting_true_when_generic_slot_occupied() {
        let mut state = SpellsState::default();
        let cast = ActiveCast::new(
            1,
            None,
            2000,
            false,
            CurrentSpellType::Generic,
            0.0,
            0.0,
            0.0,
        );
        state.set_current_spell(CurrentSpellType::Generic, cast);
        assert!(state.is_casting());
    }

    #[test]
    fn test_is_casting_false_when_only_melee_slot_occupied() {
        let mut state = SpellsState::default();
        let cast = ActiveCast::new(1, None, 0, false, CurrentSpellType::Melee, 0.0, 0.0, 0.0);
        state.set_current_spell(CurrentSpellType::Melee, cast);
        assert!(!state.is_casting());
    }
}
