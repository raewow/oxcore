//! Whether a player is in a position to talk to an NPC or use a game object's menu.

use crate::game::common::creature_flags::CREATURE_STATIC_FLAG_VISIBLE_TO_GHOSTS;
use crate::game::creature::ai::units::distance;
use crate::World;
use oxcore_shared::game::reputation::ReputationRank;
use oxcore_shared::protocol::ObjectGuid;

/// How close a player must stand to talk to an NPC.
pub const INTERACTION_DISTANCE: f32 = 5.0;

/// Game objects are measured from their centre, so allow for the object's own size.
const GAMEOBJECT_INTERACTION_DISTANCE: f32 = INTERACTION_DISTANCE + 5.0;

const UNIT_FLAG_NOT_SELECTABLE: u32 = 0x0200_0000;

/// Whether `player` may talk to the creature `npc`. With a non-zero `npc_flag_mask` the
/// creature must also carry one of those NPC flags.
///
/// The creature must be alive, selectable, out of combat, close by and not
/// hostile; ghosts may only talk to creatures visible to ghosts, and the living may not.
/// A player at Unfriendly or worse with the creature's faction is refused.
pub fn can_interact_with_npc(
    world: &World,
    player: ObjectGuid,
    npc: ObjectGuid,
    npc_flag_mask: u32,
) -> bool {
    let Some(creature) = world.managers.creature_mgr.get_creature(npc) else {
        return false;
    };
    if !creature.is_alive()
        || creature.unit_flags & UNIT_FLAG_NOT_SELECTABLE != 0
        || (npc_flag_mask != 0 && creature.npc_flags & npc_flag_mask == 0)
    {
        return false;
    }
    let (faction, static_flags1, position, map_id, instance_id) = (
        creature.faction,
        creature.static_flags1,
        creature.position,
        creature.map_id,
        creature.instance_id,
    );
    drop(creature);
    if world.managers.creature_mgr.is_in_combat(npc) {
        return false;
    }

    let visible_to_ghosts = static_flags1 & CREATURE_STATIC_FLAG_VISIBLE_TO_GHOSTS != 0;
    let reachable = world
        .managers
        .player_mgr
        .with_player(player, |p| {
            if p.taxi.is_flying() || p.map_id != map_id || p.instance_id != instance_id {
                return false;
            }
            if p.is_alive() == visible_to_ghosts {
                return false;
            }
            if crate::game::faction::creature_is_hostile_to_player(&world.dbc.read(), faction, p) {
                return false;
            }
            distance(&p.movement.position, &position) <= INTERACTION_DISTANCE
        })
        .unwrap_or(false);
    if !reachable {
        return false;
    }

    let reputation_faction = {
        let dbc = world.dbc.read();
        dbc.get_faction_template(faction)
            .map(|t| t.faction)
            .filter(|&f| dbc.get_faction(f).is_some_and(|f| f.reputation_list_id >= 0))
    };
    match reputation_faction {
        Some(faction_id) => world
            .systems
            .reputation
            .get_rank(player, faction_id, world)
            .is_none_or(|rank| rank > ReputationRank::Unfriendly),
        None => true,
    }
}

/// Whether a living player is close enough to a spawned game object to use its menu.
pub fn can_interact_with_gameobject(world: &World, player: ObjectGuid, go: ObjectGuid) -> bool {
    let Some((position, map_id, instance_id, in_world)) = world
        .managers
        .gameobject_mgr
        .with_gameobject(go, |g| (g.position, g.map_id, g.instance_id, g.in_world))
    else {
        return false;
    };
    in_world
        && world
            .managers
            .player_mgr
            .with_player(player, |p| {
                p.is_alive()
                    && !p.taxi.is_flying()
                    && p.map_id == map_id
                    && p.instance_id == instance_id
                    && distance(&p.movement.position, &position) <= GAMEOBJECT_INTERACTION_DISTANCE
            })
            .unwrap_or(false)
}
