use crate::file_loader::DbcRecord;
use crate::manager::DbcManager;
use crate::store::DbcEntry;
use anyhow::{Context, Result};
/// SpellCastTimes DBC entry
/// Format: "niii" (4 fields: ID, CastTime, CastTimePerLevel, MinCastTime)
/// CastTime is in milliseconds
#[derive(Debug, Clone)]
pub struct SpellCastTimeEntry {
    pub id: u32,
    /// Base cast time in milliseconds
    pub cast_time: i32,
    /// Cast time per level in milliseconds (for level-scaling spells)
    pub cast_time_per_level: i32,
    /// Minimum cast time in milliseconds
    pub min_cast_time: i32,
}

impl DbcEntry for SpellCastTimeEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record
            .get_u32(0)
            .context("Failed to read SpellCastTimes ID")?;

        if id == 0 {
            return Ok(None);
        }

        let entry = Self {
            id,
            cast_time: record
                .get_i32(1)
                .context("Failed to read SpellCastTimes cast_time")?,
            cast_time_per_level: record
                .get_i32(2)
                .context("Failed to read SpellCastTimes cast_time_per_level")?,
            min_cast_time: record
                .get_i32(3)
                .context("Failed to read SpellCastTimes min_cast_time")?,
        };

        Ok(Some((id, entry)))
    }
}

/// SpellDuration DBC entry
/// Format: "niii" (4 fields: ID, Duration, DurationPerLevel, MaxDuration)
/// Duration values are in milliseconds
#[derive(Debug, Clone, Default)]
pub struct SpellDurationEntry {
    pub id: u32,
    /// Base duration in milliseconds
    pub duration: i32,
    /// Duration per level in milliseconds (for level-scaling spells)
    pub duration_per_level: i32,
    /// Maximum duration in milliseconds
    pub max_duration: i32,
}

impl DbcEntry for SpellDurationEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record
            .get_u32(0)
            .context("Failed to read SpellDuration ID")?;

        if id == 0 {
            return Ok(None);
        }

        let entry = Self {
            id,
            duration: record
                .get_u32(1)
                .context("Failed to read SpellDuration duration")? as i32,
            duration_per_level: record
                .get_u32(2)
                .context("Failed to read SpellDuration duration_per_level")?
                as i32,
            max_duration: record
                .get_u32(3)
                .context("Failed to read SpellDuration max_duration")?
                as i32,
        };

        Ok(Some((id, entry)))
    }
}

/// SpellRadius DBC entry
/// Format: "nff" (3 fields: ID, Radius, RadiusPerLevel)
/// Radius values are in yards
#[derive(Debug, Clone)]
pub struct SpellRadiusEntry {
    pub id: u32,
    /// Base radius in yards
    pub radius: f32,
    /// Radius per level in yards (for level-scaling spells)
    pub radius_per_level: f32,
}

impl DbcEntry for SpellRadiusEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read SpellRadius ID")?;

        if id == 0 {
            return Ok(None);
        }

        let entry = Self {
            id,
            radius: record
                .get_f32(1)
                .context("Failed to read SpellRadius radius")?,
            radius_per_level: record
                .get_f32(2)
                .context("Failed to read SpellRadius radius_per_level")?,
        };

        Ok(Some((id, entry)))
    }
}

/// SpellRange DBC entry
/// Format: "nffxxxxxxxxxxxxxxxxxxx" (ID, RangeMin, RangeMax, then metadata)
/// Range values are in yards
///
/// Vanilla 1.12.1 SpellRange.dbc layout:
/// Field 0: ID
/// Field 1: RangeMin (friendly)
/// Field 2: RangeMax (friendly)
/// Field 3: Flags
#[derive(Debug, Clone)]
pub struct SpellRangeEntry {
    pub id: u32,
    /// Minimum range in yards
    pub range_min: f32,
    /// Maximum range in yards
    pub range_max: f32,
}

impl DbcEntry for SpellRangeEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read SpellRange ID")?;

        if id == 0 {
            return Ok(None);
        }

        let entry = Self {
            id,
            range_min: record
                .get_f32(1)
                .context("Failed to read SpellRange range_min")?,
            range_max: record
                .get_f32(2)
                .context("Failed to read SpellRange range_max")?,
        };

        Ok(Some((id, entry)))
    }
}

/// SpellFocusObject DBC entry
/// Format: "n" (id only, minimal structure)
/// Contains spell focus object information
#[derive(Debug, Clone)]
pub struct SpellFocusObjectEntry {
    pub id: u32,
}

impl DbcEntry for SpellFocusObjectEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record
            .get_u32(0)
            .context("Failed to read SpellFocusObject ID")?;

        if id == 0 {
            return Ok(None);
        }

        let entry = Self { id };

        Ok(Some((id, entry)))
    }
}

/// Spell DBC entry
/// Format: Complex structure with 176+ fields in vanilla 1.12.1
/// This structure contains all spell data needed for casting and effects
#[derive(Debug, Clone)]
pub struct SpellEntry {
    pub id: u32,
    pub name: String,
    pub rank_text: String,
    pub school: u32,
    pub category: u32,
    pub dispel: u32,
    pub mechanic: u32,
    pub attributes: u32,
    pub attributes_ex: u32,
    pub attributes_ex2: u32,
    pub attributes_ex3: u32,
    pub attributes_ex4: u32,
    pub stances: u32,
    pub stances_not: u32,
    pub targets: u32,
    pub target_creature_type: u32,
    pub requires_spell_focus: u32,
    pub caster_aura_state: u32,
    pub target_aura_state: u32,
    pub casting_time_index: u32,
    pub recovery_time: u32,
    pub category_recovery_time: u32,
    pub interrupt_flags: u32,
    pub aura_interrupt_flags: u32,
    pub channel_interrupt_flags: u32,
    pub proc_flags: u32,
    pub proc_chance: u32,
    pub proc_charges: u32,
    pub max_level: u32,
    pub base_level: u32,
    pub spell_level: u32,
    pub duration_index: u32,
    pub power_type: u32,
    pub mana_cost: u32,
    pub mana_cost_per_level: u32,
    pub mana_per_second: u32,
    pub mana_per_second_per_level: u32,
    pub range_index: u32,
    pub speed: f32,
    pub stack_amount: u32,
    pub totem: [u32; 2],
    pub reagent: [i32; 8],
    pub reagent_count: [u32; 8],
    pub equipped_item_class: i32,
    pub equipped_item_sub_class_mask: i32,
    pub equipped_item_inventory_type_mask: i32,
    /// Effect types for 3 effects (indices 0-2)
    pub effect: [u32; 3],
    /// Effect die sides for random damage/healing
    pub effect_die_sides: [i32; 3],
    /// Effect base dice
    pub effect_base_dice: [u32; 3],
    /// Effect dice per level
    pub effect_dice_per_level: [f32; 3],
    /// Effect real points per level (scaling)
    pub effect_real_points_per_level: [f32; 3],
    /// Effect base points (main value)
    pub effect_base_points: [i32; 3],
    /// Effect bonus coefficient (spell power scaling)
    pub effect_bonus_coefficient: [f32; 3],
    /// Effect mechanic
    pub effect_mechanic: [u32; 3],
    /// Effect implicit target A
    pub effect_implicit_target_a: [u32; 3],
    /// Effect implicit target B
    pub effect_implicit_target_b: [u32; 3],
    /// Effect radius index
    pub effect_radius_index: [u32; 3],
    /// Effect apply aura name (for aura effects)
    pub effect_apply_aura_name: [u32; 3],
    /// Effect amplitude (periodic tick time)
    pub effect_amplitude: [u32; 3],
    /// Effect multiple value
    pub effect_multiple_value: [f32; 3],
    /// Effect chain target count
    pub effect_chain_target: [u32; 3],
    /// Effect item type
    pub effect_item_type: [u64; 3],
    /// Effect misc value (used by many effects)
    pub effect_misc_value: [i32; 3],
    /// Effect trigger spell
    pub effect_trigger_spell: [u32; 3],
    /// Effect points per combo point
    pub effect_points_per_combo_point: [f32; 3],
    pub spell_visual: u32,
    pub spell_icon_id: u32,
    pub active_icon_id: u32,
    pub spell_priority: u32,
    pub min_target_level: u32,
    pub mana_cost_percentage: u32,
    pub start_recovery_category: u32,
    pub start_recovery_time: u32,
    pub max_target_level: u32,
    pub spell_family_name: u32,
    pub spell_family_flags: u64,
    pub max_affected_targets: u32,
    pub dmg_class: u32,
    pub prevention_type: u32,
    pub custom: u32,
    pub internal: u32,
    pub allowed_target_mask: u32,
    pub script_id: u32,
    pub dmg_multiplier: [f32; 3],
}

impl SpellEntry {
    /// Get the number of non-none effects
    pub fn get_effects_count(&self) -> u8 {
        self.effect.iter().filter(|&&e| e != 0).count() as u8
    }

    /// Check if spell has a specific attribute
    /// Test a `SPELL_ATTR_*` bit against the base `Attributes` column.
    ///
    /// The four attribute columns reuse the same bit values for unrelated flags, so a
    /// `SPELL_ATTR_EX*` constant must go through the matching accessor below — passing one
    /// here silently tests a different flag.
    pub fn has_attribute(&self, attribute: u32) -> bool {
        (self.attributes & attribute) != 0
    }

    /// Test a `SPELL_ATTR_EX_*` bit against the `AttributesEx` column.
    pub fn has_attribute_ex(&self, attribute: u32) -> bool {
        (self.attributes_ex & attribute) != 0
    }

    /// Test a `SPELL_ATTR_EX2_*` bit against the `AttributesEx2` column.
    pub fn has_attribute_ex2(&self, attribute: u32) -> bool {
        (self.attributes_ex2 & attribute) != 0
    }

    /// Test a `SPELL_ATTR_EX3_*` bit against the `AttributesEx3` column.
    pub fn has_attribute_ex3(&self, attribute: u32) -> bool {
        (self.attributes_ex3 & attribute) != 0
    }

    /// Check if spell has a specific effect
    pub fn has_effect(&self, effect_type: u32) -> bool {
        self.effect.iter().any(|&e| e == effect_type)
    }

    /// `SpellInternal::IsDirectDamageSpell` — true when any effect deals direct
    /// (non-periodic) damage. Computed on demand from the effect list rather than
    /// a precomputed internal flag.
    pub fn is_direct_damage_spell(&self) -> bool {
        // INSTAKILL(1), SCHOOL_DAMAGE(2), ENVIRONMENTAL_DAMAGE(7), HEALTH_LEECH(9),
        // WEAPON_DAMAGE_NOSCHOOL(17), WEAPON_PERCENT_DAMAGE(31), WEAPON_DAMAGE(58),
        // POWER_BURN(62), NORMALIZED_WEAPON_DMG(121).
        self.effect
            .iter()
            .any(|&e| matches!(e, 1 | 2 | 7 | 9 | 17 | 31 | 58 | 62 | 121))
    }

    /// Check if spell applies an aura
    pub fn is_aura_spell(&self) -> bool {
        self.effect.iter().enumerate().any(|(i, &e)| {
            e == 6 || e == 27 || e == 35 || e == 119 || e == 128 || e == 129 || e == 132
            // ApplyAura effects
        })
    }

    pub fn is_passive_spell(&self) -> bool {
        self.has_attribute(0x0000_0040)
    }

    pub fn is_autocastable(&self) -> bool {
        (self.attributes_ex & 0x0002_0000) == 0 && !self.is_passive_spell()
    }

    pub fn is_positive_effect(&self, idx: usize) -> bool {
        if idx >= self.effect.len() || self.effect[idx] == 0 {
            return false;
        }

        if self.custom & 0x0000_0001 != 0 {
            return true;
        }
        if self.custom & 0x0000_0002 != 0 {
            return false;
        }

        match self.effect[idx] {
            10 | 36 | 37 => true,
            1 => self.effect_implicit_target_a[idx] == 1,
            6 => match self.effect_apply_aura_name[idx] {
                3 | 8 | 62 | 84 | 85 => true,
                20 | 21 | 33 | 44 | 89 => false,
                107 | 108 => self.effect_misc_value[idx] <= 0,
                _ => {
                    self.effect_implicit_target_a[idx] == 1
                        || self.effect_implicit_target_b[idx] == 0
                }
            },
            _ => self.effect_implicit_target_a[idx] == 1,
        }
    }

    pub fn is_positive_spell(&self) -> bool {
        (self.attributes & 0x0400_0000) == 0
            && self
                .effect
                .iter()
                .enumerate()
                .all(|(idx, &effect)| effect == 0 || self.is_positive_effect(idx))
    }

    pub fn is_reflectable_spell(&self) -> bool {
        self.dmg_class == 2 && !self.is_passive_spell() && !self.is_positive_spell()
    }

    pub fn get_weapon_attack_type(&self) -> u32 {
        match self.dmg_class {
            1 => {
                if (self.attributes_ex3 & 0x0000_0800) != 0 {
                    1
                } else {
                    0
                }
            }
            2 => 2,
            _ => {
                if (self.attributes_ex2 & 0x0000_4000) != 0 {
                    2
                } else {
                    0
                }
            }
        }
    }

    pub fn get_cast_time(&self, dbc: &DbcManager) -> u32 {
        let Some(entry) = dbc.get_spell_cast_time(self.casting_time_index) else {
            return 0;
        };
        let cast_time = entry.cast_time as i32;
        if cast_time <= 0 {
            0
        } else {
            cast_time as u32
        }
    }

    pub fn get_cast_time_for_bonus(&self, _effect_type: u32) -> f32 {
        let mut cast_time = self.get_cast_time(&crate::manager::DbcManager::new()) as f32;
        if cast_time > 7000.0 {
            cast_time = 7000.0;
        }
        if cast_time < 1500.0 {
            cast_time = 1500.0;
        }
        cast_time / 3500.0
    }

    pub fn calculate_default_coefficient(&self) -> f64 {
        self.get_cast_time_for_bonus(0) as f64
    }

    pub fn calculate_custom_coefficient(&self, coeff: f64) -> f64 {
        coeff
    }

    pub fn get_duration(&self, dbc: &DbcManager) -> i32 {
        dbc.get_spell_duration(self.duration_index)
            .map(|d| d.duration)
            .unwrap_or(0)
    }

    pub fn get_max_duration(&self, dbc: &DbcManager) -> i32 {
        dbc.get_spell_duration(self.duration_index)
            .map(|d| d.max_duration)
            .unwrap_or(0)
    }

    pub fn calculate_duration(&self, _level: u32, dbc: &DbcManager) -> i32 {
        self.get_duration(dbc)
    }

    pub fn get_aura_max_ticks(&self, dbc: &DbcManager) -> u32 {
        let duration = self.get_duration(dbc);
        if duration <= 0 {
            return 1;
        }
        for idx in 0..self.effect.len() {
            if self.effect[idx] == 6 && self.effect_amplitude[idx] != 0 {
                return duration as u32 / self.effect_amplitude[idx];
            }
        }
        6
    }

    pub fn get_rank(&self) -> u32 {
        self.rank_text
            .strip_prefix("Rank ")
            .and_then(|text| text.trim().parse::<u32>().ok())
            .unwrap_or(0)
    }

    /// `Spells::GetSpellMaxRange` — max range in yards for this spell's range
    /// index, or 0 when the range entry is missing.
    pub fn get_spell_max_range(&self, dbc: &DbcManager) -> f32 {
        dbc.get_spell_range(self.range_index)
            .map_or(0.0, |range| range.range_max)
    }

    /// `Spells::GetSpellMinRange` — min range in yards for this spell's range
    /// index, or 0 when the range entry is missing.
    pub fn get_spell_min_range(&self, dbc: &DbcManager) -> f32 {
        dbc.get_spell_range(self.range_index)
            .map_or(0.0, |range| range.range_min)
    }

    pub fn is_target_in_range(&self, dist: f32, dbc: &DbcManager) -> bool {
        match self.range_index {
            1 => true,
            13 => true,
            2 => dist <= 5.0,
            // A missing range entry yields max = min = 0, so `dist < 0` is false —
            // matching the previous unwrap_or(false) behaviour.
            _ => dist < self.get_spell_max_range(dbc) && dist >= self.get_spell_min_range(dbc),
        }
    }

    pub fn can_trigger_weapon_procs(&self) -> bool {
        self.equipped_item_class == 2 && self.range_index == 1
    }
}

impl DbcEntry for SpellEntry {
    fn from_record(record: &DbcRecord) -> Result<Option<(u32, Self)>> {
        let id = record.get_u32(0).context("Failed to read Spell ID")?;

        if id == 0 {
            return Ok(None);
        }

        // Spell.dbc has many fields - we'll read the ones we need
        // Format for vanilla 1.12.1 (approximately 176 fields)
        // Fields 0-175 are the main spell data

        let mut effect = [0u32; 3];
        let mut effect_die_sides = [0i32; 3];
        let mut effect_base_dice = [0u32; 3];
        let mut effect_dice_per_level = [0f32; 3];
        let mut effect_real_points_per_level = [0f32; 3];
        let mut effect_base_points = [0i32; 3];
        let mut effect_bonus_coefficient = [0f32; 3];
        let mut effect_mechanic = [0u32; 3];
        let mut effect_implicit_target_a = [0u32; 3];
        let mut effect_implicit_target_b = [0u32; 3];
        let mut effect_radius_index = [0u32; 3];
        let mut effect_apply_aura_name = [0u32; 3];
        let mut effect_amplitude = [0u32; 3];
        let mut effect_multiple_value = [0f32; 3];
        let mut effect_chain_target = [0u32; 3];
        let mut effect_item_type = [0u64; 3];
        let mut effect_misc_value = [0i32; 3];
        let mut effect_trigger_spell = [0u32; 3];
        let mut effect_points_per_combo_point = [0f32; 3];

        // Read effect data (fields 61-117)
        for i in 0..3 {
            effect[i] = record.get_u32(61 + i).unwrap_or(0);
            effect_die_sides[i] = record.get_i32(64 + i).unwrap_or(0);
            effect_base_dice[i] = record.get_u32(67 + i).unwrap_or(0);
            effect_dice_per_level[i] = record.get_f32(70 + i).unwrap_or(0.0);
            effect_real_points_per_level[i] = record.get_f32(73 + i).unwrap_or(0.0);
            effect_base_points[i] = record.get_i32(76 + i).unwrap_or(0);
            effect_bonus_coefficient[i] = record.get_f32(79 + i).unwrap_or(0.0);
            effect_mechanic[i] = record.get_u32(82 + i).unwrap_or(0);
            effect_implicit_target_a[i] = record.get_u32(85 + i).unwrap_or(0);
            effect_implicit_target_b[i] = record.get_u32(88 + i).unwrap_or(0);
            effect_radius_index[i] = record.get_u32(91 + i).unwrap_or(0);
            effect_apply_aura_name[i] = record.get_u32(94 + i).unwrap_or(0);
            effect_amplitude[i] = record.get_u32(97 + i).unwrap_or(0);
            effect_multiple_value[i] = record.get_f32(100 + i).unwrap_or(0.0);
            effect_chain_target[i] = record.get_u32(103 + i).unwrap_or(0);
            // effect_item_type is u64 but DBC only has u32 fields
            // Read as two u32s and combine (low + high << 32)
            let item_type_low = record.get_u32(106 + i).unwrap_or(0);
            let item_type_high = record.get_u32(107 + i).unwrap_or(0);
            effect_item_type[i] = (item_type_low as u64) | ((item_type_high as u64) << 32);
            effect_misc_value[i] = record.get_i32(109 + i).unwrap_or(0);
            effect_trigger_spell[i] = record.get_u32(112 + i).unwrap_or(0);
            effect_points_per_combo_point[i] = record.get_f32(115 + i).unwrap_or(0.0);
        }

        let entry = Self {
            id,
            name: String::new(),
            rank_text: String::new(),
            school: record.get_u32(1).unwrap_or(0),
            category: record.get_u32(2).unwrap_or(0),
            dispel: record.get_u32(4).unwrap_or(0),
            mechanic: record.get_u32(5).unwrap_or(0),
            attributes: record.get_u32(6).unwrap_or(0),
            attributes_ex: record.get_u32(7).unwrap_or(0),
            attributes_ex2: record.get_u32(8).unwrap_or(0),
            attributes_ex3: record.get_u32(9).unwrap_or(0),
            attributes_ex4: record.get_u32(10).unwrap_or(0),
            stances: record.get_u32(11).unwrap_or(0),
            stances_not: record.get_u32(12).unwrap_or(0),
            targets: record.get_u32(13).unwrap_or(0),
            target_creature_type: record.get_u32(14).unwrap_or(0),
            requires_spell_focus: record.get_u32(15).unwrap_or(0),
            caster_aura_state: record.get_u32(16).unwrap_or(0),
            target_aura_state: record.get_u32(17).unwrap_or(0),
            casting_time_index: record.get_u32(18).unwrap_or(0),
            recovery_time: record.get_u32(19).unwrap_or(0),
            category_recovery_time: record.get_u32(20).unwrap_or(0),
            interrupt_flags: record.get_u32(21).unwrap_or(0),
            aura_interrupt_flags: record.get_u32(22).unwrap_or(0),
            channel_interrupt_flags: record.get_u32(23).unwrap_or(0),
            proc_flags: record.get_u32(24).unwrap_or(0),
            proc_chance: record.get_u32(25).unwrap_or(0),
            proc_charges: record.get_u32(26).unwrap_or(0),
            max_level: record.get_u32(27).unwrap_or(0),
            base_level: record.get_u32(28).unwrap_or(0),
            spell_level: record.get_u32(29).unwrap_or(0),
            duration_index: record.get_u32(30).unwrap_or(0),
            power_type: record.get_u32(31).unwrap_or(0),
            mana_cost: record.get_u32(32).unwrap_or(0),
            mana_cost_per_level: record.get_u32(33).unwrap_or(0),
            mana_per_second: record.get_u32(34).unwrap_or(0),
            mana_per_second_per_level: record.get_u32(35).unwrap_or(0),
            range_index: record.get_u32(36).unwrap_or(1),
            speed: record.get_f32(37).unwrap_or(0.0),
            stack_amount: record.get_u32(39).unwrap_or(0),
            totem: [
                record.get_u32(40).unwrap_or(0),
                record.get_u32(41).unwrap_or(0),
            ],
            reagent: [
                record.get_i32(42).unwrap_or(0),
                record.get_i32(43).unwrap_or(0),
                record.get_i32(44).unwrap_or(0),
                record.get_i32(45).unwrap_or(0),
                record.get_i32(46).unwrap_or(0),
                record.get_i32(47).unwrap_or(0),
                record.get_i32(48).unwrap_or(0),
                record.get_i32(49).unwrap_or(0),
            ],
            reagent_count: [
                record.get_u32(50).unwrap_or(0),
                record.get_u32(51).unwrap_or(0),
                record.get_u32(52).unwrap_or(0),
                record.get_u32(53).unwrap_or(0),
                record.get_u32(54).unwrap_or(0),
                record.get_u32(55).unwrap_or(0),
                record.get_u32(56).unwrap_or(0),
                record.get_u32(57).unwrap_or(0),
            ],
            equipped_item_class: record.get_i32(58).unwrap_or(-1),
            equipped_item_sub_class_mask: record.get_i32(59).unwrap_or(0),
            equipped_item_inventory_type_mask: record.get_i32(60).unwrap_or(0),
            effect,
            effect_die_sides,
            effect_base_dice,
            effect_dice_per_level,
            effect_real_points_per_level,
            effect_base_points,
            effect_bonus_coefficient,
            effect_mechanic,
            effect_implicit_target_a,
            effect_implicit_target_b,
            effect_radius_index,
            effect_apply_aura_name,
            effect_amplitude,
            effect_multiple_value,
            effect_chain_target,
            effect_item_type,
            effect_misc_value,
            effect_trigger_spell,
            effect_points_per_combo_point,
            spell_visual: record.get_u32(118).unwrap_or(0),
            spell_icon_id: record.get_u32(120).unwrap_or(0),
            active_icon_id: record.get_u32(121).unwrap_or(0),
            spell_priority: record.get_u32(122).unwrap_or(0),
            min_target_level: record.get_u32(162).unwrap_or(0),
            mana_cost_percentage: record.get_u32(159).unwrap_or(0),
            start_recovery_category: record.get_u32(160).unwrap_or(0),
            start_recovery_time: record.get_u32(161).unwrap_or(0),
            max_target_level: record.get_u32(163).unwrap_or(0),
            spell_family_name: record.get_u32(164).unwrap_or(0),
            // spell_family_flags is u64 but DBC only has u32 fields
            // Read as two u32s and combine (low + high << 32)
            spell_family_flags: {
                let flags_low = record.get_u32(165).unwrap_or(0);
                let flags_high = record.get_u32(166).unwrap_or(0);
                (flags_low as u64) | ((flags_high as u64) << 32)
            },
            max_affected_targets: record.get_u32(167).unwrap_or(0),
            dmg_class: record.get_u32(168).unwrap_or(0),
            prevention_type: record.get_u32(169).unwrap_or(0),
            custom: record.get_u32(176).unwrap_or(0),
            internal: 0,
            allowed_target_mask: 0,
            script_id: 0,
            dmg_multiplier: [
                record.get_f32(171).unwrap_or(1.0),
                record.get_f32(172).unwrap_or(1.0),
                record.get_f32(173).unwrap_or(1.0),
            ],
        };

        Ok(Some((id, entry)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_spell_entry(id: u32) -> SpellEntry {
        SpellEntry {
            id,
            name: format!("Spell{}", id),
            rank_text: String::new(),
            school: 0,
            category: 0,
            dispel: 0,
            mechanic: 0,
            attributes: 0,
            attributes_ex: 0,
            attributes_ex2: 0,
            attributes_ex3: 0,
            attributes_ex4: 0,
            stances: 0,
            stances_not: 0,
            targets: 0,
            target_creature_type: 0,
            requires_spell_focus: 0,
            caster_aura_state: 0,
            target_aura_state: 0,
            casting_time_index: 0,
            recovery_time: 0,
            category_recovery_time: 0,
            interrupt_flags: 0,
            aura_interrupt_flags: 0,
            channel_interrupt_flags: 0,
            proc_flags: 0,
            proc_chance: 0,
            proc_charges: 0,
            max_level: 0,
            base_level: 0,
            spell_level: 0,
            duration_index: 0,
            power_type: 0,
            mana_cost: 0,
            mana_cost_per_level: 0,
            mana_per_second: 0,
            mana_per_second_per_level: 0,
            range_index: 0,
            speed: 0.0,
            stack_amount: 0,
            totem: [0; 2],
            reagent: [0; 8],
            reagent_count: [0; 8],
            equipped_item_class: 0,
            equipped_item_sub_class_mask: 0,
            equipped_item_inventory_type_mask: 0,
            effect: [0; 3],
            effect_die_sides: [0; 3],
            effect_base_dice: [0; 3],
            effect_dice_per_level: [0.0; 3],
            effect_real_points_per_level: [0.0; 3],
            effect_base_points: [0; 3],
            effect_bonus_coefficient: [0.0; 3],
            effect_mechanic: [0; 3],
            effect_implicit_target_a: [0; 3],
            effect_implicit_target_b: [0; 3],
            effect_radius_index: [0; 3],
            effect_apply_aura_name: [0; 3],
            effect_amplitude: [0; 3],
            effect_multiple_value: [0.0; 3],
            effect_chain_target: [0; 3],
            effect_item_type: [0; 3],
            effect_misc_value: [0; 3],
            effect_trigger_spell: [0; 3],
            effect_points_per_combo_point: [0.0; 3],
            spell_visual: 0,
            spell_icon_id: 0,
            active_icon_id: 0,
            spell_priority: 0,
            min_target_level: 0,
            mana_cost_percentage: 0,
            start_recovery_category: 0,
            start_recovery_time: 0,
            max_target_level: 0,
            spell_family_name: 0,
            spell_family_flags: 0,
            max_affected_targets: 0,
            dmg_class: 0,
            prevention_type: 0,
            custom: 0,
            internal: 0,
            allowed_target_mask: 0,
            script_id: 0,
            dmg_multiplier: [1.0; 3],
        }
    }

    #[test]
    fn rank_parsing_prefers_rank_prefix() {
        let mut spell = make_spell_entry(100);
        spell.rank_text = "Rank 3".to_string();
        assert_eq!(spell.get_rank(), 3);
        spell.rank_text = "Passive".to_string();
        assert_eq!(spell.get_rank(), 0);
    }

    #[test]
    fn cast_time_and_duration_helpers_use_dbc_entries() {
        let mut dbc = DbcManager::new();
        dbc.spell_cast_time.insert(
            1,
            SpellCastTimeEntry {
                id: 1,
                cast_time: 2500,
                cast_time_per_level: 0,
                min_cast_time: 0,
            },
        );
        dbc.spell_duration.insert(
            2,
            SpellDurationEntry {
                id: 2,
                duration: 15000,
                duration_per_level: 0,
                max_duration: 30000,
            },
        );

        let mut spell = make_spell_entry(101);
        spell.casting_time_index = 1;
        spell.duration_index = 2;

        assert_eq!(spell.get_cast_time(&dbc), 2500);
        assert_eq!(spell.get_duration(&dbc), 15000);
        assert_eq!(spell.get_max_duration(&dbc), 30000);
        assert_eq!(spell.get_aura_max_ticks(&dbc), 6);
    }

    #[test]
    fn range_and_shapeshift_helpers_use_entry_data() {
        let mut dbc = DbcManager::new();
        dbc.spell_range.insert(
            3,
            SpellRangeEntry {
                id: 3,
                range_min: 0.0,
                range_max: 30.0,
            },
        );

        let mut spell = make_spell_entry(102);
        spell.range_index = 3;
        assert_eq!(spell.get_spell_max_range(&dbc), 30.0);
        assert!(spell.is_target_in_range(10.0, &dbc));
        assert!(!spell.is_target_in_range(35.0, &dbc));

        spell.range_index = 99;
        assert_eq!(spell.get_spell_max_range(&dbc), 0.0);
    }

    #[test]
    fn spell_range_decodes_max_range_before_flags() {
        use crate::file_loader::DbcFileLoader;

        let path =
            std::env::temp_dir().join(format!("oxcore-spell-range-{}.dbc", std::process::id()));
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"WDBC");
        bytes.extend_from_slice(&1u32.to_le_bytes()); // record count
        bytes.extend_from_slice(&4u32.to_le_bytes()); // field count
        bytes.extend_from_slice(&16u32.to_le_bytes()); // record size
        bytes.extend_from_slice(&0u32.to_le_bytes()); // string-table size
        bytes.extend_from_slice(&3u32.to_le_bytes());
        bytes.extend_from_slice(&5.0f32.to_le_bytes());
        bytes.extend_from_slice(&30.0f32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes()); // flags, not range data
        std::fs::write(&path, bytes).unwrap();

        let mut loader = DbcFileLoader::new();
        loader.load(path.to_str().unwrap(), "nffx").unwrap();
        let record = loader.get_record(0).unwrap();
        let (_, range) = SpellRangeEntry::from_record(&record).unwrap().unwrap();
        std::fs::remove_file(path).unwrap();

        assert_eq!(range.range_min, 5.0);
        assert_eq!(range.range_max, 30.0);
    }
}
