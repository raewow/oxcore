use crate::shared::protocol::ObjectGuid;
use crate::world::World;
use anyhow::Result;
use rand::Rng;

use super::constants::*;
use super::formulas::*;
use super::skill_up::*;
use super::state::*;

/// Skill system orchestrator.
///
/// Manages all skill operations: learning, removal, skill-up on combat,
/// level-up updates, and proficiency handling. This system is stateless;
/// all skill data lives in the player's SkillState.
pub struct SkillSystem {
    broadcast_mgr: std::sync::Arc<dyn crate::world::game::broadcast_mgr::BroadcastManagerTrait>,
}

/// Proficiency message to send on login / when learning a new proficiency.
pub struct ProficiencyMessage {
    pub item_class: u8,
    pub sub_class_mask: u32,
}

impl SkillSystem {
    pub fn new(
        broadcast_mgr: std::sync::Arc<dyn crate::world::game::broadcast_mgr::BroadcastManagerTrait>,
    ) -> Self {
        Self { broadcast_mgr }
    }

    // =========================================================================
    // Skill Management
    // =========================================================================

    /// Learn a new skill or update an existing skill's values.
    ///
    /// If the player already has the skill:
    /// - Updates current_value (clamped to max_value)
    /// - Updates max_value
    /// - Updates step if non-zero
    /// - Marks state as Changed
    ///
    /// If the player does not have the skill:
    /// - Allocates the next free position in the update fields array
    /// - Inserts a new SkillData with state = New
    ///
    /// If current_value is 0, the skill is marked as Deleted.
    pub fn learn_skill(
        &self,
        player_guid: ObjectGuid,
        skill_id: u16,
        current: u16,
        max: u16,
        step: u16,
        world: &World,
    ) -> Result<()> {
        if skill_id == 0 {
            return Ok(());
        }

        world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                if current == 0 {
                    // Remove skill
                    if let Some(skill_data) = player.skills.skills.get_mut(&skill_id) {
                        skill_data.state = SkillSaveState::Deleted;
                    }
                    return;
                }

                if let Some(skill_data) = player.skills.skills.get_mut(&skill_id) {
                    // Update existing skill
                    skill_data.current_value = current.min(max);
                    skill_data.max_value = max;
                    if step > 0 {
                        skill_data.step = step;
                    }
                    if skill_data.state == SkillSaveState::Deleted {
                        skill_data.state = SkillSaveState::New;
                    } else if skill_data.state != SkillSaveState::New {
                        skill_data.state = SkillSaveState::Changed;
                    }
                } else {
                    // Learn new skill - find free position
                    let position = self.find_free_skill_position(&player.skills);
                    let data = SkillData::new(skill_id, current.min(max), max, step, position);
                    player.skills.skills.insert(skill_id, data);
                }
            });

        Ok(())
    }

    /// Find the next available position in the skill update fields array.
    /// Scans positions 0..255 and returns the first one not occupied by
    /// any existing (non-deleted) skill.
    fn find_free_skill_position(&self, skills: &SkillState) -> usize {
        let mut used: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for skill_data in skills.skills.values() {
            if skill_data.state != SkillSaveState::Deleted {
                used.insert(skill_data.position);
            }
        }
        for pos in 0..PLAYER_MAX_SKILLS {
            if !used.contains(&pos) {
                return pos;
            }
        }
        // Fallback: should never happen (256 slots is more than enough)
        0
    }

    // =========================================================================
    // Combat Skill-Up
    // =========================================================================

    /// Update weapon skill after a successful melee or ranged hit.
    ///
    /// Called by the combat system (Phase 3) after every successful auto-attack
    /// or special attack that lands (outcome is not Miss/Dodge/Parry).
    ///
    /// Flow:
    /// 1. Determine which weapon skill is used (from equipped weapon)
    /// 2. Read current skill value and max
    /// 3. Calculate skill-up chance using pure function
    /// 4. Roll and apply +1 if successful
    /// 5. Mark skill as Changed for UPDATE_OBJECT and database save
    pub fn update_weapon_skills(
        &self,
        player_guid: ObjectGuid,
        target_is_player: bool,
        weapon_skill_id: u16,
        world: &World,
    ) -> Result<bool> {
        // No skill gain in PvP
        if !can_gain_skill_from_target(target_is_player) {
            return Ok(false);
        }

        if weapon_skill_id == 0 {
            return Ok(false);
        }

        // Capture snapshot: current skill, max skill, intellect
        let snapshot = world
            .systems
            .player
            .manager()
            .with_player(player_guid, |player| {
                let skill_data = player.skills.skills.get(&weapon_skill_id)?;
                if skill_data.state == SkillSaveState::Deleted {
                    return None;
                }
                let current = skill_data.current_value;
                let max = skill_data.max_value;
                if current >= max {
                    return None;
                }

                // TODO: Read actual intellect from stats system
                let intellect = 0.0f32; // Placeholder until stats integration

                Some((current, max, intellect))
            });

        let snapshot = match snapshot {
            Some(Some(s)) => s,
            _ => return Ok(false),
        };
        let (current, max, intellect) = snapshot;

        // Pure calculation - no locks held
        let skill_diff = (max - current) as u32;
        let chance = calculate_weapon_skill_up_chance(current, max, intellect, skill_diff);

        // Roll
        let mut rng = rand::thread_rng();
        let roll: f32 = rng.gen_range(0.0..100.0);

        if roll > chance {
            return Ok(false);
        }

        // Apply skill increase
        let increased = world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                if let Some(skill_data) = player.skills.skills.get_mut(&weapon_skill_id) {
                    if skill_data.current_value < skill_data.max_value {
                        skill_data.current_value += 1;
                        if skill_data.state != SkillSaveState::New {
                            skill_data.state = SkillSaveState::Changed;
                        }
                        return true;
                    }
                }
                false
            })
            .unwrap_or(false);

        Ok(increased)
    }

    /// Update defense skill after being hit by a creature.
    ///
    /// Called by the combat system (Phase 3) after the player takes a hit
    /// (any non-miss/dodge/parry outcome from the attacker's perspective).
    ///
    /// Defense skill is special:
    /// - Only improves when the player is HIT (not when dodging/parrying)
    /// - Uses creature level, not weapon skill, in the formula
    /// - Does not benefit from intellect
    pub fn update_defense_skill(
        &self,
        player_guid: ObjectGuid,
        attacker_level: u8,
        attacker_is_player: bool,
        world: &World,
    ) -> Result<bool> {
        // No skill gain in PvP
        if !can_gain_skill_from_target(attacker_is_player) {
            return Ok(false);
        }

        // Capture snapshot
        let snapshot = world
            .systems
            .player
            .manager()
            .with_player(player_guid, |player| {
                let skill_data = player.skills.skills.get(&SKILL_DEFENSE)?;
                if skill_data.state == SkillSaveState::Deleted {
                    return None;
                }
                let current = skill_data.current_value;
                let max = skill_data.max_value;
                if current >= max {
                    return None;
                }
                let player_level = player.level;

                Some((player_level, current, max))
            });

        let (player_level, current, max) = match snapshot {
            Some(Some(s)) => s,
            _ => return Ok(false),
        };

        // Pure calculation
        let chance = calculate_defense_skill_up_chance(player_level, attacker_level, current, max);

        // Roll
        let mut rng = rand::thread_rng();
        let roll: f32 = rng.gen_range(0.0..100.0);

        if roll > chance {
            return Ok(false);
        }

        // Apply
        let increased = world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                if let Some(skill_data) = player.skills.skills.get_mut(&SKILL_DEFENSE) {
                    if skill_data.current_value < skill_data.max_value {
                        skill_data.current_value += 1;
                        if skill_data.state != SkillSaveState::New {
                            skill_data.state = SkillSaveState::Changed;
                        }
                        return true;
                    }
                }
                false
            })
            .unwrap_or(false);

        Ok(increased)
    }

    // =========================================================================
    // Level-Up
    // =========================================================================

    /// Update all level-dependent skills when the player levels up.
    ///
    /// For every skill with SkillRangeType::Level, the max_value is updated
    /// to new_level * 5. Current value is NOT changed (the player must use
    /// the weapon to raise it). Skills with SKILL_FLAG_ALWAYS_MAX_VALUE are
    /// set to the new max automatically.
    ///
    /// Called by the leveling system after the player gains a level.
    pub fn update_skills_for_level(
        &self,
        player_guid: ObjectGuid,
        new_level: u8,
        world: &World,
    ) -> Result<()> {
        let new_max = get_skill_max_for_level(new_level);

        world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                for skill_data in player.skills.skills.values_mut() {
                    if skill_data.state == SkillSaveState::Deleted {
                        continue;
                    }

                    // Only update level-based skills (weapons, defense)
                    // In a full implementation, we would look up SkillRangeType from DBC.
                    // For now, we update skills whose max_value is a multiple of 5
                    // and is close to a level-based max (within one level's range).
                    //
                    // TODO: Use DBC lookup for SkillRangeType instead of heuristic
                    let is_level_based = matches!(
                        skill_data.skill_id,
                        SKILL_DEFENSE
                            | SKILL_SWORDS
                            | SKILL_AXES
                            | SKILL_BOWS
                            | SKILL_GUNS
                            | SKILL_MACES
                            | SKILL_2H_SWORDS
                            | SKILL_2H_AXES
                            | SKILL_2H_MACES
                            | SKILL_POLEARMS
                            | SKILL_STAVES
                            | SKILL_DAGGERS
                            | SKILL_THROWN
                            | SKILL_CROSSBOWS
                            | SKILL_WANDS
                            | SKILL_FIST_WEAPONS
                            | SKILL_UNARMED
                    );

                    if !is_level_based {
                        continue;
                    }

                    let old_max = skill_data.max_value;
                    skill_data.max_value = new_max;

                    // Defense skill is auto-maxed on level up
                    if skill_data.skill_id == SKILL_DEFENSE {
                        skill_data.current_value = new_max;
                    }

                    // Clamp current to new max (shouldn't happen, but safety)
                    if skill_data.current_value > new_max {
                        skill_data.current_value = new_max;
                    }

                    if old_max != new_max || skill_data.current_value != new_max {
                        if skill_data.state != SkillSaveState::New {
                            skill_data.state = SkillSaveState::Changed;
                        }
                    }
                }
            });

        Ok(())
    }

    // =========================================================================
    // Login & Equip
    // =========================================================================

    /// Called when a player logs in.
    /// Sends SMSG_SET_PROFICIENCY packets for all weapon and armor proficiencies
    /// the player currently has, so the client knows which items are equippable.
    pub fn on_player_login(
        &self,
        player_guid: ObjectGuid,
        world: &World,
    ) -> Result<Vec<ProficiencyMessage>> {
        let mut messages = Vec::new();

        world
            .systems
            .player
            .manager()
            .with_player(player_guid, |player| {
                // Collect weapon proficiency mask
                let mut weapon_mask: u32 = 0;
                let mut armor_mask: u32 = 0;

                for skill_data in player.skills.skills.values() {
                    if skill_data.state == SkillSaveState::Deleted {
                        continue;
                    }

                    match skill_data.skill_id {
                        // Weapon subclass bits (item_class = 2)
                        SKILL_AXES => weapon_mask |= ITEM_SUBCLASS_WEAPON_AXE,
                        SKILL_2H_AXES => weapon_mask |= ITEM_SUBCLASS_WEAPON_AXE2,
                        SKILL_BOWS => weapon_mask |= ITEM_SUBCLASS_WEAPON_BOW,
                        SKILL_GUNS => weapon_mask |= ITEM_SUBCLASS_WEAPON_GUN,
                        SKILL_MACES => weapon_mask |= ITEM_SUBCLASS_WEAPON_MACE,
                        SKILL_2H_MACES => weapon_mask |= ITEM_SUBCLASS_WEAPON_MACE2,
                        SKILL_POLEARMS => weapon_mask |= ITEM_SUBCLASS_WEAPON_POLEARM,
                        SKILL_SWORDS => weapon_mask |= ITEM_SUBCLASS_WEAPON_SWORD,
                        SKILL_2H_SWORDS => weapon_mask |= ITEM_SUBCLASS_WEAPON_SWORD2,
                        SKILL_STAVES => weapon_mask |= ITEM_SUBCLASS_WEAPON_STAFF,
                        SKILL_UNARMED => weapon_mask |= ITEM_SUBCLASS_WEAPON_FIST,
                        SKILL_DAGGERS => weapon_mask |= ITEM_SUBCLASS_WEAPON_DAGGER,
                        SKILL_THROWN => weapon_mask |= ITEM_SUBCLASS_WEAPON_THROWN,
                        SKILL_CROSSBOWS => weapon_mask |= ITEM_SUBCLASS_WEAPON_CROSSBOW,
                        SKILL_WANDS => weapon_mask |= ITEM_SUBCLASS_WEAPON_WAND,
                        SKILL_FIST_WEAPONS => weapon_mask |= ITEM_SUBCLASS_WEAPON_FIST,

                        // Armor subclass bits (item_class = 4)
                        SKILL_CLOTH => armor_mask |= ITEM_SUBCLASS_ARMOR_CLOTH,
                        SKILL_LEATHER => armor_mask |= ITEM_SUBCLASS_ARMOR_LEATHER,
                        SKILL_MAIL => armor_mask |= ITEM_SUBCLASS_ARMOR_MAIL,
                        SKILL_PLATE_MAIL => armor_mask |= ITEM_SUBCLASS_ARMOR_PLATE,
                        SKILL_SHIELD => armor_mask |= ITEM_SUBCLASS_ARMOR_SHIELD,

                        _ => {}
                    }
                }

                if weapon_mask != 0 {
                    messages.push(ProficiencyMessage {
                        item_class: ITEM_CLASS_WEAPON,
                        sub_class_mask: weapon_mask,
                    });
                }
                if armor_mask != 0 {
                    messages.push(ProficiencyMessage {
                        item_class: ITEM_CLASS_ARMOR,
                        sub_class_mask: armor_mask,
                    });
                }
            });

        Ok(messages)
    }

    /// Called when a weapon is equipped.
    /// If the player does not already have the weapon's skill, learns it at value 1.
    /// If the player already has it, no change.
    pub fn on_equip_weapon(
        &self,
        player_guid: ObjectGuid,
        skill_id: u16,
        world: &World,
    ) -> Result<()> {
        if skill_id == 0 {
            return Ok(());
        }

        let already_has = world
            .systems
            .player
            .manager()
            .with_player(player_guid, |player| {
                player
                    .skills
                    .skills
                    .get(&skill_id)
                    .map(|s| s.state != SkillSaveState::Deleted)
                    .unwrap_or(false)
            });

        if !already_has.unwrap_or(false) {
            let level = world
                .systems
                .player
                .manager()
                .with_player(player_guid, |player| player.level);
            if let Some(lvl) = level {
                let (current, max) = get_initial_skill_value(skill_id, lvl);
                self.learn_skill(player_guid, skill_id, current, max, 0, world)?;
            }
        }

        Ok(())
    }

    /// Get a skill value for a player (for hit table calculations).
    pub fn get_skill_value(
        &self,
        player_guid: ObjectGuid,
        skill_id: u16,
        world: &World,
    ) -> Option<u16> {
        world
            .systems
            .player
            .manager()
            .with_player(player_guid, |player| {
                player
                    .skills
                    .skills
                    .get(&skill_id)
                    .filter(|s| s.state != SkillSaveState::Deleted)
                    .map(|s| s.current_value)
            })
            .flatten()
    }

    /// Get defense skill value for hit table calculations.
    pub fn get_defense_skill(&self, player_guid: ObjectGuid, world: &World) -> u16 {
        self.get_skill_value(player_guid, SKILL_DEFENSE, world)
            .unwrap_or(0)
    }
}
