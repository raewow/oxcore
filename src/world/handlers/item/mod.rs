use anyhow::{anyhow, Result};
use tracing::{info, warn};

use crate::shared::messages::SmsgReadItemFailed;
use crate::shared::messages::SmsgReadItemOk;
use crate::shared::protocol::{ObjectGuid, WorldPacket};
use crate::world::core::common::packet::WorldPacketGuidExt;
use crate::world::game::inventory::types::EquipResult;
use crate::world::World;

pub async fn handle_use_item(
    session: &crate::world::core::session::WorldSession,
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

    // Get spell ID from template
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

    // Cast the spell from the item. Passes item_guid so SMSG_SPELL_START and
    // SMSG_SPELL_GO write the item GUID as the first packed GUID (per MaNGOS
    // protocol), preventing the client from sending CMSG_DESTROYITEM.
    world
        .systems
        .spells
        .cast_spell_from_item(
            player_guid,
            spell_id,
            Some(player_guid), // Self-target
            item_guid,
            world,
        )
        .await?;

    // Handle spell charges per MaNGOS TakeCastItem logic:
    // - spell_charges < 0: expendable (item destroyed when charges hit 0)
    // - spell_charges > 0: tracked charges, item NOT destroyed
    // - spell_charges == 0: no charge tracking (hearthstone, most items)
    let template_charges = template.spell_charges[spell_slot as usize];
    info!(
        "CMSG_USE_ITEM: item {} spell_slot={} template_charges={} — charge path {}",
        item_entry,
        spell_slot,
        template_charges,
        if template_charges != 0 {
            "ACTIVE"
        } else {
            "skipped (0)"
        }
    );
    if template_charges != 0 {
        use crate::world::game::inventory::types::ChargeResult;
        let result = world
            .systems
            .inventory
            .consume_charge(player_guid, item_guid, spell_slot)
            .await;
        match result {
            ChargeResult::Success { remaining } if remaining == 0 && template_charges < 0 => {
                // Expendable item (negative charges) exhausted — destroy it
                let _ = world
                    .systems
                    .inventory
                    .remove_item(player_guid, item_guid, 1);
            }
            ChargeResult::Success { .. } => {}
            ChargeResult::NoCharges => {
                warn!("Item {} has no charges left but was used", item_entry);
            }
            other => {
                warn!("consume_charge failed for item {}: {:?}", item_entry, other);
            }
        }
    }

    Ok(())
}

pub async fn handle_open_item(
    session: &crate::world::core::session::WorldSession,
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
    session: &crate::world::core::session::WorldSession,
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

    if let Some(item_guid) = world.systems.inventory.get_item_at(player_guid, bag, slot) {
        session.send_msg(SmsgReadItemOk { item_guid })?;
    } else {
        let _ = packet.read_u8();
        let _ = packet.read_u8();
        session.send_msg(SmsgReadItemFailed {
            item_guid: ObjectGuid::default(),
        })?;
    }

    Ok(())
}

pub async fn handle_swap_item(
    session: &crate::world::core::session::WorldSession,
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

    let result =
        world
            .systems
            .inventory
            .move_item(player_guid, src_bag, src_slot, dst_bag, dst_slot);

    match result {
        crate::world::game::inventory::types::MoveItemResult::Moved => {
            tracing::debug!("[CMSG_SWAP_ITEM] Item moved successfully");
        }
        crate::world::game::inventory::types::MoveItemResult::Swapped => {
            tracing::debug!("[CMSG_SWAP_ITEM] Items swapped successfully");
        }
        crate::world::game::inventory::types::MoveItemResult::Merged { source_removed } => {
            tracing::debug!(
                "[CMSG_SWAP_ITEM] Items merged, source_removed={}",
                source_removed
            );
        }
        crate::world::game::inventory::types::MoveItemResult::InvalidSource => {
            tracing::warn!("[CMSG_SWAP_ITEM] Invalid source slot");
            // Error packet already sent by inventory system
        }
        crate::world::game::inventory::types::MoveItemResult::InvalidDestination => {
            tracing::warn!("[CMSG_SWAP_ITEM] Invalid destination slot");
            // Error packet already sent by inventory system
        }
        crate::world::game::inventory::types::MoveItemResult::PlayerNotLoaded => {
            tracing::error!("[CMSG_SWAP_ITEM] Player not loaded");
        }
        crate::world::game::inventory::types::MoveItemResult::DatabaseError(e) => {
            tracing::error!("[CMSG_SWAP_ITEM] Database error: {}", e);
        }
        other => {
            tracing::warn!("[CMSG_SWAP_ITEM] Unexpected result: {:?}", other);
        }
    }

    Ok(())
}

pub async fn handle_swap_inv_item(
    session: &crate::world::core::session::WorldSession,
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

    const INVENTORY_SLOT_BAG_0: u8 = 255;

    // Check if source and destination are the same
    if src_slot == dst_slot {
        tracing::debug!("[CMSG_SWAP_INV_ITEM] Ignoring swap of same slot");
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
        crate::world::game::inventory::types::MoveItemResult::Moved => {
            tracing::debug!("[CMSG_SWAP_INV_ITEM] Item moved successfully");
        }
        crate::world::game::inventory::types::MoveItemResult::Swapped => {
            tracing::debug!("[CMSG_SWAP_INV_ITEM] Items swapped successfully");
        }
        crate::world::game::inventory::types::MoveItemResult::Merged { source_removed } => {
            tracing::debug!(
                "[CMSG_SWAP_INV_ITEM] Items merged, source_removed={}",
                source_removed
            );
        }
        crate::world::game::inventory::types::MoveItemResult::InvalidSource => {
            tracing::warn!("[CMSG_SWAP_INV_ITEM] Invalid source slot");
        }
        crate::world::game::inventory::types::MoveItemResult::InvalidDestination => {
            tracing::warn!("[CMSG_SWAP_INV_ITEM] Invalid destination slot");
        }
        crate::world::game::inventory::types::MoveItemResult::PlayerNotLoaded => {
            tracing::error!("[CMSG_SWAP_INV_ITEM] Player not loaded");
        }
        crate::world::game::inventory::types::MoveItemResult::DatabaseError(e) => {
            tracing::error!("[CMSG_SWAP_INV_ITEM] Database error: {}", e);
        }
        other => {
            tracing::warn!("[CMSG_SWAP_INV_ITEM] Unexpected result: {:?}", other);
        }
    }

    Ok(())
}

pub async fn handle_split_item(
    session: &crate::world::core::session::WorldSession,
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
        crate::world::game::inventory::types::SplitItemResult::Success {
            source_guid,
            new_item_guid,
        } => {
            tracing::debug!(
                "[CMSG_SPLIT_ITEM] Item split successfully: {:?} -> {:?}",
                source_guid,
                new_item_guid
            );
        }
        crate::world::game::inventory::types::SplitItemResult::MergedToExisting {
            source_guid,
            dest_guid,
        } => {
            tracing::debug!(
                "[CMSG_SPLIT_ITEM] Items merged: {:?} into {:?}",
                source_guid,
                dest_guid
            );
        }
        crate::world::game::inventory::types::SplitItemResult::InvalidCount => {
            tracing::warn!("[CMSG_SPLIT_ITEM] Invalid count");
            // Error packet already sent by inventory system
        }
        crate::world::game::inventory::types::SplitItemResult::SourceNotFound => {
            tracing::warn!("[CMSG_SPLIT_ITEM] Source item not found");
            // Error packet already sent by inventory system
        }
        crate::world::game::inventory::types::SplitItemResult::DestinationOccupied => {
            tracing::warn!("[CMSG_SPLIT_ITEM] Destination occupied or cannot stack");
            // Error packet already sent by inventory system
        }
        crate::world::game::inventory::types::SplitItemResult::PlayerNotLoaded => {
            tracing::error!("[CMSG_SPLIT_ITEM] Player not loaded");
        }
        crate::world::game::inventory::types::SplitItemResult::DatabaseError(e) => {
            tracing::error!("[CMSG_SPLIT_ITEM] Database error: {}", e);
        }
        other => {
            tracing::warn!("[CMSG_SPLIT_ITEM] Unexpected result: {:?}", other);
        }
    }

    Ok(())
}

pub async fn handle_autoequip_item_slot(
    session: &crate::world::core::session::WorldSession,
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

    const INVENTORY_SLOT_BAG_0: u8 = 255;
    let (src_bag, src_slot) = find_item_location(player_guid, item_guid, &world.systems.inventory);

    if let (Some(src_bag), Some(src_slot)) = (src_bag, src_slot) {
        let result =
            world
                .systems
                .inventory
                .equip_item(player_guid, src_bag, src_slot, equip_slot, 1, 1, 1);

        match result.await {
            crate::world::game::inventory::types::EquipResult::Equipped => {
                tracing::debug!("[CMSG_AUTOEQUIP_ITEM_SLOT] Item equipped successfully");
            }
            crate::world::game::inventory::types::EquipResult::Swapped {
                unequipped_to_bag,
                unequipped_to_slot,
            } => {
                tracing::debug!(
                    "[CMSG_AUTOEQUIP_ITEM_SLOT] Items swapped, unequipped to bag={} slot={}",
                    unequipped_to_bag,
                    unequipped_to_slot
                );
            }
            crate::world::game::inventory::types::EquipResult::LevelTooLow => {
                tracing::warn!("[CMSG_AUTOEQUIP_ITEM_SLOT] Level too low");
            }
            crate::world::game::inventory::types::EquipResult::WrongClass => {
                tracing::warn!("[CMSG_AUTOEQUIP_ITEM_SLOT] Wrong class");
            }
            crate::world::game::inventory::types::EquipResult::MissingProficiency => {
                tracing::warn!("[CMSG_AUTOEQUIP_ITEM_SLOT] Missing proficiency");
            }
            crate::world::game::inventory::types::EquipResult::WrongSlot => {
                tracing::warn!("[CMSG_AUTOEQUIP_ITEM_SLOT] Wrong slot");
            }
            crate::world::game::inventory::types::EquipResult::InventoryFull => {
                tracing::warn!("[CMSG_AUTOEQUIP_ITEM_SLOT] Inventory full");
            }
            crate::world::game::inventory::types::EquipResult::ItemNotFound => {
                tracing::warn!("[CMSG_AUTOEQUIP_ITEM_SLOT] Item not found");
            }
            crate::world::game::inventory::types::EquipResult::PlayerNotLoaded => {
                tracing::error!("[CMSG_AUTOEQUIP_ITEM_SLOT] Player not loaded");
            }
            crate::world::game::inventory::types::EquipResult::DatabaseError(e) => {
                tracing::error!("[CMSG_AUTOEQUIP_ITEM_SLOT] Database error: {}", e);
            }
            other => {
                tracing::warn!("[CMSG_AUTOEQUIP_ITEM_SLOT] Unexpected result: {:?}", other);
            }
        }
    } else {
        tracing::warn!("[CMSG_AUTOEQUIP_ITEM_SLOT] Item not found in inventory");
    }

    Ok(())
}

pub async fn handle_autoequip_item(
    session: &crate::world::core::session::WorldSession,
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

    let (entry_id, count) = {
        let item = src_item.read();
        (item.entry, item.count)
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

    // Based on MaNGOS ItemPrototype::GetAllowedEquipSlots (Item.cpp)
    let equip_slot = match template.inventory_type {
        1 => 0,   // INVTYPE_HEAD -> EQUIPMENT_SLOT_HEAD
        2 => 1,   // INVTYPE_NECK -> EQUIPMENT_SLOT_NECK
        3 => 2,   // INVTYPE_SHOULDERS -> EQUIPMENT_SLOT_SHOULDERS
        4 => 3,   // INVTYPE_BODY -> EQUIPMENT_SLOT_BODY
        5 => 4,   // INVTYPE_CHEST -> EQUIPMENT_SLOT_CHEST
        6 => 4,   // INVTYPE_ROBE -> EQUIPMENT_SLOT_CHEST
        7 => 5,   // INVTYPE_WAIST -> EQUIPMENT_SLOT_WAIST
        8 => 6,   // INVTYPE_LEGS -> EQUIPMENT_SLOT_LEGS
        9 => 7,   // INVTYPE_FEET -> EQUIPMENT_SLOT_FEET
        10 => 8,  // INVTYPE_WRISTS -> EQUIPMENT_SLOT_WRISTS
        11 => 10, // INVTYPE_FINGER -> EQUIPMENT_SLOT_FINGER1 (TODO: check if finger2 is empty)
        12 => 12, // INVTYPE_TRINKET -> EQUIPMENT_SLOT_TRINKET1 (TODO: check if trinket2 is empty)
        13 => 15, // INVTYPE_WEAPON -> EQUIPMENT_SLOT_MAINHAND
        14 => 16, // INVTYPE_SHIELD -> EQUIPMENT_SLOT_OFFHAND
        15 => 17, // INVTYPE_RANGED -> EQUIPMENT_SLOT_RANGED
        16 => 14, // INVTYPE_CLOAK -> EQUIPMENT_SLOT_BACK
        17 => 15, // INVTYPE_2HWEAPON -> EQUIPMENT_SLOT_MAINHAND
        19 => 18, // INVTYPE_TABARD -> EQUIPMENT_SLOT_TABARD
        20 => 4,  // INVTYPE_ROBE -> EQUIPMENT_SLOT_CHEST
        21 => 15, // INVTYPE_WEAPONMAINHAND -> EQUIPMENT_SLOT_MAINHAND
        22 => 16, // INVTYPE_WEAPONOFFHAND -> EQUIPMENT_SLOT_OFFHAND
        23 => 16, // INVTYPE_HOLDABLE -> EQUIPMENT_SLOT_OFFHAND
        25 => 17, // INVTYPE_THROWN -> EQUIPMENT_SLOT_RANGED
        26 => 17, // INVTYPE_RANGEDRIGHT -> EQUIPMENT_SLOT_RANGED
        _ => {
            tracing::warn!(
                "[CMSG_AUTOEQUIP_ITEM] Item {} cannot be equipped (inventory_type={})",
                entry_id,
                template.inventory_type
            );
            return Ok(());
        }
    };

    tracing::info!(
        "[CMSG_AUTOEQUIP_ITEM] Item entry={} name='{}' inventory_type={} -> equip_slot={}",
        entry_id,
        template.name,
        template.inventory_type,
        equip_slot
    );

    let result =
        world
            .systems
            .inventory
            .equip_item(player_guid, src_bag, src_slot, equip_slot, 1, 1, 1);

    match result.await {
        crate::world::game::inventory::types::EquipResult::Equipped => {
            tracing::debug!(
                "[CMSG_AUTOEQUIP_ITEM] Equipped item {:?} to slot {}",
                src_item_guid,
                equip_slot
            );
        }
        crate::world::game::inventory::types::EquipResult::Swapped {
            unequipped_to_bag,
            unequipped_to_slot,
        } => {
            tracing::debug!(
                "[CMSG_AUTOEQUIP_ITEM] Swapped item {:?} to slot {}, unequipped to bag={} slot={}",
                src_item_guid,
                equip_slot,
                unequipped_to_bag,
                unequipped_to_slot
            );
        }
        crate::world::game::inventory::types::EquipResult::LevelTooLow => {
            tracing::warn!("[CMSG_AUTOEQUIP_ITEM] Level too low");
        }
        crate::world::game::inventory::types::EquipResult::WrongClass => {
            tracing::warn!("[CMSG_AUTOEQUIP_ITEM] Wrong class");
        }
        crate::world::game::inventory::types::EquipResult::MissingProficiency => {
            tracing::warn!("[CMSG_AUTOEQUIP_ITEM] Missing proficiency");
        }
        crate::world::game::inventory::types::EquipResult::WrongSlot => {
            tracing::warn!("[CMSG_AUTOEQUIP_ITEM] Wrong slot");
        }
        crate::world::game::inventory::types::EquipResult::InventoryFull => {
            tracing::warn!("[CMSG_AUTOEQUIP_ITEM] Inventory full");
        }
        crate::world::game::inventory::types::EquipResult::ItemNotFound => {
            tracing::warn!("[CMSG_AUTOEQUIP_ITEM] Item not found");
        }
        crate::world::game::inventory::types::EquipResult::PlayerNotLoaded => {
            tracing::error!("[CMSG_AUTOEQUIP_ITEM] Player not loaded");
        }
        crate::world::game::inventory::types::EquipResult::DatabaseError(e) => {
            tracing::error!("[CMSG_AUTOEQUIP_ITEM] Database error: {}", e);
        }
        crate::world::game::inventory::types::EquipResult::InventoryError(e) => {
            tracing::warn!("[CMSG_AUTOEQUIP_ITEM] Inventory error: {:?}", e);
        }
        other => {
            tracing::warn!("[CMSG_AUTOEQUIP_ITEM] Equip failed: {:?}", other);
        }
    }

    Ok(())
}

pub async fn handle_autoequip_ground_item(
    session: &crate::world::core::session::WorldSession,
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
    session: &crate::world::core::session::WorldSession,
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
    session: &crate::world::core::session::WorldSession,
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
    session: &crate::world::core::session::WorldSession,
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
    session: &crate::world::core::session::WorldSession,
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

    let destroy_count = if count > 0 { count as u32 } else { u32::MAX };

    if let Some(item_guid) = world.systems.inventory.get_item_at(player_guid, bag, slot) {
        warn!(
            "CMSG_DESTROYITEM: player={:?} bag={} slot={} count={} item={:?} — client is destroying this item",
            player_guid, bag, slot, destroy_count, item_guid
        );
        let _ = world
            .systems
            .inventory
            .remove_item(player_guid, item_guid, destroy_count);
    }

    Ok(())
}

pub async fn handle_set_ammo(
    session: &crate::world::core::session::WorldSession,
    packet: &mut WorldPacket,
    _world: &World,
) -> Result<()> {
    let _item_entry = packet
        .read_u32()
        .ok_or_else(|| anyhow!("Failed to read item entry"))?;

    let _player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    warn!("CMSG_SET_AMMO received but not implemented");

    Ok(())
}

pub async fn handle_autobank_item(
    session: &crate::world::core::session::WorldSession,
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
    session: &crate::world::core::session::WorldSession,
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
    session: &crate::world::core::session::WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let _banker_guid = packet
        .read_packed_guid_raw()
        .ok_or_else(|| anyhow!("Failed to read banker guid"))?;

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    world
        .systems
        .inventory
        .send_buy_bank_slot_result(player_guid, 0);

    Ok(())
}

pub async fn handle_buyback_item(
    session: &crate::world::core::session::WorldSession,
    packet: &mut WorldPacket,
    _world: &World,
) -> Result<()> {
    let vendor_guid_raw = packet
        .read_packed_guid_raw()
        .ok_or_else(|| anyhow!("Failed to read vendor guid"))?;
    let vendor_guid = ObjectGuid::from(vendor_guid_raw);
    let _slot = packet
        .read_u32()
        .ok_or_else(|| anyhow!("Failed to read slot"))?;

    let _player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    warn!(
        "CMSG_BUYBACK_ITEM received but not fully implemented for vendor {:?} slot {}",
        vendor_guid, _slot
    );

    Ok(())
}

fn find_item_location(
    player_guid: ObjectGuid,
    item_guid: ObjectGuid,
    inventory: &crate::world::game::inventory::InventorySystem,
) -> (Option<u8>, Option<u8>) {
    const INVENTORY_SLOT_BAG_0: u8 = 255;

    for slot in 0..19u8 {
        if let Some(guid) = inventory.get_item_at(player_guid, INVENTORY_SLOT_BAG_0, slot) {
            if guid == item_guid {
                return (Some(INVENTORY_SLOT_BAG_0), Some(slot));
            }
        }
    }

    for slot in 23..39u8 {
        if let Some(guid) = inventory.get_item_at(player_guid, INVENTORY_SLOT_BAG_0, slot) {
            if guid == item_guid {
                return (Some(INVENTORY_SLOT_BAG_0), Some(slot));
            }
        }
    }

    (None, None)
}
