//! Spell Packet Handlers
//!
//! All handlers are slim (3-10 lines): parse packet, delegate to system.

use crate::core::common::packet::WorldPacketGuidExt;
use crate::core::session::WorldSession;
use crate::game::player::spells::state::{
    SpellCastTargets, TARGET_FLAG_CORPSE, TARGET_FLAG_DEST_LOCATION, TARGET_FLAG_ITEM,
    TARGET_FLAG_OBJECT, TARGET_FLAG_PVP_CORPSE, TARGET_FLAG_SELF, TARGET_FLAG_SOURCE_LOCATION,
    TARGET_FLAG_STRING, TARGET_FLAG_TRADE_ITEM, TARGET_FLAG_UNIT, TARGET_FLAG_UNK2,
};
use crate::World;
use anyhow::Result;
use bytes::Buf;
use oxcore_shared::protocol::{ObjectGuid, Opcode, WorldPacket};

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

/// Whether an implicit-target id is a client-chosen explicit unit target.
///
/// Mirrors MaNGOS `Spells::IsExplicitlySelectedUnitTarget` (SpellEntry.h): the set of
/// implicit targets where the client picks the unit, used to reject negative spells the
/// player explicitly aimed at themselves.
fn is_explicitly_selected_unit_target(target: u32) -> bool {
    matches!(
        target,
        6   // TARGET_UNIT_ENEMY
        | 21  // TARGET_UNIT_FRIEND
        | 25  // TARGET_UNIT
        | 35  // TARGET_UNIT_PARTY
        | 45  // TARGET_UNIT_FRIEND_CHAIN_HEAL
        | 53  // TARGET_LOCATION_CASTER_TARGET_POSITION
        | 57  // TARGET_UNIT_RAID
        | 61 // TARGET_UNIT_RAID_AND_CLASS
    )
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

    // Unknown spell id: drop the cast (MaNGOS early return), but loudly — a missing
    // spell_template row leaves the client waiting on a cast that never starts.
    let spell_entry = match world.managers.spell_mgr.get(spell_id) {
        Some(entry) => entry,
        None => {
            tracing::warn!(
                "CMSG_CAST_SPELL: spell {} not in spell_template, cast by {:?} dropped",
                spell_id,
                player_guid
            );
            return Ok(());
        }
    };

    // Guard: the player must actually know the spell and it must not be passive.
    // MaNGOS treats a violation as a cheat attempt — log and drop, no client response.
    let knows_spell = world
        .systems
        .player
        .manager()
        .with_player(player_guid, |p| p.spells.knows_spell(spell_id))
        .unwrap_or(false);
    if !knows_spell || spell_entry.is_passive_spell() {
        tracing::warn!(
            "Player {:?} tried to cast spell {} they shouldn't have (known={}, passive={})",
            player_guid,
            spell_id,
            knows_spell,
            spell_entry.is_passive_spell()
        );
        return Ok(());
    }

    // Parse full SpellCastTargets from the packet
    let targets = parse_spell_cast_targets(packet, player_guid)?;

    // Extract unit target for the current pipeline (will pass full targets later)
    let target_guid = targets.unit_target();

    // Cannot explicitly cast a negative spell on yourself (SPELL_FAILED_BAD_TARGETS).
    // The core itself casts negative spells on the caster for some mechanics, so this only
    // applies when the client explicitly selected the caster as the unit target.
    if target_guid == Some(player_guid)
        && is_explicitly_selected_unit_target(spell_entry.effect_implicit_target_a[0])
        && !spell_entry.is_positive_spell()
    {
        // SpellCastError::InvalidTarget maps to SPELL_FAILED_BAD_TARGETS (0x0A).
        world.systems.spells.send_cast_result(
            player_guid,
            spell_id,
            crate::game::player::spells::state::SpellCastError::InvalidTarget,
            world,
        );
        return Ok(());
    }

    // Casting a spell interrupts looting.
    if let Some(loot_guid) = world
        .systems
        .player
        .manager()
        .get_looting_target(player_guid)
    {
        world
            .systems
            .loot
            .handle_loot_release(player_guid, loot_guid, world)
            .await?;
    }

    world
        .systems
        .spells
        .cast_spell_with_targets(
            player_guid,
            spell_id,
            targets,
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

    let spell_entry = match world.managers.spell_mgr.get(spell_id) {
        Some(entry) => entry,
        None => return Ok(()),
    };

    // Guard battery mirroring MaNGOS HandleCancelAuraOpcode (SpellHandler.cpp:333-405).
    // SPELL_ATTR_NO_AURA_CANCEL (0x80000000): aura is flagged un-cancellable by the client.
    if spell_entry.attributes & 0x8000_0000 != 0 {
        return Ok(());
    }
    // SPELL_ATTR_DO_NOT_DISPLAY (0x80): hidden from spellbook/aura bar.
    if spell_entry.attributes & 0x0000_0080 != 0 {
        return Ok(());
    }
    // SPELL_ATTR_EX_NO_AURA_ICON (0x10000000) with no active icon: client can't show it.
    if spell_entry.attributes_ex & 0x1000_0000 != 0 && spell_entry.active_icon_id == 0 {
        return Ok(());
    }
    // Passive auras can't be cancelled.
    if spell_entry.is_passive_spell() {
        return Ok(());
    }
    // Negative auras can't be cancelled by the client. MaNGOS allows an exception only for
    // POSSESS auras while remote-controlled; we have no possession/remote-control mover, so a
    // normal self-mover player can never cancel a non-positive aura.
    if !spell_entry.is_positive_spell() {
        return Ok(());
    }

    // Channeled spell currently being cast: interrupt the channel instead of removing an aura.
    // SPELL_ATTR_EX_CHANNELED_1 = 0x04, SPELL_ATTR_EX_CHANNELED_2 = 0x40
    if (spell_entry.attributes_ex & 0x04) != 0 || (spell_entry.attributes_ex & 0x40) != 0 {
        world.systems.spells.cancel_cast(player_guid, world).await?;
        return Ok(());
    }

    // A non-own area aura can't be cancelled (e.g. another player's Devotion Aura on you).
    if spell_has_area_aura_effect(&spell_entry.effect) {
        let own_aura = aura_caster_is_self(player_guid, spell_id, world);
        if !own_aura {
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

/// CMSG_PET_CANCEL_AURA (opcode 0x026B)
///
/// Sent when the player right-clicks a removable aura on their active pet.
pub async fn handle_pet_cancel_aura(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let Some(pet_guid) = packet.read_guid_raw().map(ObjectGuid::from_raw) else {
        return Ok(());
    };
    let Some(spell_id) = packet.read_u32() else {
        return Ok(());
    };
    let Some(player_guid) = session.player_guid() else {
        return Ok(());
    };

    let active_pet = world
        .systems
        .player
        .manager()
        .with_player(player_guid, |player| player.active_pet)
        .flatten();
    if !is_active_pet_aura_request(
        session.client_mover_guid(),
        player_guid,
        active_pet,
        pet_guid,
    ) || world.managers.spell_mgr.get(spell_id).is_none()
    {
        return Ok(());
    }

    if !world.managers.creature_mgr.is_alive(pet_guid) {
        const PET_FEEDBACK_PET_DEAD: u8 = 1;
        let mut feedback = WorldPacket::new(Opcode::SMSG_PET_ACTION_FEEDBACK);
        feedback.write_u8(PET_FEEDBACK_PET_DEAD);
        world
            .managers
            .broadcast_mgr
            .send_msg_to_player(player_guid, feedback);
        return Ok(());
    }

    world
        .systems
        .auras
        .remove_spell_auras(pet_guid, spell_id, world)
        .await
}

fn is_active_pet_aura_request(
    mover_guid: Option<ObjectGuid>,
    player_guid: ObjectGuid,
    active_pet: Option<ObjectGuid>,
    requested_pet: ObjectGuid,
) -> bool {
    mover_guid == Some(player_guid) && active_pet == Some(requested_pet)
}

fn is_player_mover(mover_guid: Option<ObjectGuid>, player_guid: ObjectGuid) -> bool {
    mover_guid == Some(player_guid)
}

/// Whether any of a spell's effects is an area-aura effect (MaNGOS `IsAreaAuraEffect`).
fn spell_has_area_aura_effect(effects: &[u32; 3]) -> bool {
    effects.iter().any(|&e| {
        matches!(
            e,
            35   // APPLY_AREA_AURA_PARTY
            | 119  // APPLY_AREA_AURA_PET
            | 128  // APPLY_AREA_AURA_FRIEND
            | 129  // APPLY_AREA_AURA_ENEMY
            | 132 // APPLY_AREA_AURA_RAID
        )
    })
}

/// Whether the given spell's aura on the player was cast by the player themselves.
/// Returns true if no holder is found (mirrors MaNGOS: the area-aura caster check only
/// blocks when a holder exists with a different caster).
fn aura_caster_is_self(player_guid: ObjectGuid, spell_id: u32, world: &World) -> bool {
    world
        .systems
        .player
        .manager()
        .with_player(player_guid, |player| {
            for effect_index in 0..3u8 {
                if let Some(aura) = player.auras.container.get_aura(spell_id, effect_index) {
                    return aura.caster_guid == player_guid;
                }
            }
            true
        })
        .unwrap_or(true)
}

/// CMSG_CANCEL_GROWTH_AURA (opcode 0x029B)
///
/// The Vanilla server intentionally treats this opcode as a no-op.
pub async fn handle_cancel_growth_aura(
    _session: &WorldSession,
    _packet: &mut WorldPacket,
    _world: &World,
) -> Result<()> {
    Ok(())
}

/// CMSG_CANCEL_AUTO_REPEAT_SPELL (opcode 0x026D)
///
/// Sent when the player cancels auto-repeat spells (auto-shot, wand).
pub async fn handle_cancel_auto_repeat_spell(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    if !is_player_mover(session.client_mover_guid(), player_guid) {
        return Ok(());
    }

    world
        .systems
        .spells
        .cancel_auto_repeat_spell(player_guid, world)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_unit_targets_match_mangos_set() {
        // Client-chosen unit targets (TARGET_UNIT_ENEMY/FRIEND/UNIT/PARTY/CHAIN_HEAL/
        // CASTER_TARGET_POSITION/RAID/RAID_AND_CLASS).
        for t in [6, 21, 25, 35, 45, 53, 57, 61] {
            assert!(
                is_explicitly_selected_unit_target(t),
                "target {t} should be explicit"
            );
        }
    }

    #[test]
    fn non_explicit_targets_are_rejected() {
        // Self (1), area/AoE (15, 16, 22, 24), pet (27), nearby-enemy (2), and none (0)
        // are NOT client-selected explicit unit targets.
        for t in [0, 1, 2, 15, 16, 22, 24, 27] {
            assert!(
                !is_explicitly_selected_unit_target(t),
                "target {t} should not be explicit"
            );
        }
    }

    #[test]
    fn detects_area_aura_effects() {
        // APPLY_AREA_AURA_PARTY/PET/FRIEND/ENEMY/RAID in any effect slot.
        assert!(spell_has_area_aura_effect(&[35, 0, 0]));
        assert!(spell_has_area_aura_effect(&[0, 119, 0]));
        assert!(spell_has_area_aura_effect(&[0, 0, 128]));
        assert!(spell_has_area_aura_effect(&[129, 0, 0]));
        assert!(spell_has_area_aura_effect(&[0, 132, 0]));
    }

    #[test]
    fn ignores_non_area_aura_effects() {
        // APPLY_AURA (6), SCHOOL_DAMAGE (2), HEAL (10), none (0).
        assert!(!spell_has_area_aura_effect(&[0, 0, 0]));
        assert!(!spell_has_area_aura_effect(&[6, 2, 10]));
    }

    #[test]
    fn pet_aura_cancellation_requires_self_mover_and_active_pet() {
        let player = ObjectGuid::new_player(1);
        let pet = ObjectGuid::new_pet(100, 2);
        let other_pet = ObjectGuid::new_pet(100, 3);

        assert!(is_active_pet_aura_request(
            Some(player),
            player,
            Some(pet),
            pet
        ));
        assert!(!is_active_pet_aura_request(
            Some(other_pet),
            player,
            Some(pet),
            pet
        ));
        assert!(!is_active_pet_aura_request(
            Some(player),
            player,
            Some(pet),
            other_pet
        ));
    }

    #[test]
    fn auto_repeat_cancellation_requires_the_player_mover() {
        let player = ObjectGuid::new_player(1);
        let pet = ObjectGuid::new_pet(100, 2);

        assert!(is_player_mover(Some(player), player));
        assert!(!is_player_mover(Some(pet), player));
        assert!(!is_player_mover(None, player));
    }
}
