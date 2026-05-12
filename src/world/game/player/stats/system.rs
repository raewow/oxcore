//! Stats System
//!
//! Stateless processor that calculates and broadcasts player stats.
//! Follows the ExperienceSystem pattern.

use anyhow::Result;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tracing::info;

use crate::shared::messages::update::{ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::ObjectGuid;
use crate::world::core::common::guid::ObjectGuid as WorldObjectGuid;
use crate::world::game::common::update_fields::*;
use crate::world::game::broadcast_mgr::{BroadcastManager, BroadcastManagerExt};
use crate::world::game::player::PlayerManager;

use super::base_stats::BaseStatsData;
use super::derived;
use super::modifiers::{UnitModifierType, UnitMods};

/// Stateless stats system
pub struct StatsSystem {
    broadcast_mgr: Arc<BroadcastManager>,
    player_mgr: Arc<PlayerManager>,
    world_pool: Arc<sqlx::MySqlPool>,
    base_stats: OnceLock<BaseStatsData>,
}

impl StatsSystem {
    pub fn new(
        broadcast_mgr: Arc<BroadcastManager>,
        player_mgr: Arc<PlayerManager>,
        world_pool: Arc<sqlx::MySqlPool>,
    ) -> Self {
        Self {
            broadcast_mgr,
            player_mgr,
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

        self.player_mgr.with_player_mut(guid, |player| {
            let race = player.race;
            let class = player.class;
            let level = player.level;

            // 1. Load base stats from DB
            let base = base_stats.get_level_stats(race, class, level);
            let class_base = base_stats.get_class_level_stats(class, level);

            player.stats.base_health = class_base.base_health;
            player.stats.base_mana = class_base.base_mana;

            // 2. Calculate effective stats using modifier formula
            let strength = player
                .stats
                .unit_mods
                .calculate_total_value(UnitMods::StatStrength, base.strength as f32);
            let agility = player
                .stats
                .unit_mods
                .calculate_total_value(UnitMods::StatAgility, base.agility as f32);
            let stamina = player
                .stats
                .unit_mods
                .calculate_total_value(UnitMods::StatStamina, base.stamina as f32);
            let intellect = player
                .stats
                .unit_mods
                .calculate_total_value(UnitMods::StatIntellect, base.intellect as f32);
            let spirit = player
                .stats
                .unit_mods
                .calculate_total_value(UnitMods::StatSpirit, base.spirit as f32);

            player.stats.strength = strength.max(0.0) as u32;
            player.stats.agility = agility.max(0.0) as u32;
            player.stats.stamina = stamina.max(0.0) as u32;
            player.stats.intellect = intellect.max(0.0) as u32;
            player.stats.spirit = spirit.max(0.0) as u32;

            // 3. Health: base_health + stamina_bonus, modified by Health unit mods
            let stamina_bonus = derived::health_bonus_from_stamina(stamina);
            let health_base_value =
                player.stats.unit_mods.get_modifier_value(UnitMods::Health, UnitModifierType::BaseValue);
            let health_base_pct =
                player.stats.unit_mods.get_modifier_value(UnitMods::Health, UnitModifierType::BasePct);
            let health_total_value =
                player.stats.unit_mods.get_modifier_value(UnitMods::Health, UnitModifierType::TotalValue);
            let health_total_pct =
                player.stats.unit_mods.get_modifier_value(UnitMods::Health, UnitModifierType::TotalPct);

            let max_health = ((class_base.base_health as f32 + health_base_value) * health_base_pct
                + health_total_value
                + stamina_bonus)
                * health_total_pct;
            let old_max_health = player.stats.max_health;
            player.stats.max_health = max_health.max(1.0) as u32;

            // Preserve health ratio when max health changes
            if old_max_health > 0 && player.stats.health > 0 {
                let ratio = player.stats.health as f32 / old_max_health as f32;
                player.stats.health =
                    (player.stats.max_health as f32 * ratio).max(1.0) as u32;
            }
            if player.stats.health > player.stats.max_health {
                player.stats.health = player.stats.max_health;
            }

            // 4. Mana: base_mana + intellect_bonus, modified by Mana unit mods
            let power_type = derived::power_type_for_class(class);
            if power_type == 0 {
                // Mana class
                let int_bonus = derived::mana_bonus_from_intellect(intellect);
                let mana_base_value =
                    player.stats.unit_mods.get_modifier_value(UnitMods::Mana, UnitModifierType::BaseValue);
                let mana_base_pct =
                    player.stats.unit_mods.get_modifier_value(UnitMods::Mana, UnitModifierType::BasePct);
                let mana_total_value =
                    player.stats.unit_mods.get_modifier_value(UnitMods::Mana, UnitModifierType::TotalValue);
                let mana_total_pct =
                    player.stats.unit_mods.get_modifier_value(UnitMods::Mana, UnitModifierType::TotalPct);

                let max_mana = ((class_base.base_mana as f32 + mana_base_value) * mana_base_pct
                    + mana_total_value
                    + int_bonus)
                    * mana_total_pct;
                let old_max_mana = player.stats.max_mana;
                player.stats.max_mana = max_mana.max(0.0) as u32;

                if old_max_mana > 0 && player.stats.mana > 0 {
                    let ratio = player.stats.mana as f32 / old_max_mana as f32;
                    player.stats.mana = (player.stats.max_mana as f32 * ratio).max(0.0) as u32;
                }
                if player.stats.mana > player.stats.max_mana {
                    player.stats.mana = player.stats.max_mana;
                }
            } else {
                // Non-mana class (rage/energy)
                player.stats.max_mana = derived::base_max_power(power_type);
                // Don't touch current value for rage/energy
            }

            // 5. Attack power
            let melee_ap = derived::calculate_melee_ap(class, level, strength, agility);
            player.stats.melee_attack_power = melee_ap.max(0.0) as i32;

            let ranged_ap = derived::calculate_ranged_ap(class, level, agility);
            player.stats.ranged_attack_power = ranged_ap.max(0.0) as i32;

            // 6. Armor: agility bonus + equipment (via UnitMods::Armor)
            let agi_armor = derived::armor_from_agility(agility);
            let armor_total = player
                .stats
                .unit_mods
                .calculate_total_value(UnitMods::Armor, 0.0)
                + agi_armor;
            player.stats.armor = armor_total.max(0.0) as u32;
            player.stats.resistances[0] = player.stats.armor;

            // 7. Resistances (schools 1-6)
            for school in 1..7u8 {
                if let Some(unit_mod) = UnitMods::from_resistance(school) {
                    let value = player
                        .stats
                        .unit_mods
                        .calculate_total_value(unit_mod, 0.0);
                    player.stats.resistances[school as usize] = value.max(0.0) as u32;
                }
            }

            // 8. Spell power and healing power from auras (AURA_MOD_DAMAGE_DONE / AURA_MOD_HEALING_DONE)
            {
                use crate::world::game::player::auras::effects::{
                    AURA_MOD_DAMAGE_DONE, AURA_MOD_HEALING_DONE,
                };
                // Reset spell power for each school, then accumulate from auras
                for school in 0..7usize {
                    // AURA_MOD_DAMAGE_DONE uses misc_value as school bitmask (1 << school)
                    let school_mask = 1i32 << school;
                    let from_auras: i32 = player.auras.container
                        .get_auras_by_type(AURA_MOD_DAMAGE_DONE)
                        .iter()
                        .filter(|a| (a.misc_value & school_mask) != 0)
                        .map(|a| a.current_value())
                        .sum();
                    player.stats.spell_power[school] = from_auras.max(0) as u32;
                }
                // Healing power: AURA_MOD_HEALING_DONE (misc_value irrelevant, always applies)
                let healing_from_auras = player.auras.container
                    .get_total_aura_modifier(AURA_MOD_HEALING_DONE);
                player.stats.healing_power = healing_from_auras.max(0) as u32;
            }

            // 9. Crit
            let agi_crit = derived::melee_crit_from_agility(class, level, agility);
            let base_crit = derived::class_base_crit(class);
            let aura_melee_crit = player.auras.container.get_total_aura_modifier(
                crate::world::game::player::auras::effects::AURA_MOD_CRIT_PERCENT,
            ) as f32;
            player.stats.melee_crit_pct = (base_crit + agi_crit + aura_melee_crit).max(0.0);

            let ranged_agi_crit = derived::ranged_crit_from_agility(class, level, agility);
            player.stats.ranged_crit_pct = (base_crit + ranged_agi_crit + aura_melee_crit).max(0.0);

            // 9b. Spell crit (from intellect, class-specific, + aura bonus)
            let int_spell_crit = derived::spell_crit_from_intellect(class, level, intellect);
            let base_spell_crit = derived::class_base_spell_crit(class);
            let aura_spell_crit = player.auras.container.get_total_aura_modifier(
                crate::world::game::player::auras::effects::AURA_MOD_SPELL_CRIT_CHANCE,
            ) as f32;
            player.stats.spell_crit_pct = (base_spell_crit + int_spell_crit + aura_spell_crit).max(0.0);

            // 10. Dodge
            let agi_dodge = derived::dodge_from_agility(class, level, agility);
            let base_dodge = derived::class_base_dodge(class);
            player.stats.dodge_pct = (base_dodge + agi_dodge).max(0.0);

            // 11. Parry / Block (base 5%, requires abilities)
            player.stats.parry_pct = 5.0;
            player.stats.block_pct = 5.0;

            // 12. Damage ranges (bare-hand default: 2.0s speed, AP-based)
            let default_speed_ms: u32 = 2000;
            let ap_dmg = derived::ap_damage_modifier(melee_ap.max(0.0), default_speed_ms);
            player.stats.min_damage = (1.0 + ap_dmg).max(0.0);
            player.stats.max_damage = (2.0 + ap_dmg).max(0.0);
            player.stats.min_offhand_damage = 0.0;
            player.stats.max_offhand_damage = 0.0;
            player.stats.min_ranged_damage = 0.0;
            player.stats.max_ranged_damage = 0.0;

            // 13. Mana regen
            player.stats.mana_regen_base = derived::mana_regen_from_spirit(class, spirit);
            let aura_mana_regen_interrupt = player.auras.container.get_total_aura_modifier(
                crate::world::game::player::auras::effects::AURA_MOD_MANA_REGEN_INTERRUPT,
            ) as f32;
            player.stats.mana_regen_interrupt = aura_mana_regen_interrupt;

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
        if let Some((old_str, old_agi, old_sta, old_int, old_spi, old_hp, old_mana, _old_level, _class)) = old_stats {
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

            if let Some((new_str, new_agi, new_sta, new_int, new_spi, new_hp, new_mana)) = new_stats {
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
                player.stats.min_damage,
                player.stats.max_damage,
                player.stats.min_offhand_damage,
                player.stats.max_offhand_damage,
                player.stats.min_ranged_damage,
                player.stats.max_ranged_damage,
            )
        });

        let Some((
            health, max_health, mana, max_mana,
            str_val, agi, sta, int, spi,
            melee_ap, ranged_ap, armor, resistances,
            melee_crit, ranged_crit, dodge, parry, block,
            min_dmg, max_dmg, min_oh_dmg, max_oh_dmg, min_rng_dmg, max_rng_dmg,
        )) = stats else {
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
            .set_float_field(PLAYER_BLOCK_PERCENTAGE, block);

        let update_msg = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(values_block));
        let packet = update_msg.to_world_packet();

        self.broadcast_mgr
            .broadcast_nearby(guid, &packet, true)
            ;
    }
}
