//! Player aura persistence - load/save `character_aura` rows.
//!
//! Player aura persistence - load/save `character_aura` rows.
//!
//! The `SpellAuraHolder` model groups effects under one holder per (caster, item, spell) and
//! saves one DB row per holder with up to 3 effect slots packed in (`base_pointsN` /
//! `periodic_timeN` / `effect_index_mask`). The Rust `AuraContainer` stores one [`Aura`] per
//! `(spell_id, effect_index)`. To stay compatible with the `character_aura` table, saving
//! groups the container's per-effect auras back into holders by
//! `(spell_id, caster_guid, cast_item_guid)` before writing a row, and loading reconstructs the
//! per-effect auras from a row's `effect_index_mask`.

use super::aura::AuraFlags;
use crate::game::player::spells::diminishing::DiminishSnapshot;
use crate::World;
use anyhow::Result;
use oxcore_db::database::characters::models::character::CharacterAuraRow;
use oxcore_db::database::characters::{PgCharacterAuraRow, PgCharacterRepository};
use oxcore_shared::protocol::ObjectGuid;
use std::collections::HashMap;
use std::sync::Arc;

/// SPELL_ATTR_EX4_AURA_EXPIRES_OFFLINE (0x00000004 in attributes_ex4) — aura's remaining
/// duration is debited by offline time on next login.
const SPELL_ATTR_EX4_AURA_EXPIRES_OFFLINE: u32 = 0x0000_0004;

/// SPELL_ATTR_EX_NEGATIVE-ish polarity check is already exposed via `is_positive_spell`/
/// `is_positive_effect` on `SpellEntry`; no local reimplementation needed.
const IN_MILLISECONDS: i64 = 1000;

/// Aura types that must never be persisted to `character_aura`.
const SPELL_AURA_BIND_SIGHT: u32 = 1;
const SPELL_AURA_MOD_POSSESS: u32 = 2;
const SPELL_AURA_MOD_CHARM: u32 = 6;
const SPELL_AURA_FAR_SIGHT: u32 = 76;
const SPELL_AURA_AOE_CHARM: u32 = 177;

/// AURA_INTERRUPT_LEAVE_WORLD_CANCELS (bit 19) | AURA_INTERRUPT_ENTER_WORLD_CANCELS (bit 22).
const AURA_INTERRUPT_LEAVE_OR_ENTER_WORLD_CANCELS: u32 =
    crate::game::player::auras::interrupt::AuraInterruptFlags::LEAVE_WORLD_CANCELS.0
        | crate::game::player::auras::interrupt::AuraInterruptFlags::ENTER_WORLD_CANCELS.0;

/// CLASS_WARRIOR
const CLASS_WARRIOR: u8 = 1;
/// SPELL_ID_PASSIVE_BATTLE_STANCE
const SPELL_ID_PASSIVE_BATTLE_STANCE: u32 = 2457;
/// SPELL_AURA_MOD_SHAPESHIFT
const SPELL_AURA_MOD_SHAPESHIFT: u32 = 36;

/// Load all saved auras for a player from `character_aura` and re-apply them.
///
/// `timediff` is the number of seconds the character was offline (used to debit remaining
/// duration for auras with `HasRealTimeDuration()`).
pub async fn load_auras(player_guid: ObjectGuid, timediff: u32, world: &World) -> Result<()> {
    let char_repo = PgCharacterRepository::new(Arc::new(world.databases.character.clone()));
    let rows = char_repo
        .find_auras(i64::from(player_guid.counter()))
        .await?;

    for row in rows {
        let Some(row) = aura_row_from_pg(row) else {
            tracing::warn!(
                "Ignoring invalid signed PostgreSQL aura row for {}",
                player_guid
            );
            continue;
        };
        load_aura(player_guid, &row, timediff, world).await;
    }

    // Warriors without a shapeshift aura get their passive battle stance cast.
    let (class, has_shapeshift) = world
        .systems
        .player
        .manager()
        .with_player(player_guid, |player| {
            (
                player.class,
                player
                    .auras
                    .container
                    .has_aura_type(SPELL_AURA_MOD_SHAPESHIFT),
            )
        })
        .unwrap_or((0, true));

    if class == CLASS_WARRIOR && !has_shapeshift {
        let _ = world
            .systems
            .spells
            .cast_spell(
                player_guid,
                SPELL_ID_PASSIVE_BATTLE_STANCE,
                Some(player_guid),
                true,
                world,
            )
            .await;
    }

    // NOTE: group aura-status sync is out of scope for aura persistence and belongs to the group
    // system's own login hook.

    Ok(())
}

/// Reconstruct and apply one saved aura holder row.
///
/// Skipped when:
/// - the spell id is unknown,
/// - the saved duration already ran out after debiting offline `timediff`.
async fn load_aura(player_guid: ObjectGuid, row: &CharacterAuraRow, timediff: u32, world: &World) {
    let Some(spell_entry) = world.managers.spell_mgr.get(row.spell) else {
        tracing::error!("Unknown spell (spellid {}), ignoring.", row.spell);
        return;
    };

    let mut duration = row.duration;
    if duration != -1 && spell_entry.attributes_ex4 & SPELL_ATTR_EX4_AURA_EXPIRES_OFFLINE != 0 {
        if timediff as i64 > (i32::MAX as i64 / IN_MILLISECONDS) {
            return;
        }
        if duration as i64 <= timediff as i64 * IN_MILLISECONDS {
            return;
        }
        duration -= timediff as i32 * IN_MILLISECONDS as i32;
    }

    // NOTE: SPELL_PLAYER_MUTED_VISUAL reads the session's live mute time, which has no
    // equivalent yet (no per-session mute-expiry state tracked in Rust). This is a single
    // cosmetic aura (chat-muted icon), not core aura logic.

    // prevent wrong values of remaincharges / stacks
    let charges = if spell_entry.proc_charges == 0 {
        0
    } else {
        row.charges
    };
    let stacks = if spell_entry.stack_amount == 0 {
        1
    } else if spell_entry.stack_amount < row.stacks {
        spell_entry.stack_amount
    } else if row.stacks == 0 {
        1
    } else {
        row.stacks
    };

    let duration_ms: Option<u32> = if duration < 0 {
        None
    } else {
        Some(duration as u32)
    };
    let max_duration_ms: Option<u32> = if row.max_duration < 0 {
        None
    } else {
        Some(row.max_duration as u32)
    };

    let caster_guid = ObjectGuid::from_raw(row.caster_guid);
    let base_points = [row.base_points0, row.base_points1, row.base_points2];
    let periodic_time = [row.periodic_time0, row.periodic_time1, row.periodic_time2];

    let cast_item_guid = if row.item_guid != 0 {
        Some(ObjectGuid::new_item(row.item_guid))
    } else {
        None
    };

    for effect_index in 0..3u8 {
        if (row.effect_index_mask & (1 << effect_index)) == 0 {
            continue;
        }
        let idx = effect_index as usize;
        let aura_type = spell_entry.effect_apply_aura_name[idx];
        if aura_type == 0 {
            continue;
        }

        let base_value = base_points[idx] as i32;
        let periodic_interval_ms = periodic_time[idx];
        let is_positive = spell_entry.is_positive_effect(idx);

        let flags = AuraFlags {
            is_positive,
            is_negative: !is_positive,
            is_passive: false,
            can_be_cancelled: is_positive,
            is_hidden: false,
            is_permanent: duration_ms.is_none(),
        };

        // Go through the normal apply path rather than pushing straight into the
        // container: a restored aura must re-run its effect
        // handlers, or the character comes back without the state the aura owns —
        // a warrior's saved stance would leave `shapeshift_form` at 0, a movement
        // aura would not re-apply its speed, and so on.
        let applied = world
            .systems
            .auras
            .apply_aura(
                player_guid,
                caster_guid,
                cast_item_guid,
                row.spell,
                effect_index,
                aura_type,
                spell_entry.effect_misc_value[idx],
                base_value,
                duration_ms,
                periodic_interval_ms,
                stacks.max(1) as u8,
                spell_entry.proc_charges.max(charges) as u8,
                flags,
                // Saved auras already had DR applied to the duration they were stored with;
                // re-diminishing on load would shorten them a second time.
                DiminishSnapshot::default(),
                world,
            )
            .await;

        if let Err(err) = applied {
            tracing::error!(
                "Failed to apply spell {} effect {}: {err:#}",
                row.spell,
                effect_index
            );
            continue;
        }

        // Restore the persisted counters the apply path
        // recomputed from the spell template.
        world
            .systems
            .player
            .manager()
            .with_player_mut(player_guid, |player| {
                if let Some(aura) = player.auras.container.get_aura_mut(row.spell, effect_index) {
                    aura.duration_index = spell_entry.duration_index;
                    aura.max_duration_ms = max_duration_ms;
                    aura.duration_ms = duration_ms;
                    aura.stack_count = stacks.max(1) as u8;
                    aura.charges = charges as u8;
                    aura.periodic_interval_ms = periodic_interval_ms;
                }
            });
    }
}

/// Save all of a player's currently-active, persistable auras to `character_aura`.
///
/// Deletes all existing rows for the character then re-inserts one row per (spell, caster, item)
/// holder built from the still-active per-effect auras.
pub async fn save_auras(player_guid: ObjectGuid, world: &World) -> Result<()> {
    let holders = collect_holders_for_save(player_guid, world);
    let rows = holders
        .into_iter()
        .filter_map(|holder| save_aura(player_guid, &holder, world))
        .map(aura_row_to_pg)
        .collect::<Vec<_>>();
    PgCharacterRepository::new(Arc::new(world.databases.character.clone()))
        .replace_auras(i64::from(player_guid.counter()), &rows)
        .await?;

    Ok(())
}

fn aura_row_from_pg(row: PgCharacterAuraRow) -> Option<CharacterAuraRow> {
    Some(CharacterAuraRow {
        guid: u32::try_from(row.guid).ok()?,
        caster_guid: u64::try_from(row.caster_guid).ok()?,
        item_guid: u32::try_from(row.item_guid).ok()?,
        spell: u32::try_from(row.spell).ok()?,
        stacks: u32::try_from(row.stacks).ok()?,
        charges: u32::try_from(row.charges).ok()?,
        base_points0: row.base_points0,
        base_points1: row.base_points1,
        base_points2: row.base_points2,
        periodic_time0: u32::try_from(row.periodic_time0).ok()?,
        periodic_time1: u32::try_from(row.periodic_time1).ok()?,
        periodic_time2: u32::try_from(row.periodic_time2).ok()?,
        max_duration: row.max_duration,
        duration: row.duration,
        effect_index_mask: u8::try_from(row.effect_index_mask).ok()?,
    })
}

fn aura_row_to_pg(row: CharacterAuraRow) -> PgCharacterAuraRow {
    PgCharacterAuraRow {
        guid: i64::from(row.guid),
        caster_guid: i64::try_from(row.caster_guid).unwrap_or(i64::MAX),
        item_guid: i64::from(row.item_guid),
        spell: i64::from(row.spell),
        stacks: i64::from(row.stacks),
        charges: i64::from(row.charges),
        base_points0: row.base_points0,
        base_points1: row.base_points1,
        base_points2: row.base_points2,
        periodic_time0: i64::from(row.periodic_time0),
        periodic_time1: i64::from(row.periodic_time1),
        periodic_time2: i64::from(row.periodic_time2),
        max_duration: row.max_duration,
        duration: row.duration,
        effect_index_mask: i16::from(row.effect_index_mask),
    }
}

/// One (spell, caster, item) holder's worth of per-effect aura snapshots.
struct HolderSnapshot {
    spell_id: u32,
    caster_guid: ObjectGuid,
    cast_item_guid: Option<ObjectGuid>,
    stacks: u8,
    charges: u8,
    max_duration_ms: Option<u32>,
    duration_ms: Option<u32>,
    is_passive: bool,
    /// effect_index -> (base_value, periodic_interval_ms)
    effects: [Option<(f32, u32)>; 3],
}

/// Snapshot the container's live auras, grouped by holder key, while holding the player lock
/// only briefly (no DB/async work happens inside the lock).
fn collect_holders_for_save(player_guid: ObjectGuid, world: &World) -> Vec<HolderSnapshot> {
    world
        .systems
        .player
        .manager()
        .with_player(player_guid, |player| {
            let mut by_holder: HashMap<(u32, ObjectGuid, Option<ObjectGuid>), HolderSnapshot> =
                HashMap::new();

            for aura in player.auras.container.all_auras() {
                let key = (aura.spell_id, aura.caster_guid, aura.cast_item_guid);
                let entry = by_holder.entry(key).or_insert_with(|| HolderSnapshot {
                    spell_id: aura.spell_id,
                    caster_guid: aura.caster_guid,
                    cast_item_guid: aura.cast_item_guid,
                    stacks: aura.stack_count,
                    charges: aura.charges,
                    max_duration_ms: aura.max_duration_ms,
                    duration_ms: aura.duration_ms,
                    is_passive: aura.is_passive(),
                    effects: [None, None, None],
                });
                let idx = aura.effect_index as usize;
                if idx < 3 {
                    entry.effects[idx] =
                        Some((aura.current_value() as f32, aura.periodic_interval_ms));
                }
            }

            by_holder.into_values().collect()
        })
        .unwrap_or_default()
}

/// Build a `character_aura` row for one holder, or `None` if it must not be saved.
///
/// Skip conditions:
/// - holder's spell has `AURA_INTERRUPT_LEAVE_WORLD_CANCELS`/`ENTER_WORLD_CANCELS`,
/// - holder is passive or the spell is channeled,
/// - none of its effects apply a persistable aura type
///   (`BIND_SIGHT`/`MOD_POSSESS`/`MOD_CHARM`/`FAR_SIGHT`/`AOE_CHARM` => don't save at all),
/// - the holder ends up with an empty effect_index_mask.
fn save_aura(
    player_guid: ObjectGuid,
    holder: &HolderSnapshot,
    world: &World,
) -> Option<CharacterAuraRow> {
    let spell_entry = world.managers.spell_mgr.get(holder.spell_id)?;

    if spell_entry.aura_interrupt_flags & AURA_INTERRUPT_LEAVE_OR_ENTER_WORLD_CANCELS != 0 {
        return None;
    }

    let is_channeled =
        (spell_entry.attributes_ex & 0x04) != 0 || (spell_entry.attributes_ex & 0x40) != 0;
    if holder.is_passive || is_channeled {
        return None;
    }

    // Single-target auras owned by someone else aren't saved for this character.
    // We don't track single-target holder state yet, so this only guards on caster identity,
    // matching the common case.
    let owned_by_self = holder.caster_guid == player_guid;

    let mut base_points = [0.0f32; 3];
    let mut periodic_time = [0u32; 3];
    let mut effect_index_mask = 0u8;

    for i in 0..3usize {
        match spell_entry.effect_apply_aura_name[i] {
            v if v == SPELL_AURA_BIND_SIGHT
                || v == SPELL_AURA_MOD_POSSESS
                || v == SPELL_AURA_MOD_CHARM
                || v == SPELL_AURA_FAR_SIGHT
                || v == SPELL_AURA_AOE_CHARM =>
            {
                return None;
            }
            _ => {}
        }

        if let Some((base_value, periodic_interval_ms)) = holder.effects[i] {
            base_points[i] = base_value;
            periodic_time[i] = periodic_interval_ms;
            effect_index_mask |= 1 << i;
        }
    }

    if effect_index_mask == 0 {
        return None;
    }
    if !owned_by_self {
        // Not our own single-target aura; skip saving (without full single-target-holder tracking).
        return None;
    }

    Some(CharacterAuraRow {
        guid: player_guid.counter(),
        caster_guid: holder.caster_guid.raw(),
        item_guid: holder.cast_item_guid.map(|g| g.counter()).unwrap_or(0),
        spell: holder.spell_id,
        stacks: holder.stacks.max(1) as u32,
        charges: holder.charges as u32,
        base_points0: base_points[0],
        base_points1: base_points[1],
        base_points2: base_points[2],
        periodic_time0: periodic_time[0],
        periodic_time1: periodic_time[1],
        periodic_time2: periodic_time[2],
        max_duration: holder.max_duration_ms.map(|d| d as i32).unwrap_or(-1),
        duration: holder.duration_ms.map(|d| d as i32).unwrap_or(-1),
        effect_index_mask,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupt_flag_bits_leave_and_enter_world() {
        // AURA_INTERRUPT_LEAVE_WORLD_CANCELS = 0x00080000 (bit 19)
        // AURA_INTERRUPT_ENTER_WORLD_CANCELS = 0x00400000 (bit 22)
        assert_eq!(AURA_INTERRUPT_LEAVE_OR_ENTER_WORLD_CANCELS, 0x0048_0000);
    }

    #[test]
    fn banned_aura_types_are_correct() {
        assert_eq!(SPELL_AURA_BIND_SIGHT, 1);
        assert_eq!(SPELL_AURA_MOD_POSSESS, 2);
        assert_eq!(SPELL_AURA_MOD_CHARM, 6);
        assert_eq!(SPELL_AURA_FAR_SIGHT, 76);
        assert_eq!(SPELL_AURA_AOE_CHARM, 177);
    }

    #[test]
    fn battle_stance_constants() {
        assert_eq!(CLASS_WARRIOR, 1);
        assert_eq!(SPELL_ID_PASSIVE_BATTLE_STANCE, 2457);
        assert_eq!(SPELL_AURA_MOD_SHAPESHIFT, 36);
    }

    #[test]
    fn real_time_duration_attribute_bit() {
        // SPELL_ATTR_EX4_AURA_EXPIRES_OFFLINE = 0x00000004 (bit 2 of attributes_ex4)
        assert_eq!(SPELL_ATTR_EX4_AURA_EXPIRES_OFFLINE, 0x0000_0004);
    }
}
