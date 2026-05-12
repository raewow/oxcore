//! Resurrection Spell Effects
//!
//! Handles resurrection-related spell effects.

use super::{EffectInput, EffectResult};
use crate::shared::protocol::ObjectGuid;
use crate::world::World;
use anyhow::Result;

/// SPELL_EFFECT_RESURRECT (18)
///
/// Resurrect a dead player. In vanilla WoW this does NOT directly revive the
/// target — instead the server sends SMSG_RESURRECT_REQUEST with the caster's
/// name, and the dead player must accept via CMSG_RESURRECT_RESPONSE. The
/// actual revive happens in `DeathSystem::handle_resurrect_response`.
pub async fn effect_resurrect(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = match input.target_guid {
        Some(guid) => guid,
        None => return Ok(EffectResult::empty()),
    };

    // Only offer resurrection to dead players.
    let is_dead = world.systems.player.manager().with_player(target_guid, |player| {
        player.stats.health == 0
            || matches!(
                player.death.death_state,
                crate::world::game::player::death::state::DeathState::Corpse
                    | crate::world::game::player::death::state::DeathState::Dead
                    | crate::world::game::player::death::state::DeathState::JustDied
            )
    }).unwrap_or(false);
    if !is_dead {
        return Ok(EffectResult::empty());
    }

    // Pull caster name, target's max health/mana to compute the snapshot we
    // want to restore if the offer is accepted. base_value is the spell's
    // health-percentage coefficient.
    let health_pct = input.base_value.max(1).min(100) as u32;
    let caster_guid = input.caster_guid;

    let snapshot = world.systems.player.manager().with_player(caster_guid, |player| {
        (player.name.clone(), player.map_id, player.instance_id, player.movement.position)
    });
    let (caster_name, map_id, instance_id, location) = match snapshot {
        Some(v) => v,
        None => return Ok(EffectResult::empty()),
    };

    let (target_health, target_mana) = world.systems.player.manager().with_player(target_guid, |player| {
        let hp = (player.stats.max_health as u64 * health_pct as u64 / 100) as u32;
        let mp = (player.power.max_mana() as u64 * health_pct as u64 / 100) as u32;
        (hp.max(1), mp)
    }).unwrap_or((1, 0));

    if let Err(e) = world.systems.death.offer_resurrection(
        target_guid,
        caster_guid,
        &caster_name,
        location,
        map_id,
        instance_id,
        target_health,
        target_mana,
        world,
    ) {
        tracing::warn!("offer_resurrection failed: {}", e);
    }

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_SELF_RESURRECT (94)
///
/// Self-resurrection (Soulstone, Reincarnation, etc.).
pub async fn effect_self_resurrect(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let target_guid = input.caster_guid;
    
    // Check if caster is dead
    let is_dead = world.systems.player.manager().with_player(target_guid, |player| {
        player.stats.health == 0
    }).unwrap_or(false);
    
    if !is_dead {
        return Ok(EffectResult::empty());
    }
    
    // Resurrect at percentage of max health
    let health_pct = input.base_value.max(1).min(100) as u8;
    
    world.systems.player.manager().with_player_mut(target_guid, |player| {
        let new_health = (player.stats.max_health as f32 * health_pct as f32 / 100.0) as u32;
        player.stats.health = new_health.max(1);
        
        tracing::debug!(
            "Self-resurrect: {} self-resurrected at {}% health ({}/{})",
            player.name, health_pct, player.stats.health, player.stats.max_health
        );
    });
    
    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_RESURRECT_NEW (113)
///
/// New resurrection effect (unused in 1.12, but reserved).
pub async fn effect_resurrect_new(input: &EffectInput, world: &World) -> Result<EffectResult> {
    // This is essentially the same as regular resurrect
    // but was added for future expansion
    effect_resurrect(input, world).await
}
