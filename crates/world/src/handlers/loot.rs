use crate::core::common::packet::WorldPacketGuidExt;
use crate::core::session::WorldSession;
use crate::World;
use oxcore_shared::protocol::ObjectGuid;
use oxcore_shared::protocol::Protocol;
use oxcore_shared::protocol::WorldPacket;

/// Handle CMSG_LOOT (0x015D)
pub async fn handle_loot(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> anyhow::Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Player not logged in"))?;
    let target_guid = packet
        .read_guid_for(session.protocol())
        .ok_or_else(|| anyhow::anyhow!("Failed to read target GUID"))?;

    // Delegate everything to LootSystem
    world
        .systems
        .loot
        .handle_loot_request(player_guid, target_guid, world)
        .await
}

/// Handle CMSG_LOOT_MONEY (0x015E)
pub async fn handle_loot_money(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    world: &World,
) -> anyhow::Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Player not logged in"))?;

    // Get what the player is looting
    let target_guid = world
        .managers
        .player_mgr
        .get_looting_target(player_guid)
        .ok_or_else(|| anyhow::anyhow!("Not looting anything"))?;

    // Delegate to LootSystem
    world
        .systems
        .loot
        .handle_loot_money(player_guid, target_guid, world)
        .await
}

/// Handle CMSG_AUTOSTORE_LOOT_ITEM (0x0108)
pub async fn handle_loot_item(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> anyhow::Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Player not logged in"))?;
    let wire_slot = packet
        .read_u8()
        .ok_or_else(|| anyhow::anyhow!("Failed to read slot"))?;

    // The modern client sends a 1-based LootListID (see SmsgLootResponse::to_modern); our
    // internal loot slots are 0-based. Vanilla's wire slot is genuinely 0-based already.
    let slot = match session.protocol() {
        Protocol::Modern => wire_slot
            .checked_sub(1)
            .ok_or_else(|| anyhow::anyhow!("Invalid loot slot 0 from modern client"))?,
        Protocol::Vanilla => wire_slot,
    };

    // Get what the player is looting
    let target_guid = world
        .managers
        .player_mgr
        .get_looting_target(player_guid)
        .ok_or_else(|| anyhow::anyhow!("Not looting anything"))?;

    // Delegate to LootSystem
    world
        .systems
        .loot
        .handle_loot_item(player_guid, target_guid, slot, world)
        .await
}

/// Handle CMSG_LOOT_RELEASE (0x015F)
pub async fn handle_loot_release(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> anyhow::Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Player not logged in"))?;
    let target_guid = packet
        .read_guid_for(session.protocol())
        .ok_or_else(|| anyhow::anyhow!("Failed to read target GUID"))?;

    // Delegate to LootSystem
    world
        .systems
        .loot
        .handle_loot_release(player_guid, target_guid, world)
        .await
}

/// Handle CMSG_LOOT_ROLL (0x02A0)
pub async fn handle_loot_roll(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> anyhow::Result<()> {
    use crate::game::loot::RollVote;

    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Player not logged in"))?;

    // Vanilla: u64 lootGuid, u32 itemSlot, u8 rollType.
    // Modern: packed guid128 LootObj, u8 LootListID (1-based), u8 RollType.
    let (loot_guid, wire_slot, roll_type) = match session.protocol() {
        Protocol::Vanilla => (
            packet.read_u64().unwrap_or(0),
            packet.read_u32().unwrap_or(0),
            packet.read_u8().unwrap_or(0),
        ),
        Protocol::Modern => {
            let guid = packet
                .read_guid_for(session.protocol())
                .ok_or_else(|| anyhow::anyhow!("Failed to read loot guid"))?;
            let wire = packet.read_u8().unwrap_or(0);
            let rt = packet.read_u8().unwrap_or(0);
            (guid.raw(), u32::from(wire.saturating_sub(1)), rt)
        }
    };

    let loot_guid = ObjectGuid::from_raw(loot_guid);

    // Only pass/need/greed are valid votes.
    let vote = match roll_type {
        0 => RollVote::Pass,
        1 => RollVote::Need,
        2 => RollVote::Greed,
        _ => return Ok(()),
    };

    if let Some(group_id) = world.systems.group.get_player_group_id(player_guid) {
        world
            .systems
            .group
            .count_roll_vote(
                world,
                group_id,
                player_guid,
                loot_guid,
                wire_slot as u8,
                vote,
            )
            .await;
    }

    Ok(())
}

/// Handle CMSG_LOOT_MASTER_GIVE (0x02A3)
///
/// The master looter assigns loot to a raid member. Validates method, looter identity,
/// same group/map, and reward distance before moving the item (LootHandler.cpp:619-748).
pub async fn handle_loot_master_give(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> anyhow::Result<()> {
    use crate::game::group::LootMethod;
    use crate::game::group::rolls::LootContext;

    let master_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Player not logged in"))?;

    // Must be the group's master looter, under master-loot, and the group's looter.
    let group = world
        .systems
        .group
        .get_player_group(master_guid)
        .ok_or_else(|| anyhow::anyhow!("Not in a group"))?;
    if group.loot_method != LootMethod::MasterLooter {
        return Ok(());
    }
    let Some(group_id) = world.systems.group.get_player_group_id(master_guid) else {
        return Ok(());
    };

    // Modern: u32 count, guid128 target, then per-request { guid128 loot, u8 slot(1-based) }.
    // Vanilla: u64 loot guid, u8 slot, u64 target guid.
    let mut target_guid = ObjectGuid::empty();
    let mut requests: Vec<(ObjectGuid, u8)> = Vec::new();

    match session.protocol() {
        Protocol::Vanilla => {
            let loot_guid = packet.read_u64().unwrap_or(0);
            let slot = packet.read_u8().unwrap_or(0);
            let target = packet.read_u64().unwrap_or(0);
            target_guid = ObjectGuid::from_raw(target);
            requests.push((ObjectGuid::from_raw(loot_guid), slot));
        }
        Protocol::Modern => {
            let count = packet.read_u32().unwrap_or(0);
            target_guid = packet
                .read_guid_for(session.protocol())
                .ok_or_else(|| anyhow::anyhow!("Failed to read target guid"))?;
            for _ in 0..count {
                let loot = packet
                    .read_guid_for(session.protocol())
                    .ok_or_else(|| anyhow::anyhow!("Failed to read loot guid"))?;
                let wire_slot = packet.read_u8().unwrap_or(0);
                requests.push((loot, wire_slot.saturating_sub(1)));
            }
        }
    }

    // Target must exist, be in the same group, on the same map, and within reward distance.
    let Some(target) = world.managers.player_mgr.get_player(target_guid) else {
        return Ok(());
    };
    let target_in_group = world.systems.group.is_in_group(target_guid);
    let same_group = world
        .systems
        .group
        .get_player_group_id(target_guid)
        .map(|g| g == group_id)
        .unwrap_or(false);
    let Some(master) = world.managers.player_mgr.get_player(master_guid) else {
        return Ok(());
    };
    if !target_in_group
        || !same_group
        || target.map_id != master.map_id
        || target.instance_id != master.instance_id
    {
        return Ok(());
    }

    for (loot_guid, slot) in requests {
        let Some(context) = crate::game::group::GroupSystem::loot_context_for(world, loot_guid)
        else {
            continue;
        };
        if !crate::game::group::is_at_group_reward_distance(
            world,
            master_guid,
            &context.position,
            context.map_id,
            context.instance_id,
            context.rank,
        ) {
            continue;
        }

        // Move the item to the target's bags; leave it in the loot on failure.
        let Some(item) = world
            .systems
            .loot_manager
            .get_loot(loot_guid)
            .and_then(|loot| loot.items.iter().find(|i| i.slot == slot && !i.is_looted).cloned())
        else {
            continue;
        };
        let stored = world
            .systems
            .inventory
            .add_item(target_guid, item.item_id, item.count)
            .await;
        if let crate::game::inventory::types::AddItemResult::Success { .. } = stored {
            world
                .systems
                .loot_manager
                .with_loot_mut(loot_guid, |loot| {
                    if let Some(it) = loot.items.iter_mut().find(|i| i.slot == slot) {
                        it.is_looted = true;
                        loot.unlooted_count = loot.unlooted_count.saturating_sub(1);
                    }
                });
            use oxcore_shared::messages::loot::SmsgLootRemoved;
            world
                .managers
                .broadcast_mgr
                .send_msg_to_player(target_guid, SmsgLootRemoved { slot });
        }
    }

    Ok(())
}
