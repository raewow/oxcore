//! Teleportation Spell Effects
//!
//! Handles all teleportation, binding, and transportation effects.

use super::{EffectInput, EffectResult};
use crate::game::player::teleport;
use crate::World;
use anyhow::Result;
use oxcore_shared::protocol::{ObjectGuid, Position};

/// Caster-home-bind target id (9) — Hearthstone / recall.
const TARGET_LOCATION_CASTER_HOME_BIND: u32 = 9;
/// Database-location target id (17) — coordinates in the spell-target-position table.
const TARGET_LOCATION_DATABASE: u32 = 17;
/// Script-AoE-at-source target id (7) — also uses the spell-target-position
/// table in this context.
const TARGET_ENUM_UNITS_SCRIPT_AOE_AT_SRC_LOC: u32 = 7;

/// Teleport-units effect (5)
///
/// Teleport the target to a specific location.
/// Used for Hearthstone, portals, and recall spells.
pub async fn effect_teleport_units(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = input.target_guid.unwrap_or(input.caster_guid);

    // Look up spell entry to check implicit target B
    let spell_entry = world.managers.spell_mgr.get(input.spell_id);
    let implicit_target_b = spell_entry
        .map(|e| e.effect_implicit_target_b[input.effect_index as usize])
        .unwrap_or(0);

    // Determine destination
    let (dest_map, dest_pos) = if implicit_target_b == TARGET_LOCATION_CASTER_HOME_BIND {
        // Hearthstone / recall: teleport to homebind
        let homebind = world
            .systems
            .player
            .manager()
            .with_player(target_guid, |p| {
                (p.homebind_map, p.homebind_x, p.homebind_y, p.homebind_z)
            });

        match homebind {
            Some((map, x, y, z)) => {
                tracing::info!(
                    "Teleport to homebind: target={:?} map={} pos=({:.1}, {:.1}, {:.1})",
                    target_guid,
                    map,
                    x,
                    y,
                    z
                );
                (map, Position::new(x, y, z, 0.0))
            }
            None => {
                tracing::warn!("Player {:?} not found for teleport", target_guid);
                return Ok(EffectResult::empty());
            }
        }
    } else if implicit_target_b == TARGET_LOCATION_DATABASE
        || implicit_target_b == TARGET_ENUM_UNITS_SCRIPT_AOE_AT_SRC_LOC
    {
        // Portal / teleport spell: coordinates from spell_target_position table
        match world
            .managers
            .spell_mgr
            .get_spell_target_position(input.spell_id)
        {
            Some(pos) => {
                tracing::info!(
                    "Teleport to DB position: spell={} target={:?} map={} pos=({:.1}, {:.1}, {:.1})",
                    input.spell_id, target_guid, pos.map_id, pos.x, pos.y, pos.z
                );
                (
                    pos.map_id,
                    Position::new(pos.x, pos.y, pos.z, pos.orientation),
                )
            }
            None => {
                tracing::warn!(
                    "No spell_target_position entry for spell {} (TARGET_LOCATION_DATABASE)",
                    input.spell_id
                );
                return Ok(EffectResult::empty());
            }
        }
    } else {
        tracing::debug!(
            "Teleport units: target={:?} spell={} implicit_target_b={} (unhandled)",
            target_guid,
            input.spell_id,
            implicit_target_b
        );
        return Ok(EffectResult::empty());
    };

    // Teleporting yourself is a spell teleport: it must not interrupt its own cast.
    let mut options = if target_guid == input.caster_guid {
        teleport::options::SPELL
    } else {
        0
    };
    if implicit_target_b == TARGET_LOCATION_CASTER_HOME_BIND {
        // The hearthstone always shows the loading screen, even within a map.
        options |= teleport::options::FORCE_MAP_CHANGE;
    } else {
        // A spell's same-map teleport keeps the fight and the pet.
        options |= teleport::options::NOT_LEAVE_COMBAT | teleport::options::NOT_UNSUMMON_PET;
    }

    if teleport::teleport_to(world, target_guid, dest_map, dest_pos, options).await? {
        tracing::info!(
            "Teleport initiated: target={:?} to map={} pos=({:.1}, {:.1}, {:.1})",
            target_guid,
            dest_map,
            dest_pos.x,
            dest_pos.y,
            dest_pos.z
        );
    }

    Ok(EffectResult::empty())
}

/// Bind effect (11)
///
/// Makes the target's current spot their home (the innkeeper's bind spell).
pub async fn effect_bind(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = input.target_guid.unwrap_or(input.caster_guid);
    if !target_guid.is_player() {
        return Ok(EffectResult::empty());
    }
    teleport::bind_home(world, target_guid, input.caster_guid).await?;
    Ok(EffectResult::empty())
}

/// Teleport-units-facing-caster effect (43)
///
/// Pull the target to the spell's radius in front of the caster; the fight
/// and the pet carry on. Only players are moved: creatures do not teleport.
pub async fn effect_teleport_units_face_caster(
    input: &EffectInput,
    world: &World,
) -> Result<EffectResult> {
    let Some(target_guid) = input.target_guid.filter(ObjectGuid::is_player) else {
        return Ok(EffectResult::empty());
    };
    let caster = world
        .managers
        .player_mgr
        .with_player(input.caster_guid, |player| (player.map_id, player.movement.position))
        .or_else(|| {
            world
                .managers
                .creature_mgr
                .with_creature(input.caster_guid, |creature| {
                    (creature.map_id, creature.position)
                })
        });
    let Some((map_id, caster_pos)) = caster else {
        return Ok(EffectResult::empty());
    };
    let distance = world
        .managers
        .spell_mgr
        .get(input.spell_id)
        .map(|entry| entry.effect_radius_index[input.effect_index as usize])
        .filter(|&index| index > 0)
        .and_then(|index| world.dbc.read().get_spell_radius(index).map(|r| r.radius))
        .unwrap_or(0.0);
    let dest = Position::new(
        caster_pos.x + distance * caster_pos.o.cos(),
        caster_pos.y + distance * caster_pos.o.sin(),
        caster_pos.z,
        -caster_pos.o,
    );

    let mut options = teleport::options::NOT_LEAVE_COMBAT | teleport::options::NOT_UNSUMMON_PET;
    if target_guid == input.caster_guid {
        options |= teleport::options::SPELL;
    }
    // A near teleport only: a caster on another map cannot pull anyone.
    let on_same_map = world
        .managers
        .player_mgr
        .with_player(target_guid, |player| player.map_id == map_id)
        .unwrap_or(false);
    if on_same_map {
        teleport::teleport_to(world, target_guid, map_id, dest, options).await?;
    }
    Ok(EffectResult::empty())
}

/// Stuck effect (84)
///
/// Return a stuck player to the last spot they stood on of their own accord,
/// on the same map.
pub async fn effect_stuck(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = input.target_guid.unwrap_or(input.caster_guid);
    if !world.config.cast_unstuck {
        return Ok(EffectResult::empty());
    }
    let Some((map_id, safe)) = world
        .managers
        .player_mgr
        .with_player(target_guid, |player| (player.map_id, player.movement.safe_position))
    else {
        return Ok(EffectResult::empty());
    };
    let Some(safe) = safe.filter(|safe| safe.x.abs() > 0.1 && safe.y.abs() > 0.1) else {
        return Ok(EffectResult::empty());
    };
    tracing::info!(
        "Player {:?} used unstuck on map {}, back to ({:.1}, {:.1}, {:.1})",
        target_guid,
        map_id,
        safe.x,
        safe.y,
        safe.z
    );
    // The saved spot is lifted for falls; land just above the ground instead.
    let dest = Position::new(safe.x, safe.y, safe.z - 2.0 + 0.7, safe.o);
    teleport::teleport_to(world, target_guid, map_id, dest, 0).await?;
    Ok(EffectResult::empty())
}

/// Summon-player effect (85)
///
/// Offer the target a summon to just in front of the caster (warlock
/// ritual, meeting stone). The target accepts with CMSG_SUMMON_RESPONSE.
pub async fn effect_summon_player(input: &EffectInput, world: &World) -> Result<EffectResult> {
    /// Evil Twin: the target ignores summons.
    const EVIL_TWIN: u32 = 23445;
    /// How long an offer stands.
    const SUMMON_DELAY: std::time::Duration = std::time::Duration::from_secs(120);
    /// Distance in front of the caster the summoned player lands.
    const LANDING_DISTANCE: f32 = 1.0;

    let Some(target_guid) = input.target_guid.filter(ObjectGuid::is_player) else {
        return Ok(EffectResult::empty());
    };
    let ignores_summons = world
        .managers
        .player_mgr
        .with_player(target_guid, |player| {
            player.auras.container.all_auras().any(|aura| aura.spell_id == EVIL_TWIN)
        })
        .unwrap_or(true);
    if ignores_summons {
        return Ok(EffectResult::empty());
    }

    let caster = world
        .managers
        .player_mgr
        .with_player(input.caster_guid, |player| {
            (player.map_id, player.zone_id, player.movement.position)
        })
        .or_else(|| {
            world
                .managers
                .creature_mgr
                .with_creature(input.caster_guid, |creature| {
                    (creature.map_id, 0, creature.position)
                })
        });
    let Some((map_id, zone_id, caster_pos)) = caster else {
        return Ok(EffectResult::empty());
    };
    let landing = Position::new(
        caster_pos.x + LANDING_DISTANCE * caster_pos.o.cos(),
        caster_pos.y + LANDING_DISTANCE * caster_pos.o.sin(),
        caster_pos.z,
        caster_pos.o,
    );

    world
        .managers
        .player_mgr
        .with_player_mut(target_guid, |player| {
            player.summon_offer = Some(crate::game::player::player::SummonOffer {
                map_id,
                position: landing,
                expires_at: std::time::Instant::now() + SUMMON_DELAY,
            });
        });
    world.managers.broadcast_mgr.send_msg_to_player(
        target_guid,
        oxcore_shared::messages::movement::SmsgSummonRequest {
            summoner: input.caster_guid,
            zone_id,
            delay_ms: SUMMON_DELAY.as_millis() as u32,
        },
    );
    Ok(EffectResult::empty())
}

/// Send-taxi effect (123)
///
/// Fly the player along the scripted taxi path named by the misc value.
pub async fn effect_send_taxi(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = input.target_guid.unwrap_or(input.caster_guid);
    if !target_guid.is_player() {
        return Ok(EffectResult::empty());
    }
    let Ok(path) = u32::try_from(input.misc_value) else {
        return Ok(EffectResult::empty());
    };
    crate::game::taxi::flight::activate_path(world, target_guid, path, input.spell_id).await?;
    Ok(EffectResult::empty())
}

/// Player-pull effect (124)
///
/// Pull the player toward the caster.
/// Opposite of knockback.
pub async fn effect_player_pull(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    let pull_distance = input.base_value.max(0) as f32;

    // TODO: Pull target toward caster

    tracing::debug!(
        "Player pull: target={:?} distance={:.1}",
        target_guid,
        pull_distance
    );

    Ok(EffectResult::empty())
}
