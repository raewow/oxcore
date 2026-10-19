//! The PvP flag.
//!
//! A player is flagged by choice (the PvP toggle), by entering an area that
//! enforces it (enemy territory on PvP realms, enemy capitals, battlegrounds),
//! or by fighting a flagged player. Once nothing holds the flag, it drops five
//! minutes later; any further PvP restarts the five minutes.

use oxcore_shared::messages::update::{
    ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
};
use oxcore_shared::protocol::ObjectGuid;

use crate::game::common::unit_flags;
use crate::game::common::update_fields::{PLAYER_FLAGS, UNIT_FIELD_FLAGS};
use crate::World;

/// Stores the player's own PvP preference.
pub const PLAYER_FLAGS_PVP_DESIRED: u32 = 0x0000_0200;
/// How long the flag lingers once nothing holds it.
pub const PVP_FLAG_LINGER_MS: u32 = 300_000;

/// AreaTable faction-group values.
const AREA_TEAM_NONE: u32 = 0;
const AREA_TEAM_ALLIANCE: u32 = 2;
const AREA_TEAM_HORDE: u32 = 4;
/// AreaTable flag for capital cities.
const AREA_FLAG_CAPITAL: u32 = 0x100;

/// What keeps a player's PvP flag up.
#[derive(Debug, Clone, Default)]
pub struct PvpState {
    /// Time left before the flag drops, once nothing holds it.
    pub timer_remaining_ms: u32,
    /// In an area that keeps everyone flagged.
    pub in_enforced_area: bool,
    /// Fighting other players right now.
    pub in_pvp_combat: bool,
    /// Standing in an objective's capture area.
    pub in_capture_point: bool,
    /// Carrying a battleground flag.
    pub is_flag_carrier: bool,
}

impl PvpState {
    /// Whether anything pauses the countdown.
    fn holds_flag(&self, desired: bool) -> bool {
        self.in_pvp_combat
            || self.in_enforced_area
            || self.in_capture_point
            || self.is_flag_carrier
            || desired
    }
}

/// Whether a player is flagged.
pub fn is_pvp(world: &World, guid: ObjectGuid) -> bool {
    world
        .managers
        .player_mgr
        .with_player(guid, |p| p.unit_flags & unit_flags::PVP != 0)
        .unwrap_or(false)
}

/// Raise or drop the flag and show it to the player and everyone nearby.
fn set_pvp(world: &World, guid: ObjectGuid, on: bool) {
    let Some(flags) = world.managers.player_mgr.with_player_mut(guid, |p| {
        let flags = if on {
            p.unit_flags | unit_flags::PVP
        } else {
            p.unit_flags & !unit_flags::PVP
        };
        let changed = flags != p.unit_flags;
        p.unit_flags = flags;
        changed.then_some(flags)
    }) else {
        return;
    };
    let Some(flags) = flags else {
        return;
    };
    let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
        ValuesUpdateBlock::new(guid, ObjectType::Player).set_field(UNIT_FIELD_FLAGS, flags),
    ));
    world
        .managers
        .broadcast_mgr
        .broadcast_msg_nearby(guid, &update, true);
    world.systems.group.set_member_update(
        guid,
        oxcore_shared::game::group::group_update_flags::STATUS,
    );
}

/// Flag the player (restarting the five minutes), or try to drop the flag.
///
/// Dropping only happens once the countdown has run out, unless `overriding`.
pub fn update_pvp(world: &World, guid: ObjectGuid, state: bool, overriding: bool) {
    let Some((flagged, remaining)) = world.managers.player_mgr.with_player_mut(guid, |p| {
        if state {
            p.pvp.timer_remaining_ms = PVP_FLAG_LINGER_MS;
        } else if overriding {
            p.pvp.timer_remaining_ms = 0;
        }
        (p.unit_flags & unit_flags::PVP != 0, p.pvp.timer_remaining_ms)
    }) else {
        return;
    };
    if state {
        if !flagged {
            set_pvp(world, guid, true);
        }
    } else if flagged && remaining == 0 {
        set_pvp(world, guid, false);
    }
}

/// The PvP toggle: `explicit` sets the preference, otherwise it flips.
pub fn toggle(world: &World, guid: ObjectGuid, explicit: Option<bool>) {
    let Some(desired) = world.managers.player_mgr.with_player(guid, |p| {
        let current = p.player_flags & PLAYER_FLAGS_PVP_DESIRED != 0;
        explicit.unwrap_or(!current)
    }) else {
        return;
    };
    world
        .systems
        .group
        .set_player_flag(guid, PLAYER_FLAGS_PVP_DESIRED, desired);
    if desired {
        update_pvp(world, guid, true, false);
    }
}

/// Send the player's own flags after login so the toggle shows its state.
pub fn refresh_player_flags(world: &World, guid: ObjectGuid) {
    if let Some(flags) = world.managers.player_mgr.with_player(guid, |p| p.player_flags) {
        let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
            ValuesUpdateBlock::new(guid, ObjectType::Player).set_field(PLAYER_FLAGS, flags),
        ));
        world.managers.broadcast_mgr.send_msg_to_player(guid, update);
    }
}

/// Whether a zone keeps a player of `team` flagged.
///
/// On a PvP realm every zone the other faction controls does; on a normal
/// realm only the other faction's capitals. Battlegrounds always do.
pub fn zone_enforces_pvp(
    area_team: u32,
    area_flags: u32,
    player_is_alliance: bool,
    pvp_realm: bool,
    in_battleground: bool,
) -> bool {
    match area_team {
        AREA_TEAM_ALLIANCE => {
            !player_is_alliance && (pvp_realm || area_flags & AREA_FLAG_CAPITAL != 0)
        }
        AREA_TEAM_HORDE => player_is_alliance && (pvp_realm || area_flags & AREA_FLAG_CAPITAL != 0),
        AREA_TEAM_NONE => pvp_realm || in_battleground,
        // Sanctuaries and anything else never enforce it.
        _ => false,
    }
}

/// Re-evaluate the enforced area after a zone change (or login).
pub fn on_zone_change(world: &World, guid: ObjectGuid, zone_id: u32) {
    let Some((map_id, is_alliance, on_taxi, gm)) = world.managers.player_mgr.with_player(guid, |p| {
        (
            p.map_id,
            p.get_team() == oxcore_shared::game::chat::Team::Alliance,
            p.unit_flags & unit_flags::ON_TAXI != 0,
            p.gm_mode,
        )
    }) else {
        return;
    };
    let (area_team, area_flags) = world
        .dbc
        .read()
        .get_area(zone_id)
        .map(|area| (area.team, area.flags))
        .unwrap_or((u32::MAX, 0));
    let in_battleground = crate::game::instance::lifecycle::map_entry(world, map_id)
        .is_some_and(|entry| entry.is_battleground());
    let enforced = !gm
        && zone_enforces_pvp(
            area_team,
            area_flags,
            is_alliance,
            world.config.is_pvp_realm,
            in_battleground,
        );
    world
        .managers
        .player_mgr
        .with_player_mut(guid, |p| p.pvp.in_enforced_area = enforced);
    if enforced && !on_taxi {
        update_pvp(world, guid, true, false);
    }
}

/// One player hurt another: both sides of a fight involving a flagged player
/// stay (or become) flagged.
pub fn on_player_damaged_player(world: &World, attacker: ObjectGuid, victim: ObjectGuid) {
    if attacker == victim {
        return;
    }
    if is_pvp(world, victim) {
        world
            .managers
            .player_mgr
            .with_player_mut(attacker, |p| p.pvp.in_pvp_combat = true);
        update_pvp(world, attacker, true, false);
    }
    if is_pvp(world, attacker) {
        world
            .managers
            .player_mgr
            .with_player_mut(victim, |p| p.pvp.in_pvp_combat = true);
        update_pvp(world, victim, true, false);
    }
}

/// A player helped a flagged player (heal or buff): the helper is flagged too.
pub fn on_player_assisted_player(world: &World, helper: ObjectGuid, target: ObjectGuid) {
    if helper != target && is_pvp(world, target) {
        update_pvp(world, helper, true, false);
    }
}

/// Count down every player's flag and drop the ones that ran out.
pub fn update(world: &World, diff_ms: u32) {
    let mut expired = Vec::new();
    for guid in world.managers.player_mgr.collect_online_guids() {
        let done = world.managers.player_mgr.with_player_mut(guid, |p| {
            if p.unit_flags & unit_flags::IN_COMBAT == 0 {
                p.pvp.in_pvp_combat = false;
            }
            if p.unit_flags & unit_flags::PVP == 0 {
                return false;
            }
            let desired = p.player_flags & PLAYER_FLAGS_PVP_DESIRED != 0;
            if !p.pvp.holds_flag(desired) {
                p.pvp.timer_remaining_ms = p.pvp.timer_remaining_ms.saturating_sub(diff_ms);
            }
            p.pvp.timer_remaining_ms == 0
        });
        if done == Some(true) {
            expired.push(guid);
        }
    }
    for guid in expired {
        update_pvp(world, guid, false, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enemy_capitals_enforce_pvp_on_any_realm() {
        assert!(zone_enforces_pvp(AREA_TEAM_HORDE, AREA_FLAG_CAPITAL, true, false, false));
        assert!(!zone_enforces_pvp(AREA_TEAM_HORDE, AREA_FLAG_CAPITAL, false, false, false));
        assert!(!zone_enforces_pvp(AREA_TEAM_HORDE, 0, true, false, false));
    }

    #[test]
    fn pvp_realms_flag_enemy_and_contested_territory() {
        assert!(zone_enforces_pvp(AREA_TEAM_ALLIANCE, 0, false, true, false));
        assert!(!zone_enforces_pvp(AREA_TEAM_ALLIANCE, 0, true, true, false));
        assert!(zone_enforces_pvp(AREA_TEAM_NONE, 0, true, true, false));
        assert!(!zone_enforces_pvp(AREA_TEAM_NONE, 0, true, false, false));
    }

    #[test]
    fn battlegrounds_always_enforce_pvp_and_sanctuaries_never() {
        assert!(zone_enforces_pvp(AREA_TEAM_NONE, 0, true, false, true));
        assert!(!zone_enforces_pvp(6, 0, true, true, true));
    }

    #[test]
    fn anything_holding_the_flag_pauses_the_countdown() {
        let mut state = PvpState::default();
        assert!(!state.holds_flag(false));
        assert!(state.holds_flag(true));
        state.is_flag_carrier = true;
        assert!(state.holds_flag(false));
    }
}
