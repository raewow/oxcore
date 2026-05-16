//! Spell repository for loading spell data from spell_template

use crate::world::dbc::structures::SpellEntry;
use anyhow::{Context, Result};
use sqlx::{MySqlPool, Row};
use std::sync::Arc;

pub struct SpellRepository {
    pool: Arc<MySqlPool>,
}

impl SpellRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        Self { pool }
    }

    /// Load all spells from spell_template (matching legacy_world approach)
    pub async fn load_all(&self) -> Result<Vec<SpellEntry>> {
        let query = "SELECT entry, build, school, category, castUI, dispel, mechanic,
            attributes, attributesEx, attributesEx2, attributesEx3, attributesEx4,
            stances, stancesNot, targets, targetCreatureType, requiresSpellFocus,
            casterAuraState, targetAuraState, castingTimeIndex, recoveryTime,
            categoryRecoveryTime, interruptFlags, auraInterruptFlags, channelInterruptFlags,
            procFlags, procChance, procCharges, maxLevel, baseLevel, spellLevel,
            durationIndex, powerType, manaCost, manCostPerLevel, manaPerSecond,
            manaPerSecondPerLevel, rangeIndex, speed, modelNextSpell, stackAmount,
            totem1, totem2,
            reagent1, reagent2, reagent3, reagent4, reagent5, reagent6, reagent7, reagent8,
            reagentCount1, reagentCount2, reagentCount3, reagentCount4, reagentCount5, reagentCount6, reagentCount7, reagentCount8,
            equippedItemClass, equippedItemSubClassMask, equippedItemInventoryTypeMask,
            effect1, effect2, effect3,
            effectDieSides1, effectDieSides2, effectDieSides3,
            effectBaseDice1, effectBaseDice2, effectBaseDice3,
            effectDicePerLevel1, effectDicePerLevel2, effectDicePerLevel3,
            effectRealPointsPerLevel1, effectRealPointsPerLevel2, effectRealPointsPerLevel3,
            effectBasePoints1, effectBasePoints2, effectBasePoints3,
            effectBonusCoefficient1, effectBonusCoefficient2, effectBonusCoefficient3,
            effectMechanic1, effectMechanic2, effectMechanic3,
            effectImplicitTargetA1, effectImplicitTargetA2, effectImplicitTargetA3,
            effectImplicitTargetB1, effectImplicitTargetB2, effectImplicitTargetB3,
            effectRadiusIndex1, effectRadiusIndex2, effectRadiusIndex3,
            effectApplyAuraName1, effectApplyAuraName2, effectApplyAuraName3,
            effectAmplitude1, effectAmplitude2, effectAmplitude3,
            effectMultipleValue1, effectMultipleValue2, effectMultipleValue3,
            effectChainTarget1, effectChainTarget2, effectChainTarget3,
            effectItemType1, effectItemType2, effectItemType3,
            effectMiscValue1, effectMiscValue2, effectMiscValue3,
            effectTriggerSpell1, effectTriggerSpell2, effectTriggerSpell3,
            effectPointsPerComboPoint1, effectPointsPerComboPoint2, effectPointsPerComboPoint3,
            spellVisual1, spellVisual2,
            spellIconId, activeIconId, spellPriority,
            name, nameFlags, nameSubtext, nameSubtextFlags, description, descriptionFlags, auraDescription, auraDescriptionFlags,
            manaCostPercentage, startRecoveryCategory, startRecoveryTime,
            minTargetLevel, maxTargetLevel, spellFamilyName, spellFamilyFlags,
            maxAffectedTargets, dmgClass, preventionType, stanceBarOrder,
            dmgMultiplier1, dmgMultiplier2, dmgMultiplier3,
            minFactionId, minReputation, requiredAuraVision
            FROM spell_template t1
            WHERE build=(SELECT max(build) FROM spell_template t2 WHERE t1.entry=t2.entry AND build <= 5875)";

        let rows = sqlx::query(query)
            .fetch_all(&*self.pool)
            .await
            .context("Failed to query spell_template")?;

        let mut spells = Vec::with_capacity(rows.len());

        for row in rows.iter() {
            let entry = SpellEntry {
                id: row.get(0),
                // build at index 1 - skipped
                name: row.get::<String, _>(124),
                school: row.get(2),
                category: row.get(3),
                // castUI at index 4 - skipped
                dispel: row.get::<u32, _>(5),
                mechanic: row.get::<u32, _>(6),
                attributes: row.get(7),
                attributes_ex: row.get(8),
                attributes_ex2: row.get(9),
                attributes_ex3: row.get(10),
                attributes_ex4: row.get(11),
                stances: row.get(12),
                stances_not: row.get(13),
                targets: row.get(14),
                target_creature_type: row.get(15),
                requires_spell_focus: row.get(16),
                caster_aura_state: row.get(17),
                target_aura_state: row.get(18),
                casting_time_index: row.get(19),
                recovery_time: row.get(20),
                category_recovery_time: row.get(21),
                interrupt_flags: row.get(22),
                aura_interrupt_flags: row.get(23),
                channel_interrupt_flags: row.get(24),
                proc_flags: row.get(25),
                proc_chance: row.get(26),
                proc_charges: row.get(27),
                max_level: row.get(28),
                base_level: row.get(29),
                spell_level: row.get(30),
                duration_index: row.get(31),
                power_type: row.get(32),
                mana_cost: row.get(33),
                mana_cost_per_level: row.get(34),
                mana_per_second: row.get(35),
                mana_per_second_per_level: row.get(36),
                range_index: row.get(37),
                speed: row.get::<f32, _>(38),
                // modalNextSpell at index 39 - skipped
                stack_amount: row.get(40),
                totem: [row.get(41), row.get(42)],
                reagent: [
                    row.get::<i32, _>(43),
                    row.get::<i32, _>(44),
                    row.get::<i32, _>(45),
                    row.get::<i32, _>(46),
                    row.get::<i32, _>(47),
                    row.get::<i32, _>(48),
                    row.get::<i32, _>(49),
                    row.get::<i32, _>(50),
                ],
                reagent_count: [
                    row.get(51),
                    row.get(52),
                    row.get(53),
                    row.get(54),
                    row.get(55),
                    row.get(56),
                    row.get(57),
                    row.get(58),
                ],
                equipped_item_class: row.get::<i32, _>(59),
                equipped_item_sub_class_mask: row.get::<i32, _>(60),
                equipped_item_inventory_type_mask: row.get::<i32, _>(61),
                effect: [row.get(62), row.get(63), row.get(64)],
                effect_die_sides: [
                    row.get::<i32, _>(65),
                    row.get::<i32, _>(66),
                    row.get::<i32, _>(67),
                ],
                effect_base_dice: [row.get(68), row.get(69), row.get(70)],
                effect_dice_per_level: [
                    row.get::<f32, _>(71),
                    row.get::<f32, _>(72),
                    row.get::<f32, _>(73),
                ],
                effect_real_points_per_level: [
                    row.get::<f32, _>(74),
                    row.get::<f32, _>(75),
                    row.get::<f32, _>(76),
                ],
                effect_base_points: [
                    row.get::<i32, _>(77),
                    row.get::<i32, _>(78),
                    row.get::<i32, _>(79),
                ],
                effect_bonus_coefficient: [
                    row.get::<f32, _>(80),
                    row.get::<f32, _>(81),
                    row.get::<f32, _>(82),
                ],
                effect_mechanic: [row.get(83), row.get(84), row.get(85)],
                effect_implicit_target_a: [row.get(86), row.get(87), row.get(88)],
                effect_implicit_target_b: [row.get(89), row.get(90), row.get(91)],
                effect_radius_index: [row.get(92), row.get(93), row.get(94)],
                effect_apply_aura_name: [row.get(95), row.get(96), row.get(97)],
                effect_amplitude: [row.get(98), row.get(99), row.get(100)],
                effect_multiple_value: [
                    row.get::<f32, _>(101),
                    row.get::<f32, _>(102),
                    row.get::<f32, _>(103),
                ],
                effect_chain_target: [row.get(104), row.get(105), row.get(106)],
                effect_item_type: [
                    row.get::<u64, _>(107),
                    row.get::<u64, _>(108),
                    row.get::<u64, _>(109),
                ],
                effect_misc_value: [
                    row.get::<i32, _>(110),
                    row.get::<i32, _>(111),
                    row.get::<i32, _>(112),
                ],
                effect_trigger_spell: [row.get(113), row.get(114), row.get(115)],
                effect_points_per_combo_point: [
                    row.get::<f32, _>(116),
                    row.get::<f32, _>(117),
                    row.get::<f32, _>(118),
                ],
                spell_visual: row.get(119),
                // spellVisual2 at 120 - skipped
                spell_icon_id: row.get(121),
                active_icon_id: row.get(122),
                spell_priority: row.get(123),
                // name at 124, nameFlags at 125, nameSubtext at 126, nameSubtextFlags at 127
                // description at 128, descriptionFlags at 129, auraDescription at 130, auraDescriptionFlags at 131
                mana_cost_percentage: row.get(132),
                start_recovery_category: row.get(133),
                start_recovery_time: row.get(134),
                // minTargetLevel at 135
                max_target_level: row.get(136),
                spell_family_name: row.get(137),
                spell_family_flags: row.get::<u64, _>(138),
                max_affected_targets: row.get(139),
                dmg_class: row.get(140),
                prevention_type: row.get(141),
                // stanceBarOrder at 142
                dmg_multiplier: [
                    row.get::<f32, _>(143),
                    row.get::<f32, _>(144),
                    row.get::<f32, _>(145),
                ],
            };

            spells.push(entry);
        }

        Ok(spells)
    }
}
