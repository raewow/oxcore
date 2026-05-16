//! Script and Dummy Spell Effects
//!
//! Handles script-based and dummy effects that require custom logic.

use super::{EffectInput, EffectResult};
use crate::world::World;
use anyhow::Result;

/// SPELL_EFFECT_DUMMY (3)
///
/// Dummy effect - handled by script or hardcoded logic.
/// Routes to Lua OnEffectDummy if a script is registered for the target's entry.
pub async fn effect_dummy(input: &EffectInput, world: &World) -> Result<EffectResult> {
    if let Some(target_guid) = input.target_guid {
        // Look up the target's entry (creature or GO)
        let target_entry = world
            .managers
            .creature_mgr
            .with_creature(target_guid, |c| c.entry)
            .or_else(|| {
                world
                    .managers
                    .gameobject_mgr
                    .with_gameobject(target_guid, |go| go.entry)
            });

        if let Some(entry) = target_entry {
            if let Some(script) = world.managers.lua_mgr.get_effect_dummy_script(entry) {
                let (handled, actions) = world.managers.lua_mgr.with_lua(|lua| {
                    script.on_effect_dummy(
                        lua,
                        input.caster_guid,
                        input.spell_id,
                        input.effect_index,
                        target_guid,
                    )
                });
                if !actions.is_empty() {
                    crate::world::core::lua::execute_gossip_actions(
                        actions,
                        input.caster_guid,
                        target_guid,
                        world,
                    )
                    .await?;
                }
                if handled {
                    return Ok(EffectResult::empty());
                }
            }
        }
    }

    tracing::debug!(
        "Dummy effect: spell={} caster={:?} target={:?} (no script handler)",
        input.spell_id,
        input.caster_guid,
        input.target_guid
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_SCRIPT_EFFECT (77)
///
/// General script effect handler.
pub async fn effect_script_effect(input: &EffectInput, _world: &World) -> Result<EffectResult> {
    // Similar to dummy, but different routing
    // TODO: Implement script system routing

    tracing::debug!(
        "Script effect: spell={} caster={:?}",
        input.spell_id,
        input.caster_guid
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_SEND_EVENT (61)
///
/// Triggers a script event, routing to OnProcessEventId Lua callback if registered.
pub async fn effect_send_event(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let event_id = input.misc_value as u32;

    if event_id > 0 {
        if let Some(script) = world.managers.lua_mgr.get_process_event_script(event_id) {
            // Build a minimal player snapshot for the caster
            let player_snap =
                crate::world::core::lua::build_player_snapshot(input.caster_guid, world);
            let actions = world.managers.lua_mgr.with_lua(|lua| {
                script.on_process_event(lua, &player_snap, input.caster_guid, true)
            });
            if !actions.is_empty() {
                crate::world::core::lua::execute_gossip_actions(
                    actions,
                    input.caster_guid,
                    input
                        .target_guid
                        .unwrap_or_else(crate::shared::protocol::ObjectGuid::empty),
                    world,
                )
                .await?;
            }
        }
    }

    tracing::debug!(
        "Send event: caster={:?} event={}",
        input.caster_guid,
        event_id
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_TRIGGER_SPELL (64)
///
/// Triggers another spell.
pub async fn effect_trigger_spell(input: &EffectInput, world: &World) -> Result<EffectResult> {
    let triggered_spell_id = input.misc_value as u32;

    if triggered_spell_id == 0 {
        return Ok(EffectResult::empty());
    }

    // TODO: Trigger the spell via SpellSystem

    tracing::debug!(
        "Trigger spell: caster={:?} triggered_spell={}",
        input.caster_guid,
        triggered_spell_id
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_TRIGGER_MISSILE (32)
///
/// Triggers a missile (projectile) spell.
pub async fn effect_trigger_missile(input: &EffectInput, _world: &World) -> Result<EffectResult> {
    let missile_spell_id = input.misc_value as u32;

    // TODO: Create missile projectile

    tracing::debug!(
        "Trigger missile: caster={:?} missile_spell={}",
        input.caster_guid,
        missile_spell_id
    );

    Ok(EffectResult::empty())
}

/// SPELL_EFFECT_NOSTALRIUS (131)
///
/// Custom server-specific effect.
pub async fn effect_nostalrius(input: &EffectInput, _world: &World) -> Result<EffectResult> {
    // Server-specific custom effects
    tracing::debug!(
        "Nostalrius custom effect: spell {}, caster {}",
        input.spell_id,
        input.caster_guid
    );

    Ok(EffectResult::empty())
}
