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
///
/// 1.12 sends one wire slot. 1.14 (`CMSG_LOOT_ITEM`) sends a list: `u32 count`, then per item a
/// guid128 loot object and a 1-based LootListID. Both act on the window the player has open.
pub async fn handle_loot_item(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> anyhow::Result<()> {
    let player_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Player not logged in"))?;

    let slots: Vec<u8> = match session.protocol() {
        Protocol::Vanilla => vec![packet
            .read_u8()
            .ok_or_else(|| anyhow::anyhow!("Failed to read slot"))?],
        Protocol::Modern => {
            let count = packet.read_u32().unwrap_or(0).min(u32::from(u8::MAX));
            let mut slots = Vec::with_capacity(count as usize);
            for _ in 0..count {
                let _loot_obj = packet.read_guid_for(session.protocol());
                let Some(list_id) = packet.read_u8() else {
                    break;
                };
                // LootListID 0 is reserved by the 1.14 client and never names an item.
                if let Some(slot) = list_id.checked_sub(1) {
                    slots.push(slot);
                }
            }
            slots
        }
    };

    // Get what the player is looting
    let target_guid = world
        .managers
        .player_mgr
        .get_looting_target(player_guid)
        .ok_or_else(|| anyhow::anyhow!("Not looting anything"))?;

    for slot in slots {
        world
            .systems
            .loot
            .handle_loot_item(player_guid, target_guid, slot, world)
            .await?;
    }
    Ok(())
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
/// The master looter assigns loot to a group member. Requires master loot, the sender to be the
/// master looter with that loot open and within reward distance, and the target to be an
/// allowed looter in the same group on the same map.
pub async fn handle_loot_master_give(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> anyhow::Result<()> {
    use crate::game::group::LootMethod;

    let master_guid = session
        .player_guid()
        .ok_or_else(|| anyhow::anyhow!("Player not logged in"))?;

    // Must be the group's master looter, under master loot.
    let group = world
        .systems
        .group
        .get_player_group(master_guid)
        .ok_or_else(|| anyhow::anyhow!("Not in a group"))?;
    if group.loot_method != LootMethod::MasterLooter || group.looter_guid != master_guid {
        return Ok(());
    }
    let group_id = group.id;

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
            let count = packet.read_u32().unwrap_or(0).min(u32::from(u8::MAX));
            target_guid = packet
                .read_guid_for(session.protocol())
                .ok_or_else(|| anyhow::anyhow!("Failed to read target guid"))?;
            // The 1.14 loot object is a derived GUID; the master's open window names the corpse.
            let open = world
                .managers
                .player_mgr
                .get_looting_target(master_guid)
                .unwrap_or_else(ObjectGuid::empty);
            for _ in 0..count {
                let _loot_obj = packet.read_guid_for(session.protocol());
                let wire_slot = packet.read_u8().unwrap_or(0);
                if let Some(slot) = wire_slot.checked_sub(1) {
                    requests.push((open, slot));
                }
            }
        }
    }

    // Target must exist, be in the same group, and on the same map.
    let Some((target_map, target_instance)) = world
        .managers
        .player_mgr
        .get_player(target_guid)
        .map(|t| (t.map_id, t.instance_id))
    else {
        return Ok(());
    };
    let same_group = world
        .systems
        .group
        .get_player_group_id(target_guid)
        .is_some_and(|g| g == group_id);
    let Some((master_map, master_instance)) = world
        .managers
        .player_mgr
        .get_player(master_guid)
        .map(|m| (m.map_id, m.instance_id))
    else {
        return Ok(());
    };
    if !same_group || target_map != master_map || target_instance != master_instance {
        return Ok(());
    }

    let open_loot = world.managers.player_mgr.get_looting_target(master_guid);

    for (loot_guid, slot) in requests {
        // Only from the window the master has open, within reach, to someone allowed to loot.
        if open_loot != Some(loot_guid) {
            continue;
        }
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
        if world
            .systems
            .loot_manager
            .is_allowed_looter(loot_guid, target_guid)
            == Some(false)
        {
            continue;
        }

        // Move the item to the target's bags; leave it in the loot on failure.
        let Some(item) = world
            .systems
            .loot_manager
            .get_loot(loot_guid)
            .and_then(|loot| {
                loot.items
                    .iter()
                    .find(|i| i.slot == slot && !i.is_looted)
                    .cloned()
            })
        else {
            continue;
        };
        let stored = world
            .systems
            .inventory
            .add_item(target_guid, item.item_id, item.count)
            .await;
        if let crate::game::inventory::types::AddItemResult::Success { .. } = stored {
            world.systems.loot_manager.with_loot_mut(loot_guid, |loot| {
                if let Some(it) = loot.items.iter_mut().find(|i| i.slot == slot) {
                    if !it.is_looted {
                        it.is_looted = true;
                        loot.unlooted_count = loot.unlooted_count.saturating_sub(1);
                    }
                }
            });
            world
                .systems
                .loot
                .notify_item_removed(loot_guid, false, slot, master_guid);
            world
                .systems
                .quest
                .handle_item_added(target_guid, item.item_id, item.count);
            crate::game::gameobject::quest_activation::refresh_quest_gameobjects(
                target_guid,
                world,
            );
            world.systems.loot.clear_if_empty(loot_guid, world);
        }
    }

    Ok(())
}
