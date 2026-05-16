//! Spell Packet Handlers
//!
//! All handlers are slim (3-10 lines): parse packet, delegate to system.

use crate::shared::protocol::{ObjectGuid, Opcode, WorldPacket};
use crate::world::core::common::packet::WorldPacketGuidExt;
use crate::world::core::session::WorldSession;
use crate::world::game::player::spells::state::{
    SpellCastTargets, TARGET_FLAG_CORPSE, TARGET_FLAG_DEST_LOCATION, TARGET_FLAG_ITEM,
    TARGET_FLAG_OBJECT, TARGET_FLAG_PVP_CORPSE, TARGET_FLAG_SELF, TARGET_FLAG_SOURCE_LOCATION,
    TARGET_FLAG_STRING, TARGET_FLAG_TRADE_ITEM, TARGET_FLAG_UNIT, TARGET_FLAG_UNK2,
};
use crate::world::World;
use anyhow::Result;
use bytes::Buf;

/// Parse SpellCastTargets from a CMSG_CAST_SPELL packet.
///
/// Format matches MaNGOS SpellCastTargets::read() for 1.12.x client:
/// - u32 target_flags
/// - if UNIT|UNK2: packed GUID
/// - if OBJECT: packed GUID
/// - if ITEM|TRADE_ITEM: packed GUID
/// - if CORPSE|PVP_CORPSE: packed GUID
/// - if SOURCE_LOCATION: packed GUID (transport) + 3x f32
/// - if DEST_LOCATION: packed GUID (transport) + 3x f32
/// - if STRING: null-terminated string
fn parse_spell_cast_targets(
    packet: &mut WorldPacket,
    caster_guid: ObjectGuid,
) -> Result<SpellCastTargets> {
    let target_flags = packet
        .read_u16()
        .ok_or_else(|| anyhow::anyhow!("Failed to read target_flags"))?
        as u32;

    let mut targets = SpellCastTargets {
        target_flags,
        ..Default::default()
    };

    // Self-cast: no additional data
    if target_flags == TARGET_FLAG_SELF {
        targets.unit_target_guid = Some(caster_guid);
        return Ok(targets);
    }

    // Unit target (packed GUID)
    if target_flags & (TARGET_FLAG_UNIT | TARGET_FLAG_UNK2) != 0 {
        targets.unit_target_guid = packet.read_packed_guid();
    }

    // GameObject target (packed GUID)
    if target_flags & TARGET_FLAG_OBJECT != 0 {
        targets.gameobject_target_guid = packet.read_packed_guid();
    }

    // Item target (packed GUID)
    if target_flags & (TARGET_FLAG_ITEM | TARGET_FLAG_TRADE_ITEM) != 0 {
        targets.item_target_guid = packet.read_packed_guid();
    }

    // Corpse target (packed GUID) — read before locations per MaNGOS order
    if target_flags & (TARGET_FLAG_CORPSE | TARGET_FLAG_PVP_CORPSE) != 0 {
        targets.corpse_target_guid = packet.read_packed_guid();
    }

    // Source location (transport packed GUID + 3 floats)
    if target_flags & TARGET_FLAG_SOURCE_LOCATION != 0 {
        let _transport_guid = packet.read_packed_guid(); // transport GUID (usually 0)
        let x = packet.read_f32().unwrap_or(0.0);
        let y = packet.read_f32().unwrap_or(0.0);
        let z = packet.read_f32().unwrap_or(0.0);
        targets.src_position = Some((x, y, z));
    }

    // Destination location (transport packed GUID + 3 floats)
    if target_flags & TARGET_FLAG_DEST_LOCATION != 0 {
        let _transport_guid = packet.read_packed_guid(); // transport GUID (usually 0)
        let x = packet.read_f32().unwrap_or(0.0);
        let y = packet.read_f32().unwrap_or(0.0);
        let z = packet.read_f32().unwrap_or(0.0);
        targets.dst_position = Some((x, y, z));
    }

    // String target
    if target_flags & TARGET_FLAG_STRING != 0 {
        targets.str_target = packet.read_string();
    }

    Ok(targets)
}

/// CMSG_CAST_SPELL (opcode 0x012E)
///
/// Sent when the player presses a spell button.
pub async fn handle_cast_spell(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    // Vanilla 1.12.x format: spell_id (u32), then SpellCastTargets
    let spell_id = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read spell_id"))?;

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    // Parse full SpellCastTargets from the packet
    let targets = parse_spell_cast_targets(packet, player_guid)?;

    // Extract unit target for the current pipeline (will pass full targets later)
    let target_guid = targets.unit_target();

    world
        .systems
        .spells
        .cast_spell(
            player_guid,
            spell_id,
            target_guid,
            false, // not triggered
            world,
        )
        .await?;

    Ok(())
}

/// CMSG_CANCEL_CAST (opcode 0x012F)
///
/// Sent when the player cancels a cast (Escape key, movement, etc.)
/// Packet: u8 counter (unused), u32 spell_id
pub async fn handle_cancel_cast(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    let _counter = packet.read_u8().unwrap_or(0);
    let spell_id = packet.read_u32().unwrap_or(0);

    if spell_id != 0 {
        world
            .systems
            .spells
            .cancel_cast_by_spell_id(player_guid, spell_id, world)
            .await?;
    } else {
        world.systems.spells.cancel_cast(player_guid, world).await?;
    }

    Ok(())
}

/// CMSG_CANCEL_CHANNELLING (opcode 0x013B)
///
/// Sent when the player cancels a channeled spell.
pub async fn handle_cancel_channelling(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    // Cancel channelling is handled the same as cancel cast
    world.systems.spells.cancel_cast(player_guid, world).await?;

    Ok(())
}

/// CMSG_CANCEL_AURA (opcode 0x0136)
///
/// Sent when the player right-clicks a buff icon to remove it.
pub async fn handle_cancel_aura(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let spell_id = packet
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read spell_id"))?;

    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    // Check if this is a channeled spell — if so, interrupt the channel instead
    if let Some(spell_entry) = world.managers.spell_mgr.get(spell_id) {
        // SPELL_ATTR_EX_CHANNELED_1 = 0x04, SPELL_ATTR_EX_CHANNELED_2 = 0x40
        if (spell_entry.attributes_ex & 0x04) != 0 || (spell_entry.attributes_ex & 0x40) != 0 {
            world.systems.spells.cancel_cast(player_guid, world).await?;
            return Ok(());
        }
    }

    world
        .systems
        .auras
        .cancel_aura(player_guid, spell_id, world)
        .await?;

    Ok(())
}

/// CMSG_CANCEL_AUTO_REPEAT_SPELL (opcode 0x013C)
///
/// Sent when the player cancels auto-repeat spells (auto-shot, wand).
pub async fn handle_cancel_auto_repeat_spell(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    _world: &World,
) -> Result<()> {
    let _player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    // TODO: Implement auto-repeat spell cancellation
    // This requires integration with the combat system

    Ok(())
}
