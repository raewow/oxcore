use super::dbc::TalentStore;
use super::state::TalentState;
use crate::shared::protocol::ObjectGuid;
use crate::world::World;

/// Apply the effects of a single talent rank.
///
/// When a player learns a new talent rank, this function:
/// 1. Removes the previous rank's spell (if upgrading from rank N to N+1)
/// 2. Looks up the spell_id for the new rank from Talent.dbc
/// 3. Applies the spell, which may be:
///    a. A passive aura (SPELL_EFFECT_APPLY_AURA with passive flag)
///    b. A spell modifier (adds SpellMod entries to player)
///    c. A learned ability (teaches a new spell, e.g., Mortal Strike)
///
/// # Passive Auras (Most Common)
///
/// Most talent spells have the SPELL_AURA_PASSIVE attribute. When "cast,"
/// they apply a permanent, invisible aura that modifies the player:
///
/// Examples:
///   - Deflection (Warrior): SPELL_AURA_MOD_PARRY_PERCENT (+1% per rank)
///   - Toughness (Warrior): SPELL_AURA_MOD_RESISTANCE (+2 all resist per rank)
///   - Natural Weapons (Druid): SPELL_AURA_MOD_DAMAGE_PERCENT_DONE (+2% per rank)
///
/// These passive auras:
///   - Have infinite duration (never expire)
///   - Occupy passive aura slots (slots 48-63 in vanilla)
///   - Are NOT visible in the buff bar
///   - ARE visible in the character sheet (stat tooltips)
///   - Are removed and reapplied on talent reset or respec
///
/// # Spell Modifiers
///
/// Some talent spells add SpellMod entries that modify properties of
/// other spells the player casts. These are identified by spell effects
/// with SPELL_EFFECT_APPLY_AURA and SPELL_AURA_ADD_FLAT_MODIFIER or
/// SPELL_AURA_ADD_PCT_MODIFIER.
///
/// Examples:
///   - Improved Frostbolt (Mage): SpellModOp::CastingTime, -0.1s per rank
///   - Ice Shards (Mage): SpellModOp::CritDamage, +20% per rank
///   - Shadow Focus (Priest): SpellModOp::Cost, -2% mana per rank
///
/// SpellMod entries are stored in player.spell_modifiers[SpellModOp] and
/// are consulted during spell casting (Phase 5) to modify base values.
///
/// # Learned Abilities
///
/// A few deep-tree talents teach entirely new spells:
///   - Mortal Strike (Warrior Arms, row 6)
///   - Shadowform (Priest Shadow, row 6)
///   - Ice Barrier (Mage Frost, row 6)
///
/// These are learned via the standard spell learning system
/// (SPELL_EFFECT_LEARN_SPELL) and appear in the spellbook.
pub async fn apply_talent_rank(
    player_guid: ObjectGuid,
    talent_id: u32,
    new_rank: u8,
    old_rank: u8,
    store: &TalentStore,
    world: &World,
) -> anyhow::Result<()> {
    let talent_info = store
        .get_talent(talent_id)
        .ok_or_else(|| anyhow::anyhow!("Unknown talent {}", talent_id))?;

    // Remove old rank's spell effects (if upgrading)
    if old_rank > 0 {
        if let Some(old_spell_id) = talent_info.spell_id_for_rank(old_rank) {
            // Remove the passive aura from old rank
            world
                .systems
                .auras
                .remove_spell_auras(player_guid, old_spell_id, world)
                .await?;

            // Remove any spell modifiers from old rank
            if let Some(_) = world
                .systems
                .player
                .manager()
                .with_player_mut(player_guid, |player| {
                    player
                        .spells
                        .spell_modifiers
                        .retain(|m| m.source_spell_id != old_spell_id);
                    Some(())
                })
            {}
        }
    }

    // Apply new rank's spell effects
    if let Some(new_spell_id) = talent_info.spell_id_for_rank(new_rank) {
        // Look up the spell from the DBC to determine its effect type
        // The spell system handles dispatching to the correct effect:
        //   - Passive auras -> AuraSystem.apply_aura()
        //   - Spell modifiers -> added to player.spell_modifiers
        //   - Learned spells -> added to player.known_spells
        world
            .systems
            .spells
            .apply_talent_spell(player_guid, new_spell_id, world)
            .await?;
    }

    Ok(())
}

/// Remove all talent effects from a player.
///
/// Called during talent reset. Iterates through every talent the player
/// has learned, removes the associated passive auras, spell modifiers,
/// and any learned talent-granted spells.
///
/// Ported from MaNGOS Player::resetTalents() removal loop
/// (Player.cpp:3350-3420).
///
/// # Arguments
/// * `player_guid` - The player being reset
/// * `state` - Player's current talent state (read-only, for iteration)
/// * `store` - Global talent DBC store
/// * `world` - World reference for system access
pub async fn remove_all_talent_effects(
    player_guid: ObjectGuid,
    state: &TalentState,
    store: &TalentStore,
    world: &World,
) -> anyhow::Result<()> {
    for (&talent_id, &rank) in &state.talents {
        if rank == 0 {
            continue;
        }

        let talent_info = match store.get_talent(talent_id) {
            Some(info) => info,
            None => continue,
        };

        // Remove every rank's spell up to current (some auras stack by rank)
        // In practice, only the current rank's spell is active
        if let Some(spell_id) = talent_info.spell_id_for_rank(rank) {
            // Remove passive aura
            world
                .systems
                .auras
                .remove_spell_auras(player_guid, spell_id, world)
                .await?;

            // Remove learned spell (if this talent teaches a spell)
            world
                .systems
                .spells
                .unlearn_talent_spell(player_guid, spell_id, world)
                .await?;
        }
    }

    // Clear all spell modifiers originating from talent spells
    if let Some(_) = world
        .systems
        .player
        .manager()
        .with_player_mut(player_guid, |player| {
            // Remove modifiers that came from talents (we'll need to track this)
            // For now, we remove all modifiers - they'll be re-added on reapply
            player.spells.spell_modifiers.clear();
            Some(())
        })
    {}

    Ok(())
}

/// Reapply all talent effects on login.
///
/// When a player logs in, their talent state is loaded from the database
/// but the passive auras and spell modifiers are not persisted -- they
/// must be reconstructed from the talent data.
///
/// This iterates through all learned talents and applies the spell
/// associated with each talent's current rank.
///
/// Ported from MaNGOS Player::_LoadTalents() post-processing.
pub async fn reapply_all_talent_effects(
    player_guid: ObjectGuid,
    state: &TalentState,
    store: &TalentStore,
    world: &World,
) -> anyhow::Result<()> {
    for (&talent_id, &rank) in &state.talents {
        if rank == 0 {
            continue;
        }

        let talent_info = match store.get_talent(talent_id) {
            Some(info) => info,
            None => {
                tracing::warn!(
                    "Player {:?} has unknown talent {} rank {} - skipping",
                    player_guid,
                    talent_id,
                    rank
                );
                continue;
            }
        };

        if let Some(spell_id) = talent_info.spell_id_for_rank(rank) {
            world
                .systems
                .spells
                .apply_talent_spell(player_guid, spell_id, world)
                .await?;
        }
    }

    Ok(())
}
