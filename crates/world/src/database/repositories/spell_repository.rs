//! Spell repository for loading spell data from spell_template

use crate::dbc::structures::SpellEntry;
use anyhow::{Context, Result};
use sqlx::{postgres::PgRow, PgPool, Row};
use std::sync::Arc;

pub struct SpellRepository {
    pool: Arc<PgPool>,
}

/// Signed PostgreSQL representation of a `spell_template` row.
///
/// PostgreSQL has no unsigned integer types. Keep all narrowing at this
/// boundary so malformed database data cannot silently wrap into game data.
struct SpellTemplateRow(PgRow);

impl SpellTemplateRow {
    fn new(row: PgRow) -> Self {
        Self(row)
    }

    fn integer(&self, index: usize) -> Result<i64> {
        if let Ok(value) = self.0.try_get::<i64, _>(index) {
            return Ok(value);
        }
        if let Ok(value) = self.0.try_get::<i32, _>(index) {
            return Ok(i64::from(value));
        }
        if let Ok(value) = self.0.try_get::<i16, _>(index) {
            return Ok(i64::from(value));
        }
        self.0
            .try_get::<i64, _>(index)
            .context("spell_template column is not an integer")
    }

    fn u32(&self, index: usize) -> Result<u32> {
        u32::try_from(self.integer(index)?)
            .with_context(|| format!("spell_template column {index} is outside u32 range"))
    }

    fn i32(&self, index: usize) -> Result<i32> {
        i32::try_from(self.integer(index)?)
            .with_context(|| format!("spell_template column {index} is outside i32 range"))
    }

    fn u64(&self, index: usize) -> Result<u64> {
        u64::try_from(self.integer(index)?)
            .with_context(|| format!("spell_template column {index} must not be negative"))
    }

    fn f32(&self, index: usize) -> Result<f32> {
        Ok(self.0.try_get(index)?)
    }

    fn string(&self, index: usize) -> Result<String> {
        Ok(self.0.try_get(index)?)
    }
}

impl SpellRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Load all spells from spell_template (matching legacy_world approach)
    pub async fn load_all(&self) -> Result<Vec<SpellEntry>> {
        let query = "SELECT t1.* FROM world.spell_template t1
            WHERE build=(SELECT max(build) FROM world.spell_template t2 WHERE t1.entry=t2.entry AND build <= 5875)";

        let rows = sqlx::query(query)
            .fetch_all(&*self.pool)
            .await
            .context("Failed to query spell_template")?;

        let mut spells = Vec::with_capacity(rows.len());

        for row in rows.into_iter().map(SpellTemplateRow::new) {
            let entry = SpellEntry {
                id: row.u32(0)?,
                // build at index 1 - skipped
                name: row.string(124)?,
                rank_text: row.string(126)?,
                school: row.u32(2)?,
                category: row.u32(3)?,
                // castUI at index 4 - skipped
                dispel: row.u32(5)?,
                mechanic: row.u32(6)?,
                attributes: row.u32(7)?,
                attributes_ex: row.u32(8)?,
                attributes_ex2: row.u32(9)?,
                attributes_ex3: row.u32(10)?,
                attributes_ex4: row.u32(11)?,
                stances: row.u32(12)?,
                stances_not: row.u32(13)?,
                targets: row.u32(14)?,
                target_creature_type: row.u32(15)?,
                requires_spell_focus: row.u32(16)?,
                caster_aura_state: row.u32(17)?,
                target_aura_state: row.u32(18)?,
                casting_time_index: row.u32(19)?,
                recovery_time: row.u32(20)?,
                category_recovery_time: row.u32(21)?,
                interrupt_flags: row.u32(22)?,
                aura_interrupt_flags: row.u32(23)?,
                channel_interrupt_flags: row.u32(24)?,
                proc_flags: row.u32(25)?,
                proc_chance: row.u32(26)?,
                proc_charges: row.u32(27)?,
                max_level: row.u32(28)?,
                base_level: row.u32(29)?,
                spell_level: row.u32(30)?,
                duration_index: row.u32(31)?,
                power_type: row.u32(32)?,
                mana_cost: row.u32(33)?,
                mana_cost_per_level: row.u32(34)?,
                mana_per_second: row.u32(35)?,
                mana_per_second_per_level: row.u32(36)?,
                range_index: row.u32(37)?,
                speed: row.f32(38)?,
                // modalNextSpell at index 39 - skipped
                stack_amount: row.u32(40)?,
                totem: [row.u32(41)?, row.u32(42)?],
                reagent: [
                    row.i32(43)?,
                    row.i32(44)?,
                    row.i32(45)?,
                    row.i32(46)?,
                    row.i32(47)?,
                    row.i32(48)?,
                    row.i32(49)?,
                    row.i32(50)?,
                ],
                reagent_count: [
                    row.u32(51)?,
                    row.u32(52)?,
                    row.u32(53)?,
                    row.u32(54)?,
                    row.u32(55)?,
                    row.u32(56)?,
                    row.u32(57)?,
                    row.u32(58)?,
                ],
                equipped_item_class: row.i32(59)?,
                equipped_item_sub_class_mask: row.i32(60)?,
                equipped_item_inventory_type_mask: row.i32(61)?,
                effect: [row.u32(62)?, row.u32(63)?, row.u32(64)?],
                effect_die_sides: [row.i32(65)?, row.i32(66)?, row.i32(67)?],
                effect_base_dice: [row.u32(68)?, row.u32(69)?, row.u32(70)?],
                effect_dice_per_level: [row.f32(71)?, row.f32(72)?, row.f32(73)?],
                effect_real_points_per_level: [row.f32(74)?, row.f32(75)?, row.f32(76)?],
                effect_base_points: [row.i32(77)?, row.i32(78)?, row.i32(79)?],
                effect_bonus_coefficient: [row.f32(80)?, row.f32(81)?, row.f32(82)?],
                effect_mechanic: [row.u32(83)?, row.u32(84)?, row.u32(85)?],
                effect_implicit_target_a: [row.u32(86)?, row.u32(87)?, row.u32(88)?],
                effect_implicit_target_b: [row.u32(89)?, row.u32(90)?, row.u32(91)?],
                effect_radius_index: [row.u32(92)?, row.u32(93)?, row.u32(94)?],
                effect_apply_aura_name: [row.u32(95)?, row.u32(96)?, row.u32(97)?],
                effect_amplitude: [row.u32(98)?, row.u32(99)?, row.u32(100)?],
                effect_multiple_value: [row.f32(101)?, row.f32(102)?, row.f32(103)?],
                effect_chain_target: [row.u32(104)?, row.u32(105)?, row.u32(106)?],
                effect_item_type: [row.u64(107)?, row.u64(108)?, row.u64(109)?],
                effect_misc_value: [row.i32(110)?, row.i32(111)?, row.i32(112)?],
                effect_trigger_spell: [row.u32(113)?, row.u32(114)?, row.u32(115)?],
                effect_points_per_combo_point: [row.f32(116)?, row.f32(117)?, row.f32(118)?],
                spell_visual: row.u32(119)?,
                // spellVisual2 at 120 - skipped
                spell_icon_id: row.u32(121)?,
                active_icon_id: row.u32(122)?,
                spell_priority: row.u32(123)?,
                min_target_level: row.u32(135)?,
                // description at 128, descriptionFlags at 129, auraDescription at 130, auraDescriptionFlags at 131
                mana_cost_percentage: row.u32(132)?,
                start_recovery_category: row.u32(133)?,
                start_recovery_time: row.u32(134)?,
                max_target_level: row.u32(136)?,
                spell_family_name: row.u32(137)?,
                spell_family_flags: row.u64(138)?,
                max_affected_targets: row.u32(139)?,
                dmg_class: row.u32(140)?,
                prevention_type: row.u32(141)?,
                // stanceBarOrder at 142
                custom: row.u32(149)?,
                internal: 0,
                allowed_target_mask: 0,
                script_id: 0,
                dmg_multiplier: [row.f32(143)?, row.f32(144)?, row.f32(145)?],
            };

            spells.push(entry);
        }

        Ok(spells)
    }
}
