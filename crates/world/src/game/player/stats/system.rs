//! Stats System
//!
//! Stateless processor that calculates and broadcasts player stats.
//! Follows the ExperienceSystem pattern.

use anyhow::Result;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tracing::info;

use crate::core::common::guid::ObjectGuid as WorldObjectGuid;
use crate::game::broadcast_mgr::{BroadcastManager, BroadcastManagerExt};
use crate::game::common::update_fields::*;
use crate::game::inventory::InventorySystem;
use crate::game::items::manager::ItemTemplate;
use crate::game::items::ItemManager;
use crate::game::player::power::PowerType;
use crate::game::player::skills::{
    get_skill_max_for_level, SkillSaveState, SKILL_2H_AXES, SKILL_2H_MACES, SKILL_2H_SWORDS,
    SKILL_AXES, SKILL_BOWS, SKILL_CROSSBOWS, SKILL_DAGGERS, SKILL_DEFENSE, SKILL_FIST_WEAPONS,
    SKILL_GUNS, SKILL_MACES, SKILL_POLEARMS, SKILL_STAVES, SKILL_SWORDS, SKILL_THROWN, SKILL_WANDS,
};
use crate::game::player::PlayerManager;
use oxcore_shared::game::inventory::{EquipmentSlot, INVENTORY_SLOT_BAG_0};
use oxcore_shared::messages::update::{
    ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
};
use oxcore_shared::messages::ToWorldPacket;
use oxcore_shared::protocol::ObjectGuid;

use super::base_stats::BaseStatsData;
use super::derived;
use super::modifiers::{BaseModGroup, UnitModifierType, UnitMods};

#[derive(Default, Clone, Copy)]
struct EquippedWeaponDamage {
    min: f32,
    max: f32,
    delay_ms: u32,
    ammo_type: u8,
}

impl EquippedWeaponDamage {
    fn from_template(template: &ItemTemplate) -> Option<Self> {
        let min: f32 = template.dmg_min.iter().sum();
        let max: f32 = template.dmg_max.iter().sum();
        if min <= 0.0 && max <= 0.0 {
            return None;
        }

        Some(Self {
            min,
            max,
            delay_ms: u32::from(template.delay).max(1),
            ammo_type: template.ammo_type,
        })
    }
}

#[derive(Default)]
struct EquippedItemBonuses {
    stats: [i32; 5],
    max_health: i32,
    max_mana: i32,
    armor: i32,
    resistances: [i32; 7],
    melee_attack_power: i32,
    ranged_attack_power: i32,
    spell_power: i32,
    healing_power: i32,
    block_value: i32,
    mainhand_damage: Option<EquippedWeaponDamage>,
    offhand_damage: Option<EquippedWeaponDamage>,
    ranged_damage: Option<EquippedWeaponDamage>,
}

impl EquippedItemBonuses {
    fn add_template(&mut self, slot: u8, template: &ItemTemplate) {
        for (&stat_type, &stat_value) in template.stat_type.iter().zip(template.stat_value.iter()) {
            self.add_item_stat(stat_type, stat_value as i32);
        }

        self.armor += template.armor as i32;
        self.resistances[0] += template.armor as i32;
        self.resistances[1] += template.holy_res as i32;
        self.resistances[2] += template.fire_res as i32;
        self.resistances[3] += template.nature_res as i32;
        self.resistances[4] += template.frost_res as i32;
        self.resistances[5] += template.shadow_res as i32;
        self.resistances[6] += template.arcane_res as i32;
        self.block_value += template.block as i32;

        let weapon_damage = EquippedWeaponDamage::from_template(template);
        match EquipmentSlot::from_u8(slot) {
            Some(EquipmentSlot::Mainhand) => self.mainhand_damage = weapon_damage,
            Some(EquipmentSlot::Offhand) => self.offhand_damage = weapon_damage,
            Some(EquipmentSlot::Ranged) => self.ranged_damage = weapon_damage,
            _ => {}
        }
    }

    fn add_item_stat(&mut self, stat_type: u8, value: i32) {
        match stat_type {
            0 => self.max_mana += value,
            1 => self.max_health += value,
            3 => self.stats[1] += value,
            4 => self.stats[0] += value,
            5 => self.stats[3] += value,
            6 => self.stats[4] += value,
            7 => self.stats[2] += value,
            38 => self.melee_attack_power += value,
            39 => self.ranged_attack_power += value,
            41 => self.healing_power += value,
            42 | 45 => self.spell_power += value,
            _ => {}
        }
    }
}

/// Stateless stats system
pub struct StatsSystem {
    broadcast_mgr: Arc<BroadcastManager>,
    player_mgr: Arc<PlayerManager>,
    inventory: Arc<InventorySystem>,
    item_mgr: Arc<ItemManager>,
    world_pool: Arc<sqlx::MySqlPool>,
    base_stats: OnceLock<BaseStatsData>,
}

/// Combine an attack-power base (formula + equipped) with aura modifiers.
///
/// Mirrors MaNGOS `HandleAttackPowerModifier`: flat AP mods (positive/negative) add to the base
/// total, then the percent mods apply a multiplier — final = `(base + flat) * (1 + pct/100)`,
/// clamped to non-negative. Used for both melee and ranged attack power.
fn combine_attack_power(base: f32, flat: i32, pct: i32) -> i32 {
    ((base + flat as f32) * (1.0 + pct as f32 / 100.0)).max(0.0) as i32
}

fn calculate_non_mana_max_power(unit_mods: &super::modifiers::UnitModifierGroup, power: u8) -> u32 {
    unit_mods
        .calculate_total_value(
            UnitMods::from_power(power).unwrap(),
            derived::base_max_power(power) as f32,
        )
        .max(0.0) as u32
}

impl StatsSystem {
    pub fn new(
        broadcast_mgr: Arc<BroadcastManager>,
        player_mgr: Arc<PlayerManager>,
        inventory: Arc<InventorySystem>,
        item_mgr: Arc<ItemManager>,
        world_pool: Arc<sqlx::MySqlPool>,
    ) -> Self {
        Self {
            broadcast_mgr,
            player_mgr,
            inventory,
            item_mgr,
            world_pool,
            base_stats: OnceLock::new(),
        }
    }

    // ========== Lifecycle ==========

    pub async fn init(&self) -> Result<()> {
        let data = BaseStatsData::load(&self.world_pool).await?;
        self.base_stats
            .set(data)
            .map_err(|_| anyhow::anyhow!("BaseStatsData already initialized"))?;
        Ok(())
    }

    pub fn update(&self, _diff: Duration) -> Result<()> {
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn equipped_weapon_skill(&self, guid: ObjectGuid, slot: EquipmentSlot) -> Option<u16> {
        let item_guid =
            self.inventory
                .cache()
                .get_item_at(guid, INVENTORY_SLOT_BAG_0, slot as u8)?;
        let item = self.inventory.cache().get_item(guid, item_guid)?;
        let entry = item.read().entry;
        let template = self.item_mgr.get_template(entry)?;

        if template.item_class != 2 {
            return None;
        }

        match template.item_subclass {
            0 => Some(SKILL_AXES),
            1 => Some(SKILL_2H_AXES),
            2 => Some(SKILL_BOWS),
            3 => Some(SKILL_GUNS),
            4 => Some(SKILL_MACES),
            5 => Some(SKILL_2H_MACES),
            6 => Some(SKILL_POLEARMS),
            7 => Some(SKILL_SWORDS),
            8 => Some(SKILL_2H_SWORDS),
            10 => Some(SKILL_STAVES),
            13 => Some(SKILL_FIST_WEAPONS),
            15 => Some(SKILL_DAGGERS),
            16 => Some(SKILL_THROWN),
            18 => Some(SKILL_CROSSBOWS),
            19 => Some(SKILL_WANDS),
            _ => None,
        }
    }

    fn equipped_item_bonuses(&self, guid: ObjectGuid) -> EquippedItemBonuses {
        let mut bonuses = EquippedItemBonuses::default();

        for (slot, item_guid) in self.inventory.cache().get_equipment_slots(guid) {
            let Some(item) = self.inventory.cache().get_item(guid, item_guid) else {
                continue;
            };
            let entry = item.read().entry;
            let Some(template) = self.item_mgr.get_template(entry) else {
                continue;
            };
            bonuses.add_template(slot, &template);
        }

        bonuses
    }

    fn ammo_dps_for_template(template: &ItemTemplate) -> Option<f32> {
        const ITEM_CLASS_PROJECTILE: u32 = 6;

        if template.item_class != ITEM_CLASS_PROJECTILE || template.ammo_type == 0 {
            return None;
        }

        let min: f32 = template.dmg_min.iter().sum();
        let max: f32 = template.dmg_max.iter().sum();
        if min <= 0.0 && max <= 0.0 {
            return None;
        }

        Some((min + max) / 2.0)
    }

    fn calculate_max_health(
        base_health: f32,
        equipped_health: f32,
        stamina_bonus: f32,
        base_value: f32,
        base_pct: f32,
        total_value: f32,
        total_pct: f32,
    ) -> f32 {
        ((base_health + base_value) * base_pct + equipped_health + total_value + stamina_bonus)
            * total_pct
    }

    pub fn on_player_login(&self, guid: ObjectGuid) -> Result<()> {
        self.recalculate_all(guid);
        // Set health/mana to max on login (fresh character state)
        // TODO: Load saved health/mana from character DB
        self.player_mgr.with_player_mut(guid, |player| {
            player.stats.health = player.stats.max_health;
            player.stats.mana = player.stats.max_mana;
        });
        Ok(())
    }

    pub fn on_player_logout(&self, _guid: ObjectGuid) -> Result<()> {
        Ok(())
    }

    // ========== Core API ==========

    /// Full stat recalculation for a player
    pub fn recalculate_all(&self, guid: ObjectGuid) {
        let base_stats = match self.base_stats.get() {
            Some(bs) => bs,
            None => return,
        };
        let equipped_bonuses = self.equipped_item_bonuses(guid);

        self.player_mgr.with_player_mut(guid, |player| {
            let race = player.race;
            let class = player.class;
            let level = player.level;
            let max_skill_for_level = get_skill_max_for_level(level);
            let defense_skill = player
                .skills
                .skills
                .get(&SKILL_DEFENSE)
                .filter(|skill| skill.state != SkillSaveState::Deleted)
                .map(|skill| skill.current_value)
                .unwrap_or(max_skill_for_level);
            let defense_bonus = derived::defense_skill_bonus(defense_skill, max_skill_for_level);

            // 1. Load base stats from DB
            let base = base_stats.get_level_stats(race, class, level);
            let class_base = base_stats.get_class_level_stats(class, level);

            player.stats.base_health = class_base.base_health;
            player.stats.base_mana = class_base.base_mana;

            // 2. Calculate effective stats using modifier formula
            let strength = player.stats.unit_mods.calculate_total_value(
                UnitMods::StatStrength,
                (base.strength as i32 + equipped_bonuses.stats[0]).max(0) as f32,
            );
            let agility = player.stats.unit_mods.calculate_total_value(
                UnitMods::StatAgility,
                (base.agility as i32 + equipped_bonuses.stats[1]).max(0) as f32,
            );
            let stamina = player.stats.unit_mods.calculate_total_value(
                UnitMods::StatStamina,
                (base.stamina as i32 + equipped_bonuses.stats[2]).max(0) as f32,
            );
            let intellect = player.stats.unit_mods.calculate_total_value(
                UnitMods::StatIntellect,
                (base.intellect as i32 + equipped_bonuses.stats[3]).max(0) as f32,
            );
            let spirit = player.stats.unit_mods.calculate_total_value(
                UnitMods::StatSpirit,
                (base.spirit as i32 + equipped_bonuses.stats[4]).max(0) as f32,
            );

            player.stats.strength = strength.max(0.0) as u32;
            player.stats.agility = agility.max(0.0) as u32;
            player.stats.stamina = stamina.max(0.0) as u32;
            player.stats.intellect = intellect.max(0.0) as u32;
            player.stats.spirit = spirit.max(0.0) as u32;

            // 3. Health: base_health + stamina_bonus, modified by Health unit mods
            let stamina_bonus = derived::health_bonus_from_stamina(stamina);
            let health_base_value = player
                .stats
                .unit_mods
                .get_modifier_value(UnitMods::Health, UnitModifierType::BaseValue);
            let health_base_pct = player
                .stats
                .unit_mods
                .get_modifier_value(UnitMods::Health, UnitModifierType::BasePct);
            let health_total_value = player
                .stats
                .unit_mods
                .get_modifier_value(UnitMods::Health, UnitModifierType::TotalValue);
            let health_total_pct = player
                .stats
                .unit_mods
                .get_modifier_value(UnitMods::Health, UnitModifierType::TotalPct);

            let max_health = Self::calculate_max_health(
                class_base.base_health as f32,
                equipped_bonuses.max_health as f32,
                stamina_bonus,
                health_base_value,
                health_base_pct,
                health_total_value,
                health_total_pct,
            );
            let old_max_health = player.stats.max_health;
            player.stats.max_health = max_health.max(1.0) as u32;

            // Preserve health ratio when max health changes
            if old_max_health > 0 && player.stats.health > 0 {
                let ratio = player.stats.health as f32 / old_max_health as f32;
                player.stats.health = (player.stats.max_health as f32 * ratio).max(1.0) as u32;
            }
            if player.stats.health > player.stats.max_health {
                player.stats.health = player.stats.max_health;
            }

            // 4. Mana: base_mana + intellect_bonus, modified by Mana unit mods
            let power_type = derived::power_type_for_class(class);
            if power_type == 0 {
                // Mana class
                let int_bonus = derived::mana_bonus_from_intellect(intellect);
                let mana_base_value = player
                    .stats
                    .unit_mods
                    .get_modifier_value(UnitMods::Mana, UnitModifierType::BaseValue);
                let mana_base_pct = player
                    .stats
                    .unit_mods
                    .get_modifier_value(UnitMods::Mana, UnitModifierType::BasePct);
                let mana_total_value = player
                    .stats
                    .unit_mods
                    .get_modifier_value(UnitMods::Mana, UnitModifierType::TotalValue);
                let mana_total_pct = player
                    .stats
                    .unit_mods
                    .get_modifier_value(UnitMods::Mana, UnitModifierType::TotalPct);

                let max_mana = ((class_base.base_mana as f32
                    + equipped_bonuses.max_mana as f32
                    + mana_base_value)
                    * mana_base_pct
                    + mana_total_value
                    + int_bonus)
                    * mana_total_pct;
                player.stats.max_mana = max_mana.max(0.0) as u32;
            }

            // Every power type has a UnitMods slot. Mana retains its class/intellect formula above;
            // non-mana powers start at their expected fixed maximum and use the common modifier
            // formula. Do not rescale current power here: aura handlers apply the max delta via
            // ModifyPower after this recalculation.
            for power in 0u8..5 {
                let power_type = PowerType::from_u8(power).unwrap();
                let max_power = if power_type == PowerType::Mana {
                    player.stats.max_mana
                } else {
                    calculate_non_mana_max_power(&player.stats.unit_mods, power)
                };
                player.power.max[power as usize] = max_power;
            }

            // 5. Attack power
            // Aura AP: flat mods add to the base + equipped total, then the percent mods
            // apply a multiplier (matches MaNGOS HandleAttackPowerModifier flat vs AP_MOD_PCT
            // buckets — final = (base + flat) * (1 + pct/100)).
            {
                use crate::game::player::auras::effects::{
                    AURA_MOD_ATTACK_POWER, AURA_MOD_ATTACK_POWER_PCT, AURA_MOD_RANGED_ATTACK_POWER,
                    AURA_MOD_RANGED_ATTACK_POWER_PCT,
                };

                let melee_ap = derived::calculate_melee_ap(class, level, strength, agility);
                let melee_flat = player
                    .auras
                    .container
                    .get_total_aura_modifier(AURA_MOD_ATTACK_POWER);
                let melee_pct = player
                    .auras
                    .container
                    .get_total_aura_modifier(AURA_MOD_ATTACK_POWER_PCT);
                let melee_base = melee_ap + equipped_bonuses.melee_attack_power as f32;
                player.stats.melee_attack_power =
                    combine_attack_power(melee_base, melee_flat, melee_pct);

                let ranged_ap = derived::calculate_ranged_ap(class, level, agility);
                let ranged_flat = player
                    .auras
                    .container
                    .get_total_aura_modifier(AURA_MOD_RANGED_ATTACK_POWER);
                let ranged_pct = player
                    .auras
                    .container
                    .get_total_aura_modifier(AURA_MOD_RANGED_ATTACK_POWER_PCT);
                let ranged_base = ranged_ap + equipped_bonuses.ranged_attack_power as f32;
                player.stats.ranged_attack_power =
                    combine_attack_power(ranged_base, ranged_flat, ranged_pct);
            }

            // 6. Armor: agility bonus + equipment (via UnitMods::Armor)
            let agi_armor = derived::armor_from_agility(agility);
            let armor_total = player
                .stats
                .unit_mods
                .calculate_total_value(UnitMods::Armor, 0.0)
                + equipped_bonuses.armor as f32
                + agi_armor;
            player.stats.armor = armor_total.max(0.0) as u32;
            player.stats.resistances[0] = player.stats.armor;

            // 7. Resistances (schools 1-6)
            for school in 1..7u8 {
                if let Some(unit_mod) = UnitMods::from_resistance(school) {
                    let value = player.stats.unit_mods.calculate_total_value(unit_mod, 0.0)
                        + equipped_bonuses.resistances[school as usize] as f32;
                    player.stats.resistances[school as usize] = value.max(0.0) as u32;
                }
            }

            // 8. Spell power and healing power from auras (AURA_MOD_DAMAGE_DONE / AURA_MOD_HEALING_DONE)
            {
                use crate::game::player::auras::effects::{
                    AURA_MOD_DAMAGE_DONE, AURA_MOD_HEALING_DONE,
                };
                // Reset spell power for each school, then accumulate from auras
                for school in 0..7usize {
                    // AURA_MOD_DAMAGE_DONE uses misc_value as school bitmask (1 << school)
                    let school_mask = 1i32 << school;
                    let from_auras: i32 = player
                        .auras
                        .container
                        .get_auras_by_type(AURA_MOD_DAMAGE_DONE)
                        .iter()
                        .filter(|a| (a.misc_value & school_mask) != 0)
                        .map(|a| a.current_value())
                        .sum();
                    player.stats.spell_power[school] =
                        (from_auras + equipped_bonuses.spell_power).max(0) as u32;
                }
                // Healing power: AURA_MOD_HEALING_DONE (misc_value irrelevant, always applies)
                let healing_from_auras = player
                    .auras
                    .container
                    .get_total_aura_modifier(AURA_MOD_HEALING_DONE);
                player.stats.healing_power =
                    (healing_from_auras + equipped_bonuses.healing_power).max(0) as u32;
            }

            // 9. Crit
            let agi_crit = derived::melee_crit_from_agility(class, level, agility);
            let base_crit = derived::class_base_crit(class);
            let melee_weapon_bonus = self
                .equipped_weapon_skill(guid, EquipmentSlot::Mainhand)
                .and_then(|skill_id| player.skills.skills.get(&skill_id))
                .filter(|skill| skill.state != SkillSaveState::Deleted)
                .map(|skill| {
                    derived::weapon_skill_crit_bonus(skill.current_value, max_skill_for_level)
                })
                .unwrap_or(0.0);
            let aura_melee_crit =
                player.auras.container.get_total_aura_modifier(
                    crate::game::player::auras::effects::AURA_MOD_CRIT_PERCENT,
                ) as f32;
            player.stats.melee_crit_pct =
                (base_crit + agi_crit + melee_weapon_bonus + aura_melee_crit).max(0.0);

            let ranged_agi_crit = derived::ranged_crit_from_agility(class, level, agility);
            let ranged_weapon_bonus = self
                .equipped_weapon_skill(guid, EquipmentSlot::Ranged)
                .and_then(|skill_id| player.skills.skills.get(&skill_id))
                .filter(|skill| skill.state != SkillSaveState::Deleted)
                .map(|skill| {
                    derived::weapon_skill_crit_bonus(skill.current_value, max_skill_for_level)
                })
                .unwrap_or(0.0);
            player.stats.ranged_crit_pct =
                (base_crit + ranged_agi_crit + ranged_weapon_bonus + aura_melee_crit).max(0.0);

            // 9b. Spell crit (from intellect, class-specific, + aura bonus)
            let int_spell_crit = derived::spell_crit_from_intellect(class, level, intellect);
            let base_spell_crit = derived::class_base_spell_crit(class);
            let aura_spell_crit = player.auras.container.get_total_aura_modifier(
                crate::game::player::auras::effects::AURA_MOD_SPELL_CRIT_CHANCE,
            ) as f32;
            player.stats.spell_crit_pct =
                (base_spell_crit + int_spell_crit + aura_spell_crit).max(0.0);

            // 10. Dodge
            let agi_dodge = derived::dodge_from_agility(class, level, agility);
            let base_dodge = derived::class_base_dodge(class);
            let aura_dodge = player.auras.container.get_total_aura_modifier(
                crate::game::player::auras::effects::AURA_MOD_DODGE_PERCENT,
            ) as f32;
            player.stats.dodge_pct = (base_dodge + agi_dodge + defense_bonus + aura_dodge).max(0.0);

            // 11. Parry / Block (base 5%, requires abilities)
            let aura_parry = player.auras.container.get_total_aura_modifier(
                crate::game::player::auras::effects::AURA_MOD_PARRY_PERCENT,
            ) as f32;
            player.stats.parry_pct = (5.0 + defense_bonus + aura_parry).max(0.0);

            let aura_block = player.auras.container.get_total_aura_modifier(
                crate::game::player::auras::effects::AURA_MOD_BLOCK_PERCENT,
            ) as f32;
            player.stats.block_pct = (5.0 + defense_bonus + aura_block).max(0.0);
            let base_block_value = player
                .stats
                .base_mods
                .get_total_base_mod_value(BaseModGroup::ShieldBlockValue);
            player.stats.block_value =
                (equipped_bonuses.block_value as f32 + base_block_value).max(0.0) as u32;

            // 12. Damage ranges
            let default_speed_ms: u32 = 2000;
            let melee_ap_for_damage = player.stats.melee_attack_power.max(0) as f32;
            let ranged_ap_for_damage = player.stats.ranged_attack_power.max(0) as f32;

            if let Some(mainhand) = equipped_bonuses.mainhand_damage {
                let ap_dmg = derived::ap_damage_modifier(melee_ap_for_damage, mainhand.delay_ms);
                let min = (mainhand.min + ap_dmg).max(0.0);
                let max = (mainhand.max + ap_dmg).max(min);
                player.stats.min_damage = player
                    .stats
                    .unit_mods
                    .calculate_total_value(UnitMods::DamageMainhand, min)
                    .max(0.0);
                player.stats.max_damage = player
                    .stats
                    .unit_mods
                    .calculate_total_value(UnitMods::DamageMainhand, max)
                    .max(player.stats.min_damage);
            } else {
                let ap_dmg = derived::ap_damage_modifier(melee_ap_for_damage, default_speed_ms);
                player.stats.min_damage = (1.0 + ap_dmg).max(0.0);
                player.stats.max_damage = (2.0 + ap_dmg).max(player.stats.min_damage);
            }

            if let Some(offhand) = equipped_bonuses.offhand_damage {
                let ap_dmg = derived::ap_damage_modifier(melee_ap_for_damage, offhand.delay_ms);
                let min = (offhand.min + ap_dmg).max(0.0);
                let max = (offhand.max + ap_dmg).max(min);
                player.stats.min_offhand_damage = player
                    .stats
                    .unit_mods
                    .calculate_total_value(UnitMods::DamageOffhand, min)
                    .max(0.0);
                player.stats.max_offhand_damage = player
                    .stats
                    .unit_mods
                    .calculate_total_value(UnitMods::DamageOffhand, max)
                    .max(player.stats.min_offhand_damage);
            } else {
                player.stats.min_offhand_damage = 0.0;
                player.stats.max_offhand_damage = 0.0;
            }

            if let Some(ranged) = equipped_bonuses.ranged_damage {
                let ammo_dps = if ranged.ammo_type == 0 || player.ammo_id == 0 {
                    0.0
                } else {
                    self.item_mgr
                        .get_template(player.ammo_id)
                        .filter(|ammo| ammo.ammo_type == ranged.ammo_type)
                        .and_then(|ammo| Self::ammo_dps_for_template(&ammo))
                        .unwrap_or(0.0)
                };
                let ap_dmg = derived::ap_damage_modifier(ranged_ap_for_damage, ranged.delay_ms);
                let ammo_damage = ammo_dps * (ranged.delay_ms as f32 / 1000.0);
                let min = (ranged.min + ap_dmg + ammo_damage).max(0.0);
                let max = (ranged.max + ap_dmg + ammo_damage).max(min);
                player.stats.min_ranged_damage = player
                    .stats
                    .unit_mods
                    .calculate_total_value(UnitMods::DamageRanged, min)
                    .max(0.0);
                player.stats.max_ranged_damage = player
                    .stats
                    .unit_mods
                    .calculate_total_value(UnitMods::DamageRanged, max)
                    .max(player.stats.min_ranged_damage);
            } else {
                player.stats.min_ranged_damage = 0.0;
                player.stats.max_ranged_damage = 0.0;
            }

            // 13. Mana regen
            {
                use crate::game::player::auras::effects::{
                    AURA_MOD_MANA_REGEN_INTERRUPT, AURA_MOD_POWER_REGEN,
                    AURA_MOD_POWER_REGEN_PERCENT,
                };

                let spirit_regen = derived::mana_regen_from_spirit(class, spirit);
                let regen_multiplier = player
                    .auras
                    .container
                    .get_total_aura_multiplier_by_misc(AURA_MOD_POWER_REGEN_PERCENT, 0);
                let flat_mp5 = player
                    .auras
                    .container
                    .get_total_aura_modifier_by_misc(AURA_MOD_POWER_REGEN, 0)
                    as f32;
                let interrupt_percent = player
                    .auras
                    .container
                    .get_total_aura_modifier(AURA_MOD_MANA_REGEN_INTERRUPT)
                    as f32;
                let (full_regen, interrupt_regen) = derived::calculate_mana_regen_rates(
                    spirit_regen,
                    regen_multiplier,
                    flat_mp5,
                    interrupt_percent,
                );

                player.stats.mana_regen_base = full_regen;
                player.stats.mana_regen_interrupt = interrupt_regen;
            }

            player.stats.dirty = true;
        });
    }

    /// Called when a player levels up
    pub fn on_level_up(&self, guid: ObjectGuid) {
        let base_stats = match self.base_stats.get() {
            Some(bs) => bs,
            None => return,
        };

        // Capture old stats for delta calculation
        let old_stats = self.player_mgr.with_player_mut(guid, |player| {
            (
                player.stats.strength,
                player.stats.agility,
                player.stats.stamina,
                player.stats.intellect,
                player.stats.spirit,
                player.stats.max_health,
                player.stats.max_mana,
                player.level,
                player.class,
            )
        });

        // Recalculate all stats with new level
        self.recalculate_all(guid);

        // Set health/mana to max on level-up
        self.player_mgr.with_player_mut(guid, |player| {
            player.stats.health = player.stats.max_health;
            player.stats.mana = player.stats.max_mana;
        });

        // Build stat delta for SMSG_LEVELUP_INFO
        if let Some((
            old_str,
            old_agi,
            old_sta,
            old_int,
            old_spi,
            old_hp,
            old_mana,
            _old_level,
            _class,
        )) = old_stats
        {
            let new_stats = self.player_mgr.with_player_mut(guid, |player| {
                (
                    player.stats.strength,
                    player.stats.agility,
                    player.stats.stamina,
                    player.stats.intellect,
                    player.stats.spirit,
                    player.stats.max_health,
                    player.stats.max_mana,
                )
            });

            if let Some((new_str, new_agi, new_sta, new_int, new_spi, new_hp, new_mana)) = new_stats
            {
                // Store deltas for experience system to read
                // The experience system sends SMSG_LEVELUP_INFO with these values
                let _str_gain = new_str.saturating_sub(old_str);
                let _agi_gain = new_agi.saturating_sub(old_agi);
                let _sta_gain = new_sta.saturating_sub(old_sta);
                let _int_gain = new_int.saturating_sub(old_int);
                let _spi_gain = new_spi.saturating_sub(old_spi);
                let _hp_gain = new_hp.saturating_sub(old_hp);
                let _mana_gain = new_mana.saturating_sub(old_mana);
            }
        }
    }

    /// Get stat deltas from a level-up (for SMSG_LEVELUP_INFO)
    pub fn get_level_up_gains(
        &self,
        race: u8,
        class: u8,
        old_level: u8,
        new_level: u8,
    ) -> (u32, u32, [u32; 5]) {
        let base_stats = match self.base_stats.get() {
            Some(bs) => bs,
            None => return (0, 0, [0; 5]),
        };

        let old_base = base_stats.get_level_stats(race, class, old_level);
        let new_base = base_stats.get_level_stats(race, class, new_level);
        let old_class = base_stats.get_class_level_stats(class, old_level);
        let new_class = base_stats.get_class_level_stats(class, new_level);

        let stat_gains = [
            new_base.strength.saturating_sub(old_base.strength),
            new_base.agility.saturating_sub(old_base.agility),
            new_base.stamina.saturating_sub(old_base.stamina),
            new_base.intellect.saturating_sub(old_base.intellect),
            new_base.spirit.saturating_sub(old_base.spirit),
        ];

        // HP/mana gains from base tables + stamina/intellect scaling
        let old_sta_bonus = derived::health_bonus_from_stamina(old_base.stamina as f32);
        let new_sta_bonus = derived::health_bonus_from_stamina(new_base.stamina as f32);
        let hp_gain = (new_class.base_health + new_sta_bonus as u32)
            .saturating_sub(old_class.base_health + old_sta_bonus as u32);

        let old_int_bonus = derived::mana_bonus_from_intellect(old_base.intellect as f32);
        let new_int_bonus = derived::mana_bonus_from_intellect(new_base.intellect as f32);
        let mana_gain = (new_class.base_mana + new_int_bonus as u32)
            .saturating_sub(old_class.base_mana + old_int_bonus as u32);

        (hp_gain, mana_gain, stat_gains)
    }

    /// Send SMSG_UPDATE_OBJECT with all stat fields to client and nearby players
    pub fn send_stat_update(&self, guid: ObjectGuid) {
        let stats = self.player_mgr.with_player_mut(guid, |player| {
            // Sync current mana from power system into stats for the update packet
            player.stats.mana = player.power.current[0];
            player.stats.dirty = false;
            (
                player.stats.health,
                player.stats.max_health,
                player.stats.mana,
                player.stats.max_mana,
                player.stats.strength,
                player.stats.agility,
                player.stats.stamina,
                player.stats.intellect,
                player.stats.spirit,
                player.stats.melee_attack_power,
                player.stats.ranged_attack_power,
                player.stats.armor,
                player.stats.resistances,
                player.stats.melee_crit_pct,
                player.stats.ranged_crit_pct,
                player.stats.dodge_pct,
                player.stats.parry_pct,
                player.stats.block_pct,
                player.stats.block_value,
                player.ammo_id,
                player.stats.min_damage,
                player.stats.max_damage,
                player.stats.min_offhand_damage,
                player.stats.max_offhand_damage,
                player.stats.min_ranged_damage,
                player.stats.max_ranged_damage,
            )
        });

        let Some((
            health,
            max_health,
            mana,
            max_mana,
            str_val,
            agi,
            sta,
            int,
            spi,
            melee_ap,
            ranged_ap,
            armor,
            resistances,
            melee_crit,
            ranged_crit,
            dodge,
            parry,
            block,
            _block_value,
            ammo_id,
            min_dmg,
            max_dmg,
            min_oh_dmg,
            max_oh_dmg,
            min_rng_dmg,
            max_rng_dmg,
        )) = stats
        else {
            return;
        };

        let world_guid = WorldObjectGuid::from_raw(guid.raw());
        let values_block = ValuesUpdateBlock::new(world_guid, ObjectType::Player)
            .set_field(UNIT_FIELD_HEALTH, health)
            .set_field(UNIT_FIELD_MAXHEALTH, max_health)
            .set_field(UNIT_FIELD_POWER1, mana)
            .set_field(UNIT_FIELD_MAXPOWER1, max_mana)
            .set_field(UNIT_FIELD_STAT0, str_val)
            .set_field(UNIT_FIELD_STAT1, agi)
            .set_field(UNIT_FIELD_STAT2, sta)
            .set_field(UNIT_FIELD_STAT3, int)
            .set_field(UNIT_FIELD_STAT4, spi)
            .set_field(UNIT_FIELD_ATTACK_POWER, melee_ap as u32)
            .set_field(UNIT_FIELD_RANGED_ATTACK_POWER, ranged_ap as u32)
            // Resistances
            .set_field(UNIT_FIELD_RESISTANCES, armor)
            .set_field(UNIT_FIELD_RESISTANCES + 1, resistances[1])
            .set_field(UNIT_FIELD_RESISTANCES + 2, resistances[2])
            .set_field(UNIT_FIELD_RESISTANCES + 3, resistances[3])
            .set_field(UNIT_FIELD_RESISTANCES + 4, resistances[4])
            .set_field(UNIT_FIELD_RESISTANCES + 5, resistances[5])
            .set_field(UNIT_FIELD_RESISTANCES + 6, resistances[6])
            // Damage ranges
            .set_float_field(UNIT_FIELD_MINDAMAGE, min_dmg)
            .set_float_field(UNIT_FIELD_MAXDAMAGE, max_dmg)
            .set_float_field(UNIT_FIELD_MINOFFHANDDAMAGE, min_oh_dmg)
            .set_float_field(UNIT_FIELD_MAXOFFHANDDAMAGE, max_oh_dmg)
            .set_float_field(UNIT_FIELD_MINRANGEDDAMAGE, min_rng_dmg)
            .set_float_field(UNIT_FIELD_MAXRANGEDDAMAGE, max_rng_dmg)
            // Combat percentages
            .set_float_field(PLAYER_CRIT_PERCENTAGE, melee_crit)
            .set_float_field(PLAYER_RANGED_CRIT_PERCENTAGE, ranged_crit)
            .set_float_field(PLAYER_DODGE_PERCENTAGE, dodge)
            .set_float_field(PLAYER_PARRY_PERCENTAGE, parry)
            .set_float_field(PLAYER_BLOCK_PERCENTAGE, block)
            .set_field(PLAYER_AMMO_ID, ammo_id);

        let update_msg = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(values_block));
        let packet = update_msg.to_world_packet();

        self.broadcast_mgr.broadcast_nearby(guid, &packet, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── combine_attack_power (MaNGOS HandleAttackPowerModifier buckets) ───────

    #[test]
    fn combine_attack_power_flat_only() {
        // Battle Shout: +140 flat AP on a 200 base.
        assert_eq!(combine_attack_power(200.0, 140, 0), 340);
    }

    #[test]
    fn combine_attack_power_percent_applies_after_flat() {
        // (200 + 100) * (1 + 0.10) = 330.
        assert_eq!(combine_attack_power(200.0, 100, 10), 330);
    }

    #[test]
    fn combine_attack_power_negative_flat_and_clamped_at_zero() {
        // Curse of Weakness: negative flat AP; result never goes below zero.
        assert_eq!(combine_attack_power(50.0, -80, 0), 0);
    }

    #[test]
    fn combine_attack_power_negative_percent() {
        // Demoralizing Shout: -10% AP → (300) * 0.9 = 270.
        assert_eq!(combine_attack_power(300.0, 0, -10), 270);
    }

    #[test]
    fn combine_attack_power_no_mods_is_base() {
        assert_eq!(combine_attack_power(250.0, 0, 0), 250);
    }

    fn item_template_with_bonuses() -> ItemTemplate {
        ItemTemplate {
            entry: 1,
            name: "Equipped Test Item".to_string(),
            display_id: 0,
            quality: 0,
            item_level: 1,
            required_level: 1,
            item_class: 0,
            item_subclass: 0,
            inventory_type: 0,
            max_count: 0,
            stackable: 1,
            max_durability: 0,
            buy_count: 1,
            buy_price: 0,
            sell_price: 0,
            bag_family: 0,
            container_slots: 0,
            start_quest: 0,
            stat_type: [4, 3, 7, 5, 6, 0, 1, 38, 39, 255],
            stat_value: [10, 11, 12, 13, 14, 100, 200, 30, 40, 999],
            delay: 0,
            ammo_type: 0,
            dmg_min: [0.0; 5],
            dmg_max: [0.0; 5],
            dmg_type: [0; 5],
            block: 25,
            armor: 50,
            holy_res: 1,
            fire_res: 2,
            nature_res: 3,
            frost_res: 4,
            shadow_res: 5,
            arcane_res: 6,
            spell_id: [0; 5],
            spell_trigger: [0; 5],
            spell_charges: [0; 5],
            spell_cooldown: [0; 5],
            spell_category: [0; 5],
            spell_category_cooldown: [0; 5],
        }
    }

    #[test]
    fn equipped_item_bonuses_map_core_stats_and_resistances() {
        let mut bonuses = EquippedItemBonuses::default();
        bonuses.add_template(EquipmentSlot::Chest as u8, &item_template_with_bonuses());

        assert_eq!(bonuses.stats, [10, 11, 12, 13, 14]);
        assert_eq!(bonuses.max_mana, 100);
        assert_eq!(bonuses.max_health, 200);
        assert_eq!(bonuses.melee_attack_power, 30);
        assert_eq!(bonuses.ranged_attack_power, 40);
        assert_eq!(bonuses.block_value, 25);
        assert_eq!(bonuses.armor, 50);
        assert_eq!(bonuses.resistances, [50, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn max_health_applies_equipped_health_after_base_percentage() {
        let max_health = StatsSystem::calculate_max_health(100.0, 50.0, 0.0, 20.0, 1.5, 10.0, 1.0);

        assert_eq!(max_health, 240.0);
    }

    #[test]
    fn non_mana_power_max_uses_unit_modifiers_and_base_values() {
        let mut unit_mods = super::super::modifiers::UnitModifierGroup::new();
        unit_mods.handle_stat_modifier(UnitMods::Energy, UnitModifierType::TotalValue, 20.0, true);
        unit_mods.handle_stat_modifier(UnitMods::Energy, UnitModifierType::TotalPct, 10.0, true);

        assert_eq!(calculate_non_mana_max_power(&unit_mods, 1), 1000);
        assert_eq!(calculate_non_mana_max_power(&unit_mods, 2), 100);
        assert_eq!(calculate_non_mana_max_power(&unit_mods, 3), 132);
        assert_eq!(calculate_non_mana_max_power(&unit_mods, 4), 1_050_000);
    }

    #[test]
    fn equipped_item_bonuses_map_spell_and_healing_power() {
        let mut template = item_template_with_bonuses();
        template.stat_type = [41, 42, 45, 0, 0, 0, 0, 0, 0, 0];
        template.stat_value = [15, 20, 25, 0, 0, 0, 0, 0, 0, 0];

        let mut bonuses = EquippedItemBonuses::default();
        bonuses.add_template(EquipmentSlot::Chest as u8, &template);

        assert_eq!(bonuses.healing_power, 15);
        assert_eq!(bonuses.spell_power, 45);
    }

    #[test]
    fn equipped_item_bonuses_collect_weapon_damage_by_slot() {
        let mut mainhand = item_template_with_bonuses();
        mainhand.dmg_min = [3.0, 1.0, 0.0, 0.0, 0.0];
        mainhand.dmg_max = [8.0, 2.0, 0.0, 0.0, 0.0];
        mainhand.delay = 2400;

        let mut offhand = item_template_with_bonuses();
        offhand.dmg_min = [2.0, 0.0, 0.0, 0.0, 0.0];
        offhand.dmg_max = [5.0, 0.0, 0.0, 0.0, 0.0];
        offhand.delay = 1600;

        let mut ranged = item_template_with_bonuses();
        ranged.dmg_min = [7.0, 0.0, 0.0, 0.0, 0.0];
        ranged.dmg_max = [11.0, 0.0, 0.0, 0.0, 0.0];
        ranged.delay = 2800;

        let mut bonuses = EquippedItemBonuses::default();
        bonuses.add_template(EquipmentSlot::Mainhand as u8, &mainhand);
        bonuses.add_template(EquipmentSlot::Offhand as u8, &offhand);
        bonuses.add_template(EquipmentSlot::Ranged as u8, &ranged);

        let mainhand_damage = bonuses.mainhand_damage.unwrap();
        assert_eq!(mainhand_damage.min, 4.0);
        assert_eq!(mainhand_damage.max, 10.0);
        assert_eq!(mainhand_damage.delay_ms, 2400);

        let offhand_damage = bonuses.offhand_damage.unwrap();
        assert_eq!(offhand_damage.min, 2.0);
        assert_eq!(offhand_damage.max, 5.0);
        assert_eq!(offhand_damage.delay_ms, 1600);

        let ranged_damage = bonuses.ranged_damage.unwrap();
        assert_eq!(ranged_damage.min, 7.0);
        assert_eq!(ranged_damage.max, 11.0);
        assert_eq!(ranged_damage.delay_ms, 2800);
        assert_eq!(ranged_damage.ammo_type, 0);
    }

    #[test]
    fn ammo_dps_requires_projectile_ammo_template() {
        let mut ammo = item_template_with_bonuses();
        ammo.item_class = 6;
        ammo.ammo_type = 2;
        ammo.dmg_min = [4.0, 0.0, 0.0, 0.0, 0.0];
        ammo.dmg_max = [8.0, 0.0, 0.0, 0.0, 0.0];

        assert_eq!(StatsSystem::ammo_dps_for_template(&ammo), Some(6.0));

        ammo.item_class = 2;
        assert_eq!(StatsSystem::ammo_dps_for_template(&ammo), None);

        ammo.item_class = 6;
        ammo.ammo_type = 0;
        assert_eq!(StatsSystem::ammo_dps_for_template(&ammo), None);
    }

    #[test]
    fn ranged_weapon_damage_remembers_required_ammo_type() {
        let mut ranged = item_template_with_bonuses();
        ranged.dmg_min = [7.0, 0.0, 0.0, 0.0, 0.0];
        ranged.dmg_max = [11.0, 0.0, 0.0, 0.0, 0.0];
        ranged.delay = 2800;
        ranged.ammo_type = 3;

        let mut bonuses = EquippedItemBonuses::default();
        bonuses.add_template(EquipmentSlot::Ranged as u8, &ranged);

        assert_eq!(bonuses.ranged_damage.unwrap().ammo_type, 3);
    }
}
