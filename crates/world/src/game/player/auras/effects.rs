//! Aura effect types and stat modifier mapping
//!
//! Aura effects fall into categories. Each category has its own handler module.

// --- Stat Modifiers ---
// These auras add/remove stat modifiers via StatsSystem.
// On apply: create StatModifier and call stats.apply_modifier()
// On remove: call stats.remove_modifier(source)

pub const AURA_MOD_STAT: u32 = 29; // Flat stat bonus (+X Strength)
pub const AURA_MOD_PERCENT_STAT: u32 = 80; // Pct stat bonus (+10% Agility)
pub const AURA_MOD_TOTAL_STAT_PERCENTAGE: u32 = 137; // Pct total stat (+10% all stats)
pub const AURA_MOD_RESISTANCE: u32 = 22; // Flat resistance (+50 Fire Resist)
pub const AURA_MOD_BASE_RESISTANCE: u32 = 83; // Base resistance (before multipliers)
pub const AURA_MOD_RESISTANCE_PCT: u32 = 101; // Pct resistance (+10% Shadow Resist)
pub const AURA_MOD_BASE_RESISTANCE_PCT: u32 = 142; // Pct base resistance
pub const AURA_MOD_RESISTANCE_OF_STAT_PERCENT: u32 = 182; // Armor % of Intellect
pub const AURA_MOD_ATTACK_POWER: u32 = 99; // Flat AP (+140 AP from Battle Shout)
pub const AURA_MOD_ATTACK_POWER_PCT: u32 = 166; // Pct AP
pub const AURA_MOD_RANGED_ATTACK_POWER: u32 = 124; // Flat RAP
pub const AURA_MOD_RANGED_ATTACK_POWER_PCT: u32 = 167; // Pct RAP
pub const AURA_MOD_DAMAGE_DONE: u32 = 13; // Flat damage bonus
pub const AURA_MOD_DAMAGE_PERCENT_DONE: u32 = 79; // Pct damage bonus
pub const AURA_MOD_HEALING_DONE: u32 = 135; // Flat healing bonus
pub const AURA_MOD_HEALING_DONE_PERCENT: u32 = 136; // Pct healing bonus
pub const AURA_MOD_HEALING_PCT: u32 = 118; // Pct healing taken
pub const AURA_MOD_CRIT_PERCENT: u32 = 52; // Flat crit% (+5% crit)
pub const AURA_MOD_SPELL_CRIT_CHANCE: u32 = 57; // Flat spell crit%
pub const AURA_MOD_HIT_CHANCE: u32 = 54; // Flat hit%
pub const AURA_MOD_SPELL_HIT_CHANCE: u32 = 55; // Flat spell hit%
pub const AURA_MOD_PARRY_PERCENT: u32 = 47; // Flat parry%
pub const AURA_MOD_DODGE_PERCENT: u32 = 49; // Flat dodge%
pub const AURA_MOD_BLOCK_PERCENT: u32 = 51; // Flat block%
pub const AURA_MOD_INCREASE_HEALTH: u32 = 34; // Flat max health
pub const AURA_MOD_INCREASE_HEALTH_PERCENT: u32 = 133; // Pct max health
pub const AURA_MOD_INCREASE_ENERGY: u32 = 35; // Flat max mana/energy
pub const AURA_MOD_INCREASE_ENERGY_PERCENT: u32 = 132; // Pct max mana/energy
pub const AURA_MOD_INCREASE_SPEED: u32 = 31; // Movement speed increase
pub const AURA_MOD_DECREASE_SPEED: u32 = 33; // Movement speed decrease (snares)
pub const AURA_MOD_INCREASE_MOUNTED_SPEED: u32 = 32; // Mounted speed
pub const AURA_MOD_MELEE_HASTE: u32 = 138; // Melee haste
pub const AURA_MOD_RANGED_HASTE: u32 = 140; // Ranged haste
pub const AURA_MOD_RANGED_AMMO_HASTE: u32 = 141; // Ranged haste for ammo-requiring weapons
pub const AURA_MOD_POWER_COST_PCT: u32 = 72; // Percentage power cost by spell school
pub const AURA_MOD_POWER_COST: u32 = 73; // Flat power cost by spell school

// --- Observation/Regen Effects (Food/Drink) ---
// These auras restore a percentage of max health/mana per tick.

pub const AURA_OBS_MOD_HEALTH: u32 = 20; // Food: % max health per tick
pub const AURA_OBS_MOD_MANA: u32 = 21; // Drink: % max mana per tick

// --- Periodic Effects ---
// These auras tick damage/healing/energize every N seconds.

pub const AURA_PERIODIC_DAMAGE: u32 = 3; // DoT (Corruption, Immolate)
pub const AURA_PERIODIC_HEAL: u32 = 8; // HoT (Renew, Rejuvenation)
pub const AURA_PERIODIC_ENERGIZE: u32 = 24; // Power restore (Innervate, Evocation)
pub const AURA_PERIODIC_LEECH: u32 = 53; // Drain Life
pub const AURA_PERIODIC_MANA_LEECH: u32 = 64; // Mana drain
pub const AURA_PERIODIC_TRIGGER_SPELL: u32 = 23; // Trigger spell every tick
pub const AURA_PERIODIC_DAMAGE_PERCENT: u32 = 89; // % health DoT
pub const AURA_PERIODIC_HEALTH_FUNNEL: u32 = 62; // Transfer caster health periodically
pub const AURA_POWER_BURN_MANA: u32 = 162; // Periodic mana burn

pub fn is_periodic_aura_type(aura_type: u32) -> bool {
    matches!(
        aura_type,
        AURA_PERIODIC_DAMAGE
            | AURA_PERIODIC_HEAL
            | AURA_PERIODIC_ENERGIZE
            | AURA_OBS_MOD_HEALTH
            | AURA_PERIODIC_LEECH
            | AURA_PERIODIC_HEALTH_FUNNEL
            | AURA_PERIODIC_MANA_LEECH
            | AURA_PERIODIC_DAMAGE_PERCENT
            | AURA_POWER_BURN_MANA
            | AURA_PERIODIC_TRIGGER_SPELL
    )
}

// --- Proc Effects ---
// These auras trigger when specific combat events occur.

pub const AURA_PROC_TRIGGER_SPELL: u32 = 42; // Cast spell on proc
pub const AURA_PROC_TRIGGER_DAMAGE: u32 = 43; // Deal damage on proc
pub const AURA_DUMMY: u32 = 4; // Dummy (custom proc per spell ID)
pub const AURA_OVERRIDE_CLASS_SCRIPTS: u32 = 112; // Class-specific script procs

// --- Crowd Control ---
pub const AURA_MOD_STUN: u32 = 12; // Stun
pub const AURA_MOD_ROOT: u32 = 26; // Root
pub const AURA_MOD_FEAR: u32 = 7; // Fear
pub const AURA_MOD_CHARM: u32 = 6; // Mind Control
pub const AURA_MOD_CONFUSE: u32 = 5; // Confusion (Polymorph movement)
pub const AURA_MOD_SILENCE: u32 = 27; // Silence
pub const AURA_MOD_PACIFY: u32 = 25; // Pacify
pub const AURA_MOD_PACIFY_SILENCE: u32 = 60; // Pacify + Silence
pub const AURA_MOD_DISARM: u32 = 67; // Disarm

// --- Absorb / Shield ---
pub const AURA_SPELL_MAGNET: u32 = 96; // Spell redirection / magnet (Grounding Totem)
pub const AURA_SCHOOL_ABSORB: u32 = 69; // Damage absorb (Power Word: Shield)
pub const AURA_MANA_SHIELD: u32 = 97; // Mana Shield
pub const AURA_DAMAGE_SHIELD: u32 = 15; // Damage reflect (Thorns)
pub const AURA_SPLIT_DAMAGE_PCT: u32 = 81; // Split damage with another unit

// --- Passive / Utility ---
pub const AURA_MOD_STEALTH: u32 = 16; // Stealth
pub const AURA_MOD_INVISIBILITY: u32 = 18; // Invisibility
pub const AURA_MOD_INVISIBILITY_DETECTION: u32 = 19; // Detect invisibility
pub const AURA_MOUNTED: u32 = 78; // Mount
pub const AURA_MOD_SHAPESHIFT: u32 = 36; // Shapeshift form
pub const AURA_WATER_BREATHING: u32 = 82; // Water breathing
pub const AURA_MOD_WATER_BREATHING: u32 = 155; // Water breathing interval multiplier
pub const AURA_WATER_WALK: u32 = 104; // Water walking
pub const AURA_FEATHER_FALL: u32 = 105; // Slow fall
pub const AURA_SAFE_FALL: u32 = 144; // Extra safe fall distance
pub const AURA_MOD_RESISTANCE_EXCLUSIVE: u32 = 143; // Exclusive resistance modifier
pub const AURA_HOVER: u32 = 106; // Hover
pub const AURA_GHOST: u32 = 95; // Ghost (dead state)
pub const AURA_FEIGN_DEATH: u32 = 66; // Feign Death

// --- Crowd Control (Charm/Possess family) ---
pub const AURA_BIND_SIGHT: u32 = 1; // Bind Sight (caster views target)
pub const AURA_MOD_POSSESS: u32 = 2; // Possess (mind control via GM/Eyes)
pub const AURA_FAR_SIGHT: u32 = 76; // Far Sight (caster views a location/object)
pub const AURA_MOD_POSSESS_PET: u32 = 128; // Possess pet
pub const AURA_AOE_CHARM: u32 = 177; // AoE charm

// --- Faction / Reaction ---
pub const AURA_FORCE_REACTION: u32 = 139; // Forces a specific reputation rank vs a faction

// --- Skill ---
pub const AURA_MOD_SKILL: u32 = 30; // Temporary skill bonus
pub const AURA_MOD_SKILL_TALENT: u32 = 98; // Permanent skill bonus (talent)

// --- Item reward on death ---
pub const AURA_CHANNEL_DEATH_ITEM: u32 = 86; // Soul Shard-style item reward on target's death

// --- Tracking (continued) ---
pub const AURA_TRACK_STEALTHED: u32 = 151; // Track Stealthed Units (Track Hidden minimap dot)
pub const AURA_UNTRACKABLE: u32 = 120; // Untrackable (hidden from tracking)

// --- Misc detection ---
pub const AURA_DETECT_AMORE: u32 = 170; // Love is in the Air trinket detection

// --- Spell Modifiers (Talents/Auras) ---
pub const AURA_ADD_FLAT_MODIFIER: u32 = 107; // Flat spell modifier (talent: -0.5s cast time)
pub const AURA_ADD_PCT_MODIFIER: u32 = 108; // Pct spell modifier (talent: +10% damage)

// --- Damage Done Modifiers ---
pub const AURA_MOD_DAMAGE_DONE_CREATURE: u32 = 59; // Flat damage done vs creature type
pub const AURA_MOD_DAMAGE_DONE_VERSUS: u32 = 168; // % damage done vs creature type
pub const AURA_MELEE_ATTACK_POWER_ATTACKER_BONUS: u32 = 165; // Melee AP bonus on attacker
pub const AURA_RANGED_ATTACK_POWER_ATTACKER_BONUS: u32 = 127; // Ranged AP bonus on attacker
pub const AURA_MOD_MELEE_ATTACK_POWER_VERSUS: u32 = 102; // Melee AP vs creature type
pub const AURA_MOD_RANGED_ATTACK_POWER_VERSUS: u32 = 131; // Ranged AP vs creature type
pub const AURA_MOD_OFFHAND_DAMAGE_PCT: u32 = 122; // Off-hand damage penalty
pub const AURA_MOD_SPELL_HEALING_OF_STAT_PERCENT: u32 = 175; // Heal bonus % of stat (spirit)
pub const AURA_MOD_SPELL_DAMAGE_OF_STAT_PERCENT: u32 = 173; // Spell damage bonus % of stat
pub const AURA_MOD_FLAT_SPELL_DAMAGE_VERSUS: u32 = 171; // Flat spell damage vs creature type

// --- Damage Taken Modifiers ---
pub const AURA_MOD_DAMAGE_PERCENT_TAKEN: u32 = 87; // % damage taken modifier (e.g., Defensive Stance)
pub const AURA_MOD_CASTING_SPEED_NOT_STACK: u32 = 65; // Casting speed (haste, non-stacking)

// --- Threat ---
pub const AURA_MOD_THREAT: u32 = 10; // Threat multiplier
pub const AURA_MOD_TAUNT: u32 = 11; // Taunt aura

// --- Tracking ---
pub const AURA_TRACK_CREATURES: u32 = 44; // Track Beasts/Humanoids/etc on minimap
pub const AURA_TRACK_RESOURCES: u32 = 45; // Track Herbs/Mining on minimap
pub const AURA_MOD_STALKED: u32 = 68; // Hunter's Mark tracking

// --- Immunities ---
pub const AURA_EFFECT_IMMUNITY: u32 = 37; // Immune to specific effect
pub const AURA_STATE_IMMUNITY: u32 = 38; // Immune to specific state
pub const AURA_SCHOOL_IMMUNITY: u32 = 39; // Immune to spell school
pub const AURA_DAMAGE_IMMUNITY: u32 = 40; // Immune to damage school
pub const AURA_MECHANIC_IMMUNITY: u32 = 77; // Immune to mechanic (e.g., stun immunity)
pub const AURA_MOD_MECHANIC_RESISTANCE: u32 = 117; // Resist mechanic chance

// --- Visual / Transform ---
pub const AURA_MOD_SCALE: u32 = 61; // Size change
pub const AURA_TRANSFORM: u32 = 56; // Polymorph visual model change
pub const AURA_MOD_UNATTACKABLE: u32 = 93; // Vanish / Ice Block unattackable

// --- Power Regen ---
pub const AURA_MOD_REGEN: u32 = 84; // Health regen
pub const AURA_MOD_POWER_REGEN: u32 = 85; // Power regen (MP5)
pub const AURA_MOD_HEALTH_REGEN_PERCENT: u32 = 88; // Pct health regen
pub const AURA_MOD_POWER_REGEN_PERCENT: u32 = 110; // Pct power regen
pub const AURA_MOD_MANA_REGEN_INTERRUPT: u32 = 134; // Mana regen while casting (Meditation)

// --- Shield Block ---
pub const AURA_MOD_SHIELD_BLOCKVALUE: u32 = 158; // Flat shield block value
pub const AURA_MOD_SHIELD_BLOCKVALUE_PCT: u32 = 150; // Pct shield block value

// --- Combo Points ---
pub const AURA_RETAIN_COMBO_POINTS: u32 = 148; // Retain combo points on aura expire

// --- Misc / Others ---
pub const AURA_REFLECT_SPELLS: u32 = 28; // Flat chance to reflect any reflectable spell
pub const AURA_REFLECT_SPELLS_SCHOOL: u32 = 74; // Reflect chance -> SPELLMOD_RESIST_MISS_CHANCE
pub const AURA_EMPATHY: u32 = 121; // Beast/Humanoid inspect info (UNIT_DYNFLAG_SPECIALINFO)
pub const UNIT_DYNFLAG_SPECIALINFO: u32 = 0x0010;
pub const AURA_SPIRIT_OF_REDEMPTION: u32 = 176; // Priest: die at aura end, act as spirit healer
pub const AURA_AURA_SPELL: u32 = 192; // Apply/remove passive aura on target while active

/// Check if an aura type is a spell modifier (ADD_FLAT_MODIFIER / ADD_PCT_MODIFIER).
/// These create SpellMod entries in `player.spells.spell_modifiers` instead of stat modifiers.
pub fn is_spell_modifier_aura(aura_type: u32) -> bool {
    matches!(aura_type, AURA_ADD_FLAT_MODIFIER | AURA_ADD_PCT_MODIFIER)
}

/// Check if an aura type is a stat modifier that requires StatsSystem recalculation.
pub fn is_stat_modifier_aura(aura_type: u32) -> bool {
    matches!(
        aura_type,
        AURA_MOD_STAT
            | AURA_MOD_PERCENT_STAT
            | AURA_MOD_TOTAL_STAT_PERCENTAGE
            | AURA_MOD_RESISTANCE
            | AURA_MOD_BASE_RESISTANCE
            | AURA_MOD_RESISTANCE_PCT
            | AURA_MOD_SHIELD_BLOCKVALUE
            | AURA_MOD_BASE_RESISTANCE_PCT
            | AURA_MOD_RESISTANCE_OF_STAT_PERCENT
            | AURA_MOD_ATTACK_POWER
            | AURA_MOD_ATTACK_POWER_PCT
            | AURA_MOD_RANGED_ATTACK_POWER
            | AURA_MOD_RANGED_ATTACK_POWER_PCT
            | AURA_MOD_DAMAGE_DONE
            | AURA_MOD_DAMAGE_PERCENT_DONE
            | AURA_MOD_OFFHAND_DAMAGE_PCT
            | AURA_MOD_HEALING_DONE
            | AURA_MOD_HEALING_DONE_PERCENT
            | AURA_MOD_CRIT_PERCENT
            | AURA_MOD_SPELL_CRIT_CHANCE
            | AURA_MOD_HIT_CHANCE
            | AURA_MOD_SPELL_HIT_CHANCE
            | AURA_MOD_PARRY_PERCENT
            | AURA_MOD_DODGE_PERCENT
            | AURA_MOD_BLOCK_PERCENT
            | AURA_MOD_INCREASE_HEALTH
            | AURA_MOD_INCREASE_HEALTH_PERCENT
            | AURA_MOD_INCREASE_ENERGY
            | AURA_MOD_INCREASE_ENERGY_PERCENT
            | AURA_MOD_MELEE_HASTE
            | AURA_MOD_RANGED_HASTE
            | AURA_MOD_RANGED_AMMO_HASTE
            | AURA_MOD_POWER_COST_PCT
            | AURA_MOD_POWER_REGEN
            | AURA_MOD_REGEN
            | AURA_MOD_HEALTH_REGEN_PERCENT
            | AURA_MOD_POWER_REGEN_PERCENT
            | AURA_MOD_MANA_REGEN_INTERRUPT
            | AURA_MOD_DAMAGE_PERCENT_TAKEN
            | AURA_MOD_CASTING_SPEED_NOT_STACK
            | AURA_MOD_INCREASE_SPEED
            | AURA_MOD_DECREASE_SPEED
            | AURA_MOD_INCREASE_MOUNTED_SPEED
    )
}

// =============================================================================
// Unit Flag Constants (for CC aura application)
// =============================================================================

pub const UNIT_FLAG_STUNNED: u32 = 0x00040000;
pub const UNIT_FLAG_CONFUSED: u32 = 0x00400000;
pub const UNIT_FLAG_FLEEING: u32 = 0x00800000;
pub const UNIT_FLAG_SILENCED: u32 = 0x00002000;
pub const UNIT_FLAG_PACIFIED: u32 = 0x00020000;
pub const UNIT_FLAG_DISABLE_MOVE: u32 = 0x00000004;
pub const UNIT_FLAG_DISARMED: u32 = 0x00200000;

/// Get the unit flag corresponding to a CC aura type.
/// Returns None if the aura type doesn't set a unit flag.
pub fn cc_aura_unit_flag(aura_type: u32) -> Option<u32> {
    match aura_type {
        AURA_MOD_STUN => Some(UNIT_FLAG_STUNNED),
        AURA_MOD_ROOT => Some(UNIT_FLAG_DISABLE_MOVE),
        AURA_MOD_FEAR => Some(UNIT_FLAG_FLEEING),
        AURA_MOD_CHARM | AURA_MOD_CONFUSE => Some(UNIT_FLAG_CONFUSED),
        AURA_MOD_SILENCE => Some(UNIT_FLAG_SILENCED),
        AURA_MOD_PACIFY => Some(UNIT_FLAG_PACIFIED),
        AURA_MOD_PACIFY_SILENCE => Some(UNIT_FLAG_PACIFIED | UNIT_FLAG_SILENCED),
        AURA_MOD_DISARM => Some(UNIT_FLAG_DISARMED),
        _ => None,
    }
}

/// Check if an aura type is a CC effect that sets unit flags.
pub fn is_cc_aura(aura_type: u32) -> bool {
    cc_aura_unit_flag(aura_type).is_some()
}

/// Stat indices for MOD_STAT effects
pub const STAT_STRENGTH: usize = 0;
pub const STAT_AGILITY: usize = 1;
pub const STAT_STAMINA: usize = 2;
pub const STAT_INTELLECT: usize = 3;
pub const STAT_SPIRIT: usize = 4;

/// Modifier source for stat modifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModifierSource {
    /// Aura from a spell
    Aura(u32),
    /// Equipment item
    Equipment(u32),
    /// Talent
    Talent(u32),
    /// Base racial/class bonus
    Base,
}

/// Stat modifier for the StatsSystem
#[derive(Debug, Clone)]
pub struct StatModifier {
    /// Source of the modifier (for removal tracking)
    pub source: ModifierSource,
    /// Which stat this modifies (0-4 for STR/AGI/STA/INT/SPI)
    pub stat: usize,
    /// Flat value modifier (+X to stat)
    pub flat_value: f32,
    /// Percentage modifier (+X% to stat, 0.1 = 10%)
    pub pct_value: f32,
}

// =============================================================================
// Visibility / Model Byte Flags (Stealth, Invisibility, Detect Amore)
// =============================================================================
//
// Mirrors `Aura::HandleModStealth`, `Aura::HandleInvisibility`,
// `Aura::HandleInvisibilityDetect`, `Aura::HandleDetectAmore` in SpellAuras.cpp.

/// UNIT_FIELD_BYTES_1 byte offset 3 (vis flag byte). Set while stealthed
/// (`Aura::HandleModStealth`, `UNIT_BYTES_1_OFFSET_VIS_FLAG`).
pub const UNIT_VIS_FLAGS_CREEP: u8 = 0x02;
pub const UNIT_VIS_FLAGS_UNTRACKABLE: u8 = 0x04;

/// PLAYER_FIELD_BYTES2 byte offset 1 (extra flags byte) bits.
pub const PLAYER_FIELD_BYTE2_DETECT_AMORE: u8 = 0x01; // SPELL_AURA_DETECT_AMORE
pub const PLAYER_FIELD_BYTE2_STEALTH: u8 = 0x20; // set while SPELL_AURA_MOD_STEALTH active
pub const PLAYER_FIELD_BYTE2_INVISIBILITY_GLOW: u8 = 0x40; // set while SPELL_AURA_MOD_INVISIBILITY active

/// Recompute the `invisibility_mask` (bit per invisibility type 0-31) from the
/// set of currently-active `SPELL_AURA_MOD_INVISIBILITY` misc values.
/// Mirrors the "recalculate value at modifier remove" loop in `Aura::HandleInvisibility`.
pub fn recompute_invisibility_mask(active_misc_values: impl Iterator<Item = i32>) -> u32 {
    let mut mask = 0u32;
    for misc in active_misc_values {
        if (0..32).contains(&misc) {
            mask |= 1 << misc;
        }
    }
    mask
}

/// Recompute the `detect_invisibility_mask` from active `SPELL_AURA_MOD_INVISIBILITY_DETECTION` auras.
/// Mirrors the same recalculation loop in `Aura::HandleInvisibilityDetect`.
pub fn recompute_detect_invisibility_mask(active_misc_values: impl Iterator<Item = i32>) -> u32 {
    recompute_invisibility_mask(active_misc_values)
}

// =============================================================================
// Shapeshift Display IDs
// =============================================================================
//
// Mirrors `GetShapeshiftDisplayInfo()` in SpellAuras.cpp. Only forms with a
// hardcoded display id are covered (the rest keep the unit's native model).
// `is_alliance` selects the alliance/horde variant where the form differs by faction.
pub const FORM_CAT: u8 = 1;
pub const FORM_TREE: u8 = 2;
pub const FORM_TRAVEL: u8 = 3;
pub const FORM_AQUA: u8 = 4;
pub const FORM_BEAR: u8 = 5;
pub const FORM_AMBIENT: u8 = 6;
pub const FORM_GHOUL: u8 = 7;
pub const FORM_DIREBEAR: u8 = 8;
pub const FORM_CREATUREBEAR: u8 = 16;
pub const FORM_GHOSTWOLF: u8 = 17;
pub const FORM_BATTLESTANCE: u8 = 18;
pub const FORM_DEFENSIVESTANCE: u8 = 19;
pub const FORM_BERSERKERSTANCE: u8 = 20;
pub const FORM_MOONKIN: u8 = 27;
pub const FORM_SPIRITOFREDEMPTION: u8 = 28;

/// Returns `(display_id, scale)` for a shapeshift form, or `(0, 1.0)` if the
/// form has no hardcoded display (keeps native model). `is_alliance` picks the
/// faction-specific model variant for forms that differ by team (Cat/Bear/Moonkin/Ghoul).
pub fn shapeshift_display_info(form: u8, is_alliance: bool) -> (u32, f32) {
    match form {
        FORM_CAT => (if is_alliance { 892 } else { 8571 }, 0.80),
        FORM_TRAVEL => (632, 0.80),
        FORM_AQUA => (2428, 0.80),
        FORM_BEAR | FORM_DIREBEAR => (if is_alliance { 2281 } else { 2289 }, 1.0),
        FORM_GHOUL => {
            if is_alliance {
                (10045, 1.0)
            } else {
                (0, 1.0)
            }
        }
        FORM_GHOSTWOLF => (4613, 0.80),
        FORM_MOONKIN => (if is_alliance { 15374 } else { 15375 }, 1.0),
        FORM_TREE => (864, 1.0),
        FORM_SPIRITOFREDEMPTION => (16031, 1.0),
        _ => (0, 1.0),
    }
}

/// Power type override while shapeshifted, mirroring `Aura::HandleAuraModShapeshift`'s
/// `PowerType` switch. Returns `None` when the form keeps the class default (mana).
pub fn shapeshift_power_type(form: u8) -> Option<crate::game::player::power::PowerType> {
    use crate::game::player::power::PowerType;
    match form {
        FORM_CAT => Some(PowerType::Energy),
        FORM_BEAR | FORM_DIREBEAR => Some(PowerType::Rage),
        FORM_BATTLESTANCE | FORM_BERSERKERSTANCE | FORM_DEFENSIVESTANCE => Some(PowerType::Rage),
        _ => None,
    }
}
