//! Group reward-distance helpers.
//!
//! Ports of `Player::IsAtGroupRewardDistance` (reference `Objects/Player.cpp:20122`) and
//! `WorldObject::IsWithinLootXPDist` (reference `Objects/Object.cpp:1477`). Used by group XP
//! distribution, loot rolls, and round-robin looter rotation.

use crate::World;
use oxcore_shared::protocol::{ObjectGuid, Position};

/// Elite rank of world bosses (adds 150 yards to the reward distance).
pub const CREATURE_ELITE_WORLDBOSS: u8 = 3;

/// Default group XP/loot reward distance in yards (`MaxGroupXPDistance`).
pub const DEFAULT_GROUP_XP_DISTANCE: f32 = 74.0;

/// Distance added to the reward radius for world-boss kills.
const WORLDBOSS_DISTANCE_BONUS: f32 = 150.0;

/// Whether `player_guid` is close enough to `source_pos` to earn kill rewards or roll on loot.
///
/// Mirrors the reference semantics: the player must be on the same map and instance, within
/// `group_xp_distance` yards (plus 150 for world-boss sources). A dead player falls back to their
/// corpse position, as in `Player::IsAtGroupRewardDistance`.
pub fn is_at_group_reward_distance(
    world: &World,
    player_guid: ObjectGuid,
    source_pos: &Position,
    source_map: u32,
    source_instance: u32,
    source_rank: u8,
) -> bool {
    let Some(player) = world.managers.player_mgr.get_player(player_guid) else {
        return false;
    };

    // Same map AND instance, as in `WorldObject::IsWithinLootXPDist`'s `IsInMap` check.
    if player.map_id != source_map || player.instance_id != source_instance {
        return false;
    }

    let distance = reward_distance(world, source_rank);
    if player
        .movement
        .position
        .is_within_range(source_pos, distance)
    {
        return true;
    }

    // Dead players fall back to their corpse position (reference `Player::IsAtGroupRewardDistance`).
    let alive = matches!(
        player.death.death_state,
        crate::game::player::death::DeathState::Alive
    );
    if !alive {
        if let Some(corpse_position) = player.death.corpse_position {
            if player.death.corpse_map_id == Some(source_map) {
                return corpse_position.is_within_range(source_pos, distance);
            }
        }
    }

    false
}

/// Effective reward radius for a source with the given elite rank.
fn reward_distance(world: &World, source_rank: u8) -> f32 {
    let base = world.config.group_xp_distance;
    if source_rank == CREATURE_ELITE_WORLDBOSS {
        base + WORLDBOSS_DISTANCE_BONUS
    } else {
        base
    }
}
