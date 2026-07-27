use anyhow::{anyhow, Result};
use tracing::{info, warn};

use crate::core::common::packet::WorldPacketGuidExt;
use crate::game::inventory::types::EquipResult;
use crate::handlers::spells::parse_spell_cast_targets;
use crate::World;
use oxcore_shared::game::inventory::{
    is_bag_pos, is_bank_pos, is_equipment_pos, INVENTORY_SLOT_BAG_0,
};
use oxcore_shared::messages::SmsgReadItemOk;
use oxcore_shared::protocol::{ObjectGuid, WorldPacket};

const NPC_FLAG_BANKER: u32 = 0x0000_0100;
const BUY_BANK_SLOT_NOT_BANKER: u8 = 2;

fn normalize_buyback_slot(slot: u32) -> Result<u8> {
    let slot = u8::try_from(slot).map_err(|_| anyhow!("Invalid buyback slot"))?;
    // The client identifies visible buyback entries by their zero-based index,
    // while inventory stores them in absolute slots 69..80. Accept absolute slots
    // too so manually constructed packets remain valid.
    Ok(if slot < 12 { 69 + slot } else { slot })
}

fn can_use_bank(player_guid: ObjectGuid, banker_guid: Option<ObjectGuid>, world: &World) -> bool {
    let current_banker_guid = world
        .managers
        .player_mgr
        .get_player(player_guid)
        .and_then(|player| player.current_banker_guid);

    if let Some(banker_guid) = banker_guid {
        if current_banker_guid == Some(banker_guid) {
            return true;
        }

        return world
            .managers
            .creature_mgr
            .get_creature(banker_guid)
            .is_some_and(|creature| (creature.npc_flags & NPC_FLAG_BANKER) != 0);
    }

    current_banker_guid.is_some()
}

fn send_too_far_from_bank(session: &crate::core::session::WorldSession) -> Result<()> {
    session.send_msg(oxcore_shared::messages::SmsgInventoryChangeFailure::new(
        oxcore_shared::messages::EQUIP_ERR_TOO_FAR_AWAY_FROM_BANK,
    ))
}

pub async fn handle_use_item(
    session: &crate::core::session::WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    // Read packet data
    let bag = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read bag"))?;
    let slot = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read slot"))?;
    let spell_slot = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read spell slot"))?;

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    // CMSG_USE_ITEM carries the same target payload as CMSG_CAST_SPELL.
    let targets = parse_spell_cast_targets(packet, player_guid)?;

    // Get item GUID from inventory
    let item_guid = match world.systems.inventory.get_item_at(player_guid, bag, slot) {
        Some(guid) => guid,
        None => {
            warn!("CMSG_USE_ITEM: Item not found at bag={} slot={}", bag, slot);
            return Ok(());
        }
    };

    // Get item entry from cache
    let item_entry = world
        .systems
        .inventory
        .cache()
        .get_item(player_guid, item_guid)
        .map(|item| item.read().entry)
        .ok_or_else(|| anyhow!("Item not in cache"))?;

    // Get item template
    let template = world
        .systems
        .item_mgr
        .get_template(item_entry)
        .ok_or_else(|| anyhow!("Item template {} not found", item_entry))?;

    // Validate spell slot (0-4)
    if spell_slot >= 5 {
        warn!("Invalid spell slot {} for item {}", spell_slot, item_entry);
        return Ok(());
    }

    if template.start_quest != 0 {
        let Some(start_quest) = world
            .systems
            .quest
            .manager
            .get_quest_template(template.start_quest)
        else {
            warn!(
                "Item {} references missing start quest {}",
                item_entry, template.start_quest
            );
            return Ok(());
        };

        if world
            .systems
            .quest
            .can_take_quest(player_guid, &start_quest, world)
        {
            info!(
                "CMSG_USE_ITEM: player {:?} using item {} to start quest {}",
                player_guid, item_entry, template.start_quest
            );
            world.systems.quest.send_quest_details(
                player_guid,
                item_guid,
                template.start_quest,
                world,
            )?;
            return Ok(());
        }
    }

    // Validate the client-selected slot, then cast every on-use effect below.
    // This mirrors Player::CastItemUseSpell: the slot is an anti-cheat check,
    // not a request to suppress the item's other on-use spells.
    let spell_id = template.spell_id[spell_slot as usize];
    if spell_id == 0 {
        warn!("Item {} has no spell at slot {}", item_entry, spell_slot);
        return Ok(());
    }

    // Check spell trigger type (0 = On Use)
    let spell_trigger = template.spell_trigger[spell_slot as usize];
    if spell_trigger != 0 {
        warn!(
            "Item spell trigger {} not supported (only On Use=0)",
            spell_trigger
        );
        return Ok(());
    }

    info!(
        "CMSG_USE_ITEM: player {:?} using item {} (spell {}) from bag={} slot={}",
        player_guid, item_entry, spell_id, bag, slot
    );

    let mut on_use_count = 0;
    for index in 0..5 {
        let spell_id = template.spell_id[index];
        if spell_id == 0 || template.spell_trigger[index] != 0 {
            continue;
        }

        // The first spell pays the item cost; additional effects are triggered.
        world
            .systems
            .spells
            .cast_spell_from_item(
                player_guid,
                spell_id,
                targets.clone(),
                item_guid,
                on_use_count > 0,
                world,
            )
            .await?;
        on_use_count += 1;
    }

    Ok(())
}

pub async fn handle_open_item(
    session: &crate::core::session::WorldSession,
    packet: &mut WorldPacket,
    _world: &World,
) -> Result<()> {
    let _bag = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read bag"))?;
    let _slot = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read slot"))?;

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    warn!(
        "CMSG_OPEN_ITEM received but not fully implemented for {:?}",
        player_guid
    );

    Ok(())
}

pub async fn handle_read_item(
    session: &crate::core::session::WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let bag = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read bag"))?;
    let slot = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read slot"))?;

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    let Some(item_guid) = world.systems.inventory.get_item_at(player_guid, bag, slot) else {
        session.send_msg(oxcore_shared::messages::SmsgInventoryChangeFailure::new(
            oxcore_shared::messages::EQUIP_ERR_ITEM_NOT_FOUND,
        ))?;
        return Ok(());
    };

    let Some(item_entry) = world
        .systems
        .inventory
        .cache()
        .get_item(player_guid, item_guid)
        .map(|item| item.read().entry)
    else {
        session.send_msg(oxcore_shared::messages::SmsgInventoryChangeFailure::new(
            oxcore_shared::messages::EQUIP_ERR_ITEM_NOT_FOUND,
        ))?;
        return Ok(());
    };

    if world.systems.item_mgr.get_template(item_entry).is_none() {
        session.send_msg(oxcore_shared::messages::SmsgInventoryChangeFailure::new(
            oxcore_shared::messages::EQUIP_ERR_ITEM_NOT_FOUND,
        ))?;
        return Ok(());
    }

    session.send_msg(SmsgReadItemOk { item_guid })?;

    Ok(())
}

pub async fn handle_swap_item(
    session: &crate::core::session::WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let dst_bag = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read dst bag"))?;
    let dst_slot = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read dst slot"))?;
    let src_bag = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read src bag"))?;
    let src_slot = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read src slot"))?;

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    // Check if source and destination are the same (client sometimes sends this)
    if src_bag == dst_bag && src_slot == dst_slot {
        tracing::debug!("[CMSG_SWAP_ITEM] Ignoring swap of same slot");
        return Ok(());
    }

    if (is_bank_pos(src_bag, src_slot) || is_bank_pos(dst_bag, dst_slot))
        && !can_use_bank(player_guid, None, world)
    {
        send_too_far_from_bank(session)?;
        tracing::warn!(
            "[CMSG_SWAP_ITEM] Bank slot swap rejected without active banker: src={}:{} dst={}:{}",
            src_bag,
            src_slot,
            dst_bag,
            dst_slot
        );
        return Ok(());
    }

    let result =
        world
            .systems
            .inventory
            .move_item(player_guid, src_bag, src_slot, dst_bag, dst_slot);

    match result {
        crate::game::inventory::types::MoveItemResult::Moved => {
            tracing::debug!("[CMSG_SWAP_ITEM] Item moved successfully");
        }
        crate::game::inventory::types::MoveItemResult::Swapped => {
            tracing::debug!("[CMSG_SWAP_ITEM] Items swapped successfully");
        }
        crate::game::inventory::types::MoveItemResult::Merged { source_removed } => {
            tracing::debug!(
                "[CMSG_SWAP_ITEM] Items merged, source_removed={}",
                source_removed
            );
        }
        crate::game::inventory::types::MoveItemResult::InvalidSource => {
            tracing::warn!("[CMSG_SWAP_ITEM] Invalid source slot");
            // Error packet already sent by inventory system
        }
        crate::game::inventory::types::MoveItemResult::InvalidDestination => {
            tracing::warn!("[CMSG_SWAP_ITEM] Invalid destination slot");
            // Error packet already sent by inventory system
        }
        crate::game::inventory::types::MoveItemResult::PlayerNotLoaded => {
            tracing::error!("[CMSG_SWAP_ITEM] Player not loaded");
        }
        crate::game::inventory::types::MoveItemResult::DatabaseError(e) => {
            tracing::error!("[CMSG_SWAP_ITEM] Database error: {}", e);
        }
        other => {
            tracing::warn!("[CMSG_SWAP_ITEM] Unexpected result: {:?}", other);
        }
    }

    Ok(())
}

pub async fn handle_swap_inv_item(
    session: &crate::core::session::WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let src_slot = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read src slot"))?;
    let dst_slot = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read dst slot"))?;

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    // Check if source and destination are the same
    if src_slot == dst_slot {
        tracing::debug!("[CMSG_SWAP_INV_ITEM] Ignoring swap of same slot");
        return Ok(());
    }

    if (is_bank_pos(INVENTORY_SLOT_BAG_0, src_slot) || is_bank_pos(INVENTORY_SLOT_BAG_0, dst_slot))
        && !can_use_bank(player_guid, None, world)
    {
        send_too_far_from_bank(session)?;
        tracing::warn!(
            "[CMSG_SWAP_INV_ITEM] Bank slot swap rejected without active banker: src={} dst={}",
            src_slot,
            dst_slot
        );
        return Ok(());
    }

    let result = world.systems.inventory.move_item(
        player_guid,
        INVENTORY_SLOT_BAG_0,
        src_slot,
        INVENTORY_SLOT_BAG_0,
        dst_slot,
    );

    match result {
        crate::game::inventory::types::MoveItemResult::Moved => {
            tracing::debug!("[CMSG_SWAP_INV_ITEM] Item moved successfully");
        }
        crate::game::inventory::types::MoveItemResult::Swapped => {
            tracing::debug!("[CMSG_SWAP_INV_ITEM] Items swapped successfully");
        }
        crate::game::inventory::types::MoveItemResult::Merged { source_removed } => {
            tracing::debug!(
                "[CMSG_SWAP_INV_ITEM] Items merged, source_removed={}",
                source_removed
            );
        }
        crate::game::inventory::types::MoveItemResult::InvalidSource => {
            tracing::warn!("[CMSG_SWAP_INV_ITEM] Invalid source slot");
        }
        crate::game::inventory::types::MoveItemResult::InvalidDestination => {
            tracing::warn!("[CMSG_SWAP_INV_ITEM] Invalid destination slot");
        }
        crate::game::inventory::types::MoveItemResult::PlayerNotLoaded => {
            tracing::error!("[CMSG_SWAP_INV_ITEM] Player not loaded");
        }
        crate::game::inventory::types::MoveItemResult::DatabaseError(e) => {
            tracing::error!("[CMSG_SWAP_INV_ITEM] Database error: {}", e);
        }
        other => {
            tracing::warn!("[CMSG_SWAP_INV_ITEM] Unexpected result: {:?}", other);
        }
    }

    Ok(())
}

pub async fn handle_split_item(
    session: &crate::core::session::WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let src_bag = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read src bag"))?;
    let src_slot = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read src slot"))?;
    let dst_bag = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read dst bag"))?;
    let dst_slot = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read dst slot"))?;
    let count = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read count"))?;

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    // Check if source and destination are the same
    if src_bag == dst_bag && src_slot == dst_slot {
        tracing::debug!("[CMSG_SPLIT_ITEM] Ignoring split to same slot");
        return Ok(());
    }

    // Validate count
    if count == 0 {
        tracing::warn!("[CMSG_SPLIT_ITEM] Invalid count: 0");
        return Ok(());
    }

    let result = world.systems.inventory.split_item(
        player_guid,
        src_bag,
        src_slot,
        dst_bag,
        dst_slot,
        count as u32,
    );

    match result.await {
        crate::game::inventory::types::SplitItemResult::Success {
            source_guid,
            new_item_guid,
        } => {
            tracing::debug!(
                "[CMSG_SPLIT_ITEM] Item split successfully: {:?} -> {:?}",
                source_guid,
                new_item_guid
            );
        }
        crate::game::inventory::types::SplitItemResult::MergedToExisting {
            source_guid,
            dest_guid,
        } => {
            tracing::debug!(
                "[CMSG_SPLIT_ITEM] Items merged: {:?} into {:?}",
                source_guid,
                dest_guid
            );
        }
        crate::game::inventory::types::SplitItemResult::InvalidCount => {
            tracing::warn!("[CMSG_SPLIT_ITEM] Invalid count");
            // Error packet already sent by inventory system
        }
        crate::game::inventory::types::SplitItemResult::SourceNotFound => {
            tracing::warn!("[CMSG_SPLIT_ITEM] Source item not found");
            // Error packet already sent by inventory system
        }
        crate::game::inventory::types::SplitItemResult::DestinationOccupied => {
            tracing::warn!("[CMSG_SPLIT_ITEM] Destination occupied or cannot stack");
            // Error packet already sent by inventory system
        }
        crate::game::inventory::types::SplitItemResult::PlayerNotLoaded => {
            tracing::error!("[CMSG_SPLIT_ITEM] Player not loaded");
        }
        crate::game::inventory::types::SplitItemResult::DatabaseError(e) => {
            tracing::error!("[CMSG_SPLIT_ITEM] Database error: {}", e);
        }
        other => {
            tracing::warn!("[CMSG_SPLIT_ITEM] Unexpected result: {:?}", other);
        }
    }

    Ok(())
}

pub async fn handle_autoequip_item_slot(
    session: &crate::core::session::WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let item_guid = packet
        .read_packed_guid_raw()
        .ok_or_else(|| anyhow!("Failed to read item guid"))?;
    let item_guid = ObjectGuid::from(item_guid);
    let equip_slot = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read equip slot"))?;

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    if !is_equipment_pos(INVENTORY_SLOT_BAG_0, equip_slot) {
        return Ok(());
    }

    let (Some(src_bag), Some(src_slot)) =
        find_item_location(player_guid, item_guid, &world.systems.inventory)
    else {
        return Ok(());
    };

    if src_bag == INVENTORY_SLOT_BAG_0 && src_slot == equip_slot {
        return Ok(());
    }

    match world.systems.inventory.move_item(
        player_guid,
        src_bag,
        src_slot,
        INVENTORY_SLOT_BAG_0,
        equip_slot,
    ) {
        crate::game::inventory::types::MoveItemResult::Moved => {
            tracing::debug!("[CMSG_AUTOEQUIP_ITEM_SLOT] Item moved to equipment slot");
        }
        crate::game::inventory::types::MoveItemResult::Swapped => {
            tracing::debug!("[CMSG_AUTOEQUIP_ITEM_SLOT] Item swapped into equipment slot");
        }
        crate::game::inventory::types::MoveItemResult::Merged { source_removed } => {
            tracing::debug!(
                "[CMSG_AUTOEQUIP_ITEM_SLOT] Items merged, source_removed={}",
                source_removed
            );
        }
        crate::game::inventory::types::MoveItemResult::InvalidSource => {
            tracing::warn!("[CMSG_AUTOEQUIP_ITEM_SLOT] Invalid source slot");
        }
        crate::game::inventory::types::MoveItemResult::InvalidDestination => {
            tracing::warn!("[CMSG_AUTOEQUIP_ITEM_SLOT] Invalid destination slot");
        }
        crate::game::inventory::types::MoveItemResult::PlayerNotLoaded => {
            tracing::error!("[CMSG_AUTOEQUIP_ITEM_SLOT] Player not loaded");
        }
        crate::game::inventory::types::MoveItemResult::DatabaseError(e) => {
            tracing::error!("[CMSG_AUTOEQUIP_ITEM_SLOT] Database error: {}", e);
        }
        other => {
            tracing::warn!("[CMSG_AUTOEQUIP_ITEM_SLOT] Unexpected result: {:?}", other);
        }
    }

    Ok(())
}

pub async fn handle_autoequip_item(
    session: &crate::core::session::WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let src_bag = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read src bag"))?;
    let src_slot = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read src slot"))?;

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    let src_item_guid = match world
        .systems
        .inventory
        .get_item_at(player_guid, src_bag, src_slot)
    {
        Some(guid) => guid,
        None => {
            tracing::warn!(
                "[CMSG_AUTOEQUIP_ITEM] Item not found at bag={} slot={}",
                src_bag,
                src_slot
            );
            return Ok(());
        }
    };

    let src_item = match world
        .systems
        .inventory
        .cache()
        .get_item(player_guid, src_item_guid)
    {
        Some(item) => item,
        None => {
            tracing::warn!(
                "[CMSG_AUTOEQUIP_ITEM] Item object not found: {:?}",
                src_item_guid
            );
            return Ok(());
        }
    };

    let entry_id = {
        let item = src_item.read();
        item.entry
    };

    let template = match world.systems.item_mgr.get_template(entry_id) {
        Some(t) => t,
        None => {
            tracing::warn!(
                "[CMSG_AUTOEQUIP_ITEM] Template not found for item {}",
                entry_id
            );
            return Ok(());
        }
    };

    let player_class = world
        .managers
        .player_mgr
        .get_player(player_guid)
        .map(|player| player.class)
        .unwrap_or(1);
    let allowed_slots = template.get_allowed_equip_slots(player_class, true);
    let equip_slot = match allowed_slots
        .iter()
        .copied()
        .filter(|slot| *slot != 255)
        .find(|slot| {
            world
                .systems
                .inventory
                .get_item_at(player_guid, INVENTORY_SLOT_BAG_0, *slot)
                .is_none()
        })
        .or_else(|| allowed_slots.iter().copied().find(|slot| *slot != 255))
    {
        Some(slot) => slot,
        None => {
            session.send_msg(oxcore_shared::messages::SmsgInventoryChangeFailure::new(
                oxcore_shared::messages::EQUIP_ERR_ITEM_CANT_BE_EQUIPPED,
            ))?;
            return Ok(());
        }
    };

    if src_bag == INVENTORY_SLOT_BAG_0 && src_slot == equip_slot {
        return Ok(());
    }

    tracing::info!(
        "[CMSG_AUTOEQUIP_ITEM] Item entry={} name='{}' inventory_type={} -> equip_slot={}",
        entry_id,
        template.name,
        template.inventory_type,
        equip_slot
    );

    let result = world.systems.inventory.move_item(
        player_guid,
        src_bag,
        src_slot,
        INVENTORY_SLOT_BAG_0,
        equip_slot,
    );
    let equipped = matches!(
        result,
        crate::game::inventory::types::MoveItemResult::Moved
            | crate::game::inventory::types::MoveItemResult::Swapped
    );

    match result {
        crate::game::inventory::types::MoveItemResult::Moved => {
            tracing::debug!("[CMSG_AUTOEQUIP_ITEM] Item moved to slot {}", equip_slot);
        }
        crate::game::inventory::types::MoveItemResult::Swapped => {
            tracing::debug!("[CMSG_AUTOEQUIP_ITEM] Item swapped to slot {}", equip_slot);
        }
        crate::game::inventory::types::MoveItemResult::Merged { source_removed } => {
            tracing::debug!(
                "[CMSG_AUTOEQUIP_ITEM] Items merged, source_removed={}",
                source_removed
            );
        }
        crate::game::inventory::types::MoveItemResult::InvalidSource => {
            tracing::warn!("[CMSG_AUTOEQUIP_ITEM] Invalid source slot");
        }
        crate::game::inventory::types::MoveItemResult::InvalidDestination => {
            tracing::warn!("[CMSG_AUTOEQUIP_ITEM] Invalid destination slot");
        }
        crate::game::inventory::types::MoveItemResult::PlayerNotLoaded => {
            tracing::error!("[CMSG_AUTOEQUIP_ITEM] Player not loaded");
        }
        crate::game::inventory::types::MoveItemResult::DatabaseError(e) => {
            tracing::error!("[CMSG_AUTOEQUIP_ITEM] Database error: {}", e);
        }
        other => {
            tracing::warn!("[CMSG_AUTOEQUIP_ITEM] Move failed: {:?}", other);
        }
    }

    if equipped && is_bag_pos(INVENTORY_SLOT_BAG_0, equip_slot) {
        session.send_msg(oxcore_shared::messages::SmsgOpenContainer {
            item_guid: src_item_guid,
        })?;
    }

    Ok(())
}

pub async fn handle_autoequip_ground_item(
    session: &crate::core::session::WorldSession,
    packet: &mut WorldPacket,
    _world: &World,
) -> Result<()> {
    let _item_guid = packet
        .read_packed_guid_raw()
        .ok_or_else(|| anyhow!("Failed to read item guid"))?;

    let _player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    warn!("CMSG_AUTOEQUIP_GROUND_ITEM received but not implemented");

    Ok(())
}

pub async fn handle_autostore_ground_item(
    session: &crate::core::session::WorldSession,
    packet: &mut WorldPacket,
    _world: &World,
) -> Result<()> {
    let _item_guid = packet
        .read_packed_guid_raw()
        .ok_or_else(|| anyhow!("Failed to read item guid"))?;

    let _player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    warn!("CMSG_AUTOSTORE_GROUND_ITEM received but not implemented");

    Ok(())
}

pub async fn handle_autostore_bag_item(
    session: &crate::core::session::WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let src_bag = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read src bag"))?;
    let src_slot = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read src slot"))?;
    let dst_bag = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read dst bag"))?;

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    let _ = world
        .systems
        .inventory
        .move_item(player_guid, src_bag, src_slot, dst_bag, 0);

    Ok(())
}

pub async fn handle_drop_item(
    session: &crate::core::session::WorldSession,
    packet: &mut WorldPacket,
    _world: &World,
) -> Result<()> {
    let _bag = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read bag"))?;
    let _slot = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read slot"))?;

    let _player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    warn!("CMSG_DROP_ITEM received but not implemented");

    Ok(())
}

pub async fn handle_destroy_item(
    session: &crate::core::session::WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let bag = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read bag"))?;
    let slot = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read slot"))?;
    let count = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read count"))?;

    let _ = packet.read_u8();
    let _ = packet.read_u8();
    let _ = packet.read_u8();

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    if let Some(item_guid) = world.systems.inventory.get_item_at(player_guid, bag, slot) {
        let destroy_count = if count > 0 {
            count as u32
        } else {
            world
                .systems
                .inventory
                .cache()
                .get_item(player_guid, item_guid)
                .map(|item| item.read().count)
                .unwrap_or(0)
        };

        warn!(
            "CMSG_DESTROYITEM: player={:?} bag={} slot={} count={} item={:?} — client is destroying this item",
            player_guid, bag, slot, destroy_count, item_guid
        );

        if destroy_count > 0 {
            let _ = world
                .systems
                .inventory
                .remove_item(player_guid, item_guid, destroy_count);
        }
    } else {
        session.send_msg(oxcore_shared::messages::SmsgInventoryChangeFailure::new(
            oxcore_shared::messages::EQUIP_ERR_ITEM_NOT_FOUND,
        ))?;
    }

    Ok(())
}

pub async fn handle_set_ammo(
    session: &crate::core::session::WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    const ITEM_CLASS_PROJECTILE: u32 = 6;

    let item_entry = packet
        .read_u32()
        .ok_or_else(|| anyhow!("Failed to read item entry"))?;

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    if item_entry != 0 {
        let Some(template) = world.managers.item_mgr.get_template(item_entry) else {
            warn!("CMSG_SET_AMMO: unknown ammo entry {}", item_entry);
            return Ok(());
        };

        if template.item_class != ITEM_CLASS_PROJECTILE || template.ammo_type == 0 {
            warn!("CMSG_SET_AMMO: entry {} is not projectile ammo", item_entry);
            return Ok(());
        }
    }

    world
        .systems
        .player
        .manager()
        .with_player_mut(player_guid, |player| {
            player.ammo_id = item_entry;
        });

    world.systems.stats.recalculate_all(player_guid);
    world.systems.stats.send_stat_update(player_guid);

    Ok(())
}

pub async fn handle_autobank_item(
    session: &crate::core::session::WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let bag = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read bag"))?;
    let slot = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read slot"))?;

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    if !can_use_bank(player_guid, None, world) {
        send_too_far_from_bank(session)?;
        tracing::warn!(
            "[CMSG_AUTOBANK_ITEM] Bank move rejected without active banker: bag={} slot={}",
            bag,
            slot
        );
        return Ok(());
    }

    let result = world
        .systems
        .inventory
        .auto_bank_item(player_guid, bag, slot);
    tracing::debug!(
        "CMSG_AUTOBANK_ITEM: player={:?} bag={} slot={} result={:?}",
        player_guid,
        bag,
        slot,
        result
    );

    Ok(())
}

pub async fn handle_autostore_bank_item(
    session: &crate::core::session::WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let bag = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read bag"))?;
    let slot = packet
        .read_u8()
        .ok_or_else(|| anyhow!("Failed to read slot"))?;

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    if !can_use_bank(player_guid, None, world) {
        send_too_far_from_bank(session)?;
        tracing::warn!(
            "[CMSG_AUTOSTORE_BANK_ITEM] Bank move rejected without active banker: bag={} slot={}",
            bag,
            slot
        );
        return Ok(());
    }

    let result = world
        .systems
        .inventory
        .auto_store_bank_item(player_guid, bag, slot);
    tracing::debug!(
        "CMSG_AUTOSTORE_BANK_ITEM: player={:?} bag={} slot={} result={:?}",
        player_guid,
        bag,
        slot,
        result
    );

    Ok(())
}

pub async fn handle_buy_bank_slot(
    session: &crate::core::session::WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let banker_guid = packet
        .read_packed_guid_raw()
        .ok_or_else(|| anyhow!("Failed to read banker guid"))?;
    let banker_guid = ObjectGuid::from(banker_guid);

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    if !can_use_bank(player_guid, Some(banker_guid), world) {
        tracing::warn!(
            "[CMSG_BUY_BANK_SLOT] rejected non-banker: player={:?} banker={:?}",
            player_guid,
            banker_guid
        );
        world
            .systems
            .inventory
            .send_buy_bank_slot_result(player_guid, BUY_BANK_SLOT_NOT_BANKER);
        return Ok(());
    }

    world
        .systems
        .inventory
        .send_buy_bank_slot_result(player_guid, 0);

    Ok(())
}

pub async fn handle_buyback_item(
    session: &crate::core::session::WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let vendor_guid = packet
        .read_guid()
        .ok_or_else(|| anyhow!("Failed to read vendor guid"))?;
    let slot = packet
        .read_u32()
        .ok_or_else(|| anyhow!("Failed to read slot"))?;

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    let slot = normalize_buyback_slot(slot)?;
    world
        .systems
        .vendor
        .handle_buyback_item(player_guid, vendor_guid, slot)
        .await?;

    Ok(())
}

fn find_item_location(
    player_guid: ObjectGuid,
    item_guid: ObjectGuid,
    inventory: &crate::game::inventory::InventorySystem,
) -> (Option<u8>, Option<u8>) {
    if let Some(item) = inventory.cache().get_item(player_guid, item_guid) {
        let item = item.read();
        return (Some(item.bag), Some(item.slot));
    }

    (None, None)
}

#[cfg(test)]
mod tests {
    use super::normalize_buyback_slot;

    #[test]
    fn buyback_client_indices_map_to_inventory_slots() {
        assert_eq!(normalize_buyback_slot(0).unwrap(), 69);
        assert_eq!(normalize_buyback_slot(11).unwrap(), 80);
    }

    #[test]
    fn buyback_absolute_slots_are_preserved() {
        assert_eq!(normalize_buyback_slot(69).unwrap(), 69);
        assert_eq!(normalize_buyback_slot(80).unwrap(), 80);
    }

    #[test]
    fn buyback_slot_rejects_values_outside_packet_range() {
        assert!(normalize_buyback_slot(256).is_err());
    }
}
