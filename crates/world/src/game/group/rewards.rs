//! Group kill-reward distribution.
//!
//! Ports `Group::RewardGroupAtKill` and `Group::GetDataForXPAtKill` (reference
//! `game/Group/Group.cpp:2373` and `:1299`) together with `MaNGOS::XP::xp_in_group_rate`
//! (`game/Formulas.h:161`). Invoked from the creature-death pipeline whenever the loot
//! recipient belongs to a group; every in-range member receives quest credit and XP.

use crate::game::group::reward::is_at_group_reward_distance;
use crate::game::player::death::DeathState;
use crate::game::player::experience;
use crate::World;
use oxcore_shared::game::experience::XpSource;
use oxcore_shared::game::group::GroupError;
use oxcore_shared::protocol::{ObjectGuid, Position};

/// Group XP rate for a member count (`MaNGOS::XP::xp_in_group_rate`, Formulas.h:161).
///
/// Two-member parties have no bonus; the rate climbs to 1.4 for five members, then decays
/// `1 - count * 0.05`, floored at 0.01, for raids. The reference's `is_raid` argument is
/// unused.
pub fn xp_in_group_rate(count: u32) -> f32 {
    match count {
        0 | 1 | 2 => 1.0,
        3 => 1.166,
        4 => 1.3,
        5 => 1.4,
        _ => (1.0 - count as f32 * 0.05).max(0.01),
    }
}

/// Accumulated XP-relevant data over the members within reward distance
/// (`Group::GetDataForXPAtKill`). Levels are `u32` to avoid casts in arithmetic.
#[derive(Default)]
struct XpKillData {
    count: u32,
    sum_level: u32,
    member_with_max_level: Option<(ObjectGuid, u32)>,
    not_gray_member_with_max_level: Option<(ObjectGuid, u32)>,
}

/// Distribute kill rewards — quest credit and XP — across a group.
///
/// `tapper_guid` is the player who dealt the killing blow (the reference's `additional`
/// argument); they are counted and rewarded alongside the other members, but never twice.
/// The victim is a creature, so the PvP branches of the reference do not apply.
pub async fn reward_group_at_kill(
    world: &World,
    group_id: u32,
    victim_guid: ObjectGuid,
    victim_entry: u32,
    victim_level: u8,
    victim_rank: u8,
    victim_pos: &Position,
    victim_map: u32,
    victim_instance: u32,
    tapper_guid: ObjectGuid,
) -> Result<(), GroupError> {
    let group = world
        .systems
        .group
        .get_group(group_id)
        .ok_or(GroupError::NotInGroup)?;

    // Gather XP data for every alive, in-world, in-range member; the tapper is processed
    // last as the reference's `additional` argument.
    let mut data = XpKillData::default();
    for member in &group.members {
        if member.guid == tapper_guid {
            continue;
        }
        accumulate_xp_data(
            world,
            &mut data,
            member.guid,
            victim_level,
            victim_pos,
            victim_map,
            victim_instance,
            victim_rank,
        );
    }
    accumulate_xp_data(
        world,
        &mut data,
        tapper_guid,
        victim_level,
        victim_pos,
        victim_map,
        victim_instance,
        victim_rank,
    );

    // No in-range member — nothing to reward.
    let Some(_) = data.member_with_max_level else {
        return Ok(());
    };

    // Base XP is computed against the highest-level member for whom the victim is not gray.
    let xp = match data.not_gray_member_with_max_level {
        Some((_, level)) => {
            let is_elite = victim_rank != 0 && victim_rank != 4;
            experience::calculate_creature_xp(victim_level, level as u8, is_elite)
        }
        None => 0,
    };
    let group_rate = xp_in_group_rate(data.count);

    for member in &group.members {
        if member.guid == tapper_guid {
            continue;
        }
        reward_group_member(
            world,
            member.guid,
            victim_guid,
            victim_entry,
            &data,
            group_rate,
            xp,
            victim_pos,
            victim_map,
            victim_instance,
            victim_rank,
        )
        .await;
    }
    reward_group_member(
        world,
        tapper_guid,
        victim_guid,
        victim_entry,
        &data,
        group_rate,
        xp,
        victim_pos,
        victim_map,
        victim_instance,
        victim_rank,
    )
    .await;

    Ok(())
}

/// Count a member into `data` if they are alive, in the world, and within reward distance
/// (`Group::GetDataForXPAtKill`).
fn accumulate_xp_data(
    world: &World,
    data: &mut XpKillData,
    member_guid: ObjectGuid,
    victim_level: u8,
    victim_pos: &Position,
    victim_map: u32,
    victim_instance: u32,
    victim_rank: u8,
) {
    let Some(player) = world.managers.player_mgr.get_player(member_guid) else {
        return; // not in the world
    };
    if !matches!(player.death.death_state, DeathState::Alive) {
        return;
    }
    if !is_at_group_reward_distance(
        world,
        member_guid,
        victim_pos,
        victim_map,
        victim_instance,
        victim_rank,
    ) {
        return;
    }

    let level = player.level as u32;
    data.count += 1;
    data.sum_level += level;

    if data.member_with_max_level.map(|(_, l)| l).unwrap_or(0) < level {
        data.member_with_max_level = Some((member_guid, level));
    }

    if victim_level as u32 > experience::get_gray_level(player.level) as u32 {
        if data
            .not_gray_member_with_max_level
            .map(|(_, l)| l)
            .unwrap_or(0)
            < level
        {
            data.not_gray_member_with_max_level = Some((member_guid, level));
        }
    }
}

/// Reward one group member (`Group::RewardGroupAtKill_helper`): quest credit for alive or
/// undeclared-body members, XP only for alive members at or below the highest non-gray level.
async fn reward_group_member(
    world: &World,
    member_guid: ObjectGuid,
    victim_guid: ObjectGuid,
    victim_entry: u32,
    data: &XpKillData,
    group_rate: f32,
    xp: u32,
    victim_pos: &Position,
    victim_map: u32,
    victim_instance: u32,
    victim_rank: u8,
) {
    let Some(player) = world.managers.player_mgr.get_player(member_guid) else {
        return; // not in the world
    };
    if !is_at_group_reward_distance(
        world,
        member_guid,
        victim_pos,
        victim_map,
        victim_instance,
        victim_rank,
    ) {
        return;
    }

    let is_alive = matches!(player.death.death_state, DeathState::Alive);
    let member_level = player.level;

    // Quest objectives are met by alive members and by dead members whose body is still at
    // the corpse (spirit not yet released — i.e. not `DeathState::Dead`).
    if is_alive || !matches!(player.death.death_state, DeathState::Dead) {
        world
            .systems
            .quest
            .handle_kill_credit(member_guid, victim_entry, victim_guid);
        // Filling an objective can take the quest's objects out of play.
        crate::game::gameobject::quest_activation::refresh_quest_gameobjects(member_guid, world);
    }

    // XP only for alive members.
    if !is_alive {
        return;
    }

    let itr_xp = member_xp(data, member_level, xp, group_rate);
    if itr_xp > 0 {
        let _ = world
            .systems
            .experience
            .add_xp(
                member_guid,
                itr_xp,
                XpSource::Kill,
                Some(victim_guid),
                group_rate,
            )
            .await;
    }
}

/// Per-member XP share, mirroring `RewardGroupAtKill_helper`'s `itr_xp` computation.
///
/// Returns 0 for members above the highest non-gray member (the victim is gray to them).
fn member_xp(data: &XpKillData, member_level: u8, xp: u32, group_rate: f32) -> u32 {
    let rate = group_rate * member_level as f32 / data.sum_level.max(1) as f32;
    match data.not_gray_member_with_max_level {
        Some((_, not_gray_level)) if (member_level as u32) <= not_gray_level => {
            let halved = data.member_with_max_level != data.not_gray_member_with_max_level;
            if halved {
                (xp as f32 * rate / 2.0) as u32 + 1
            } else {
                (xp as f32 * rate) as u32
            }
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guid(n: u32) -> ObjectGuid {
        ObjectGuid::new_player(n)
    }

    #[test]
    fn test_xp_in_group_rate() {
        assert_eq!(xp_in_group_rate(0), 1.0);
        assert_eq!(xp_in_group_rate(1), 1.0);
        assert_eq!(xp_in_group_rate(2), 1.0);
        assert_eq!(xp_in_group_rate(3), 1.166);
        assert_eq!(xp_in_group_rate(4), 1.3);
        assert_eq!(xp_in_group_rate(5), 1.4);
        // Raid decay: 1 - 6*0.05 = 0.7
        assert_eq!(xp_in_group_rate(6), 0.7);
        // Floor at 0.01
        assert_eq!(xp_in_group_rate(40), 0.01);
    }

    #[test]
    fn test_member_xp_mixed_level_split() {
        // 3-man group, levels 10/20/30, victim level 15 (gray for the 30).
        // gray(30) = 30 - 1 - 6 = 23 > 15, so the level-30 member is not "not gray".
        // group_rate(3) = 1.166, sum_level = 60.
        let mut data = XpKillData::default();
        data.count = 3;
        data.sum_level = 60;
        data.member_with_max_level = Some((guid(30), 30));
        data.not_gray_member_with_max_level = Some((guid(20), 20));

        // xp = calculate_creature_xp(15, 20, false):
        //   zero_diff(20) = 11, factor = (11 + 15 - 20) / 11 = 0.54545
        //   base = (20*5 + 45) * 0.54545 = 79.09 -> 79
        let xp = experience::calculate_creature_xp(15, 20, false);
        assert_eq!(xp, 79);

        // Levels 10 and 20 are both at/below the not-gray level 20, and the max-level member
        // (30) differs from the not-gray max (20), so both shares are halved.
        assert_eq!(member_xp(&data, 30, xp, 1.166), 0); // gray for level 30
        assert_eq!(member_xp(&data, 20, xp, 1.166), 16);
        assert_eq!(member_xp(&data, 10, xp, 1.166), 8);
    }

    #[test]
    fn test_member_xp_full_share() {
        // 2-man group, levels 10/12, victim level 10 (not gray for either).
        // group_rate(2) = 1.0, sum_level = 22. Max member == not-gray max, so no halving.
        let mut data = XpKillData::default();
        data.count = 2;
        data.sum_level = 22;
        data.member_with_max_level = Some((guid(12), 12));
        data.not_gray_member_with_max_level = Some((guid(12), 12));

        // xp = calculate_creature_xp(10, 12, false):
        //   zero_diff(12) = 8, factor = (8 + 10 - 12) / 8 = 0.75
        //   base = (12*5 + 45) * 0.75 = 78.75 -> 79
        let xp = experience::calculate_creature_xp(10, 12, false);
        assert_eq!(xp, 79);

        // member 12: 1.0 * 12/22 = 0.54545 -> 43
        assert_eq!(member_xp(&data, 12, xp, 1.0), 43);
        // member 10: 1.0 * 10/22 = 0.45454 -> 35
        assert_eq!(member_xp(&data, 10, xp, 1.0), 35);
    }
}
