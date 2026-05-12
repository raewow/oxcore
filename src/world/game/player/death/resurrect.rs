//! Resurrection methods
//!
//! Five distinct resurrection paths exist, each with different triggers
//! and consequences.

use super::flow::*;
use super::state::{DeathState, DeathSystemState, ResurrectionData};
use crate::shared::protocol::ObjectGuid;
use crate::shared::protocol::Position;

/// Resurrection method determines the consequences of coming back to life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResurrectionMethod {
    /// Player walked their ghost back to their corpse (within 40 yards).
    /// Result: 50% HP/mana, no sickness, no extra durability loss.
    CorpseRun,
    /// Player talked to the spirit healer NPC at the graveyard.
    /// Result: Full HP/mana, resurrection sickness, +25% durability loss.
    SpiritHealer,
    /// Another player cast a resurrection spell (Druid Rebirth, Priest
    /// Resurrection, Paladin Redemption, Shaman Ancestral Spirit).
    /// Result: Spell-defined HP/mana, no sickness.
    PlayerSpell,
    /// Self-resurrection via Warlock Soulstone or Shaman Reincarnation.
    /// Result: Ability-defined HP/mana, no sickness.
    SelfResurrection,
    /// Battleground auto-resurrection on the 30-second wave timer.
    /// Result: Full HP/mana, no sickness, no durability loss.
    Battleground,
}

impl ResurrectionMethod {
    /// Whether this method applies resurrection sickness.
    pub fn applies_sickness(&self) -> bool {
        matches!(self, ResurrectionMethod::SpiritHealer)
    }

    /// Whether this method causes additional durability loss beyond
    /// the initial death penalty.
    pub fn applies_extra_durability_loss(&self) -> bool {
        matches!(self, ResurrectionMethod::SpiritHealer)
    }
}

/// Execute a corpse-run resurrection.
///
/// Called when the player clicks "Resurrect" near their corpse
/// (CMSG_RECLAIM_CORPSE). The server validates proximity before calling this.
///
/// Restores 50% health and 50% mana. No sickness, no extra durability loss.
pub fn resurrect_at_corpse(
    state: &mut DeathSystemState,
    max_health: u32,
    max_mana: u32,
) -> (u32, u32) {
    state.death_state = DeathState::JustAlived;
    state.death_timer_ms = 0;
    state.corpse_guid = None;
    state.resurrection_data = None;

    let health = (max_health as f32 * CORPSE_RES_HEALTH_PCT) as u32;
    let mana = (max_mana as f32 * CORPSE_RES_MANA_PCT) as u32;

    (health.max(1), mana)
}

/// Execute a spirit healer resurrection.
///
/// Called when the player interacts with the spirit healer NPC at a
/// graveyard. Applies resurrection sickness and additional durability loss.
pub fn resurrect_at_spirit_healer(
    state: &mut DeathSystemState,
    max_health: u32,
    max_mana: u32,
) -> (u32, u32) {
    state.death_state = DeathState::JustAlived;
    state.death_timer_ms = 0;
    state.corpse_guid = None;
    state.resurrection_data = None;

    // Spirit healer gives full HP/mana (sickness debuff then reduces stats)
    (max_health, max_mana)
}

/// Execute a player-spell resurrection.
///
/// Called when the dead player accepts a resurrection offer from another
/// player (CMSG_RESURRECT_RESPONSE with accept=1). Uses the health and
/// mana values stored in the resurrection data.
pub fn resurrect_from_spell(state: &mut DeathSystemState) -> Option<(u32, u32, Position, u32)> {
    let res_data = state.resurrection_data.take()?;

    state.death_state = DeathState::JustAlived;
    state.death_timer_ms = 0;
    state.corpse_guid = None;

    Some((
        res_data.health.max(1),
        res_data.mana,
        res_data.location,
        res_data.map_id,
    ))
}

/// Offer a resurrection to a dead player.
///
/// Called by the spell system when a resurrection spell finishes casting.
/// The dead player receives a popup dialog (SMSG_RESURRECT_REQUEST) and
/// can accept or decline.
///
/// Only one resurrection offer can be pending at a time. A new offer
/// replaces any existing one.
pub fn offer_resurrection(
    state: &mut DeathSystemState,
    resurrector_guid: ObjectGuid,
    location: Position,
    map_id: u32,
    instance_id: u32,
    health: u32,
    mana: u32,
) {
    state.resurrection_data = Some(ResurrectionData::new(
        resurrector_guid,
        location,
        map_id,
        instance_id,
        health,
        mana,
    ));
}

/// Decline a pending resurrection offer.
///
/// Called when the player clicks "Decline" on the resurrection popup
/// (CMSG_RESURRECT_RESPONSE with accept=0).
pub fn decline_resurrection(state: &mut DeathSystemState) {
    state.resurrection_data = None;
}

/// Check if the given GUID matches the pending resurrection offer.
pub fn is_resurrection_requested_by(
    state: &DeathSystemState,
    resurrector_guid: ObjectGuid,
) -> bool {
    state
        .resurrection_data
        .as_ref()
        .map_or(false, |data| data.resurrector_guid == resurrector_guid)
}
