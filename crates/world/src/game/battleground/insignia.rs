//! Taking a dead enemy's insignia in a battleground.
//!
//! A player who dies in a battleground can be skinned, and their body is
//! marked lootable. An enemy casting Remove Insignia on either releases the
//! victim's spirit if they have not released yet, and opens the body: a few
//! coins and the battleground's player loot, shared with anyone else of the
//! killers' side who takes it after. Once emptied it is plain bones.

use oxcore_shared::messages::battleground::SmsgPlayerSkinned;
use oxcore_shared::messages::update::{
    ObjectType, SmsgUpdateObject, UpdateBlockData, ValuesUpdateBlock,
};
use oxcore_shared::protocol::update_fields::{CORPSE_FIELD_DYNAMIC_FLAGS, UNIT_FIELD_FLAGS};
use oxcore_shared::protocol::ObjectGuid;
use rand::Rng;

use crate::game::common::unit_flags;
use crate::game::player::death::DeathState;
use crate::World;

/// Mark or clear a dead player as skinnable, and show it to everyone nearby.
pub fn set_skinnable(world: &World, player: ObjectGuid, on: bool) {
    let Some(Some(flags)) = world.managers.player_mgr.with_player_mut(player, |p| {
        let flags = if on {
            p.unit_flags | unit_flags::SKINNABLE
        } else {
            p.unit_flags & !unit_flags::SKINNABLE
        };
        let changed = flags != p.unit_flags;
        p.unit_flags = flags;
        changed.then_some(flags)
    }) else {
        return;
    };
    let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
        ValuesUpdateBlock::new(player, ObjectType::Player)
            .set_field(UNIT_FIELD_FLAGS, flags | unit_flags::PLAYER_CONTROLLED),
    ));
    world
        .managers
        .broadcast_mgr
        .broadcast_msg_nearby(player, &update, true);
}

/// Coins on a taken insignia: random around a curve of the victim's level.
fn insignia_gold(level: u32, rate: f32) -> u32 {
    let roll = rand::thread_rng().gen_range(50..=150) as f32;
    (roll * 0.016 * (level as f32 / 5.76).powf(2.5) * rate) as u32
}

/// Remove Insignia, cast by `looter` on a body (`target` a corpse) or a dead
/// player who has not released yet (`target` the player).
pub fn remove_insignia(world: &World, looter: ObjectGuid, target: ObjectGuid) {
    let owner = if target.is_corpse() {
        world.managers.corpse_mgr.get(target).map(|c| c.owner_guid)
    } else if target.is_player() {
        Some(target)
    } else {
        None
    };
    let Some(owner) = owner else {
        return;
    };
    let players = &world.managers.player_mgr;
    let Some(looter_team) = players.with_player(looter, |p| p.is_alive().then(|| p.get_team())).flatten()
    else {
        return;
    };
    let owner_state = players.with_player(owner, |p| (p.get_team(), p.death.death_state, p.level));
    let corpse = if target.is_corpse() {
        Some(target)
    } else {
        players.with_player(owner, |p| p.death.corpse_guid).flatten()
    };
    let Some(corpse) = corpse else {
        return;
    };
    let Some(body) = world.managers.corpse_mgr.get(corpse) else {
        return;
    };
    // Only the other side may take it; an offline owner's side is unknown,
    // so their body is anyone's.
    if !body.lootable || owner_state.is_some_and(|(team, ..)| team == looter_team) {
        return;
    }

    // A victim still lying at the body is released first.
    if let Some((_, DeathState::JustDied | DeathState::Corpse, _)) = owner_state {
        let _ = world.systems.death.handle_release_spirit(owner, world);
    }

    let first = !body.insignia_taken;
    if first {
        world
            .managers
            .corpse_mgr
            .with_corpse_mut(corpse, |c| c.insignia_taken = true);
        send_dynamic_flags(world, corpse);
        if owner_state.is_some() {
            world
                .managers
                .broadcast_mgr
                .send_msg_to_player(owner, SmsgPlayerSkinned { free_repop: false });
        }
        set_skinnable(world, owner, false);
    }

    let level = owner_state.map(|(_, _, level)| u32::from(level)).unwrap_or(1);
    let gold = insignia_gold(level, world.config.rate_drop_money);
    let reference = crate::game::battleground::system::match_of(world, looter)
        .and_then(|id| world.managers.battleground_mgr.get_match(id))
        .and_then(|m| world.managers.battleground_mgr.template(m.bg_type))
        .map(|t| t.player_loot_id)
        .unwrap_or(0);
    world
        .systems
        .loot
        .open_insignia_loot(looter, corpse, gold, reference, world);
}

/// The body's loot ran out: it is no longer shown as lootable.
pub fn on_emptied(world: &World, corpse: ObjectGuid) {
    world
        .managers
        .corpse_mgr
        .with_corpse_mut(corpse, |c| c.insignia_taken = false);
    send_dynamic_flags(world, corpse);
}

/// Show a body's lootable state to the players on its map copy.
fn send_dynamic_flags(world: &World, corpse: ObjectGuid) {
    let Some(body) = world.managers.corpse_mgr.get(corpse) else {
        return;
    };
    let flags = crate::game::corpse::manager::corpse_dynamic_flags(&body);
    let update = SmsgUpdateObject::new().add_block(UpdateBlockData::Values(
        ValuesUpdateBlock::new(corpse, ObjectType::Corpse)
            .set_field(CORPSE_FIELD_DYNAMIC_FLAGS, flags),
    ));
    let viewers: Vec<ObjectGuid> = world
        .managers
        .player_mgr
        .collect_online_guids()
        .into_iter()
        .filter(|&guid| {
            world
                .managers
                .player_mgr
                .with_player(guid, |p| p.map_id == body.map_id && p.instance_id == body.instance_id)
                .unwrap_or(false)
        })
        .collect();
    world
        .managers
        .broadcast_mgr
        .broadcast_msg_to_players(&viewers, &update);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insignia_coins_grow_with_the_victims_level() {
        // At level 60 the curve gives roughly 3-8 silver; at 10 a few copper.
        for _ in 0..50 {
            let low = insignia_gold(10, 1.0);
            let high = insignia_gold(60, 1.0);
            assert!(low <= 10, "{low}");
            assert!((280..=841).contains(&high), "{high}");
        }
    }
}
