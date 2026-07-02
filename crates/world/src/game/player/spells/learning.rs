//! Spell Learning
//!
//! Handles spell learning, unlearning, auto-learning on level up, and spellbook management.

use crate::game::broadcast_mgr::{BroadcastManagerExt, BroadcastManagerTrait};
use crate::World;
use anyhow::Result;
use oxcore_shared::messages::spells::{
    InitialSpellCooldown, SmsgInitialSpells, SmsgLearnedSpell, SmsgRemovedSpell,
};
use oxcore_shared::messages::ToWorldPacket;
use oxcore_shared::protocol::ObjectGuid;
use std::sync::Arc;

/// SPELL_EFFECT_SKILL numeric id (effect slot index 1, i.e. EFFECT_INDEX_1).
const SPELL_EFFECT_SKILL: u32 = 118;
const EFFECT_INDEX_1: usize = 1;

/// Whether casting `spell_id` teaches a profession or riding skill.
///
/// Matches C++ `SpellMgr::IsProfessionOrRidingSpell`.
pub fn is_profession_or_riding_spell(spell_id: u32, world: &World) -> bool {
    let Some(entry) = world.managers.spell_mgr.get(spell_id) else {
        return false;
    };
    if entry.effect[EFFECT_INDEX_1] != SPELL_EFFECT_SKILL {
        return false;
    }
    let skill = entry.effect_misc_value[EFFECT_INDEX_1] as u32;
    let category_id = world.dbc.read().get_skill_line(skill).map_or(0, |s| s.category_id);
    crate::game::player::skills::formulas::is_profession_or_riding_skill(skill as u16, category_id)
}

/// Whether casting `spell_id` teaches a profession skill (primary or secondary).
///
/// Matches C++ `SpellMgr::IsProfessionSpell`.
pub fn is_profession_spell(spell_id: u32, world: &World) -> bool {
    let Some(entry) = world.managers.spell_mgr.get(spell_id) else {
        return false;
    };
    if entry.effect[EFFECT_INDEX_1] != SPELL_EFFECT_SKILL {
        return false;
    }
    let skill = entry.effect_misc_value[EFFECT_INDEX_1] as u32;
    let category_id = world.dbc.read().get_skill_line(skill).map_or(0, |s| s.category_id);
    crate::game::player::skills::formulas::is_profession_skill(skill as u16, category_id)
}

/// Whether casting `spell_id` teaches a primary profession skill.
///
/// Matches C++ `SpellMgr::IsPrimaryProfessionSpell`.
pub fn is_primary_profession_spell(spell_id: u32, world: &World) -> bool {
    let Some(entry) = world.managers.spell_mgr.get(spell_id) else {
        return false;
    };
    if entry.effect[EFFECT_INDEX_1] != SPELL_EFFECT_SKILL {
        return false;
    }
    let skill = entry.effect_misc_value[EFFECT_INDEX_1] as u32;
    let category_id = world.dbc.read().get_skill_line(skill).map_or(0, |s| s.category_id);
    crate::game::player::skills::formulas::is_primary_profession_skill(category_id)
}

/// Whether `spell_id` is the first (rank 1) spell of a primary profession chain.
///
/// Matches C++ `SpellMgr::IsPrimaryProfessionFirstRankSpell`.
pub fn is_primary_profession_first_rank_spell(spell_id: u32, world: &World) -> bool {
    is_primary_profession_spell(spell_id, world) && world.managers.spell_mgr.get_spell_rank(spell_id) == 1
}

/// Get current game time in milliseconds
fn get_game_time_ms(world: &World) -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Learn a new spell for a player.
///
/// Called from:
/// - Trainer NPC interaction
/// - Quest rewards
/// - Level-up auto-learning
/// - GM commands
///
/// Returns true if the spell was newly learned, false if already known.
pub async fn learn_spell(
    player_guid: ObjectGuid,
    spell_id: u32,
    world: &World,
    broadcast_mgr: &Arc<dyn BroadcastManagerTrait>,
) -> Result<bool> {
    let mut newly_learned = false;

    world
        .systems
        .player
        .manager()
        .with_player_mut(player_guid, |player| {
            if player.spells.learn_spell(spell_id) {
                newly_learned = true;
                tracing::info!("Player {} learned spell {}", player.name, spell_id);
            }
        });

    if newly_learned {
        // Send SMSG_LEARNED_SPELL to client
        let msg = SmsgLearnedSpell { spell_id };
        broadcast_mgr.send_msg_to_player(player_guid, msg.to_world_packet());

        // TODO: Check if this spell teaches a dependent spell (e.g., learning a rank teaches the rank)
        // Check SkillLineAbility.dbc for forward references
    }

    Ok(newly_learned)
}

/// Unlearn a spell.
///
/// Called from:
/// - Talent reset
/// - Unlearn profession
/// - GM commands
pub async fn unlearn_spell(
    player_guid: ObjectGuid,
    spell_id: u32,
    world: &World,
    broadcast_mgr: &Arc<dyn BroadcastManagerTrait>,
) -> Result<()> {
    let was_removed = world
        .systems
        .player
        .manager()
        .with_player_mut(player_guid, |player| player.spells.unlearn_spell(spell_id))
        .unwrap_or(false);

    if was_removed {
        // Send SMSG_REMOVED_SPELL to client
        let msg = SmsgRemovedSpell { spell_id };
        broadcast_mgr.send_msg_to_player(player_guid, msg.to_world_packet());

        // Remove any auras from this spell
        world
            .systems
            .auras
            .remove_spell_auras(player_guid, spell_id, world)
            .await?;
    }

    Ok(())
}

/// Send the full spellbook to the client on login.
///
/// Sends SMSG_INITIAL_SPELLS which contains:
/// - All known spell IDs
/// - All active cooldowns
pub fn send_initial_spells(
    player_guid: ObjectGuid,
    world: &World,
    broadcast_mgr: &Arc<dyn BroadcastManagerTrait>,
) -> Result<()> {
    let now = get_game_time_ms(world);
    let mut spellbook: Vec<u32> = Vec::new();
    let mut cooldowns: Vec<InitialSpellCooldown> = Vec::new();

    world
        .systems
        .player
        .manager()
        .with_player_mut(player_guid, |player| {
            spellbook = player.spells.spellbook.clone();

            // Collect active cooldowns for the initial spells packet
            for (&spell_id, &cd_end) in &player.spells.cooldowns {
                if cd_end > now {
                    cooldowns.push(InitialSpellCooldown {
                        spell_id,
                        item_id: 0,  // TODO: Look up from DBC if spell is from item
                        category: 0, // TODO: Look up from DBC
                        spell_cooldown_ms: (cd_end - now) as u32,
                        category_cooldown_ms: 0, // TODO: Look up from DBC
                    });
                }
            }
        });

    let msg = SmsgInitialSpells {
        cast_count: 0,
        spells: spellbook,
        cooldowns,
    };
    broadcast_mgr.send_msg_to_player(player_guid, msg.to_world_packet());

    Ok(())
}

/// Auto-learn spells for a level up.
///
/// Uses SkillLineAbility.dbc to determine which spells are auto-learned.
/// Each row maps: skill -> spell -> learn_at_level
///
/// For class spells, the skill is the class skill (e.g., "Arms" for Warriors).
/// For racial spells, the skill is the racial skill.
pub async fn auto_learn_for_level(
    player_guid: ObjectGuid,
    _new_level: u8,
    _world: &World,
    _broadcast_mgr: &Arc<dyn BroadcastManagerTrait>,
) -> Result<()> {
    // TODO: Query SkillLineAbility.dbc for spells that auto-learn at this level
    // for the player's class and race.
    //
    // Pseudocode:
    // for entry in dbc.skill_line_ability.iter() {
    //     if entry.auto_learn_type == 1  // CLASS_SKILL_LEARN
    //         && entry.min_skill_value <= new_level as u32 * 5
    //         && player_has_skill(entry.skill_id)
    //     {
    //         learn_spell(player_guid, entry.spell_id, world, broadcast_mgr).await?;
    //     }
    // }

    let _ = player_guid;
    Ok(())
}

/// Load learned spells from database on login.
#[allow(dead_code)]
pub fn load_from_db(_player_guid: ObjectGuid, _world: &World) -> Result<()> {
    // TODO: Query character_spells table
    // SELECT spell_id FROM character_spells WHERE guid = ?
    //
    // world.systems.player.manager().with_player_mut(player_guid, |player| {
    //     for row in results {
    //         player.spells.learned_spells.insert(row.spell_id);
    //         player.spells.spellbook.push(row.spell_id);
    //     }
    // });

    Ok(())
}

/// Save learned spells to database on logout.
#[allow(dead_code)]
pub fn save_to_db(_player_guid: ObjectGuid, _world: &World) -> Result<()> {
    // TODO: Implement database persistence
    // Only save if needs_save is true
    // DELETE FROM character_spells WHERE guid = ?
    // INSERT INTO character_spells (guid, spell_id) VALUES (?, ?) for each spell
    Ok(())
}
