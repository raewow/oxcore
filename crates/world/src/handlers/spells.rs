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
use oxcore_shared::messages::spells::next_cast_sequence;
use oxcore_shared::messages::update::DEFAULT_REALM_ID;
use oxcore_shared::protocol::bitbuf::BitReader;
use oxcore_shared::protocol::guid::cast_guid128;
use oxcore_shared::protocol::{ObjectGuid, Opcode, Protocol, WorldPacket};

/// Parse SpellCastTargets from a CMSG_CAST_SPELL packet.
///
/// Format for 1.12.x client:
/// - u32 target_flags
/// - if UNIT|UNK2: packed GUID
/// - if OBJECT: packed GUID
/// - if ITEM|TRADE_ITEM: packed GUID
/// - if CORPSE|PVP_CORPSE: packed GUID
/// - if SOURCE_LOCATION: packed GUID (transport) + 3x f32
/// - if DEST_LOCATION: packed GUID (transport) + 3x f32
/// - if STRING: null-terminated string
pub(crate) fn parse_spell_cast_targets(
    protocol: Protocol,
    packet: &mut WorldPacket,
    caster_guid: ObjectGuid,
) -> Result<SpellCastTargets> {
    match protocol {
        Protocol::Vanilla => parse_spell_cast_targets_vanilla(packet, caster_guid),
        Protocol::Modern => {
            let mut reader = BitReader::new(packet.contents());
            let targets = parse_spell_cast_targets_modern_reader(&mut reader, caster_guid)?;
            let consumed = reader.consumed();
            packet.advance(consumed);
            Ok(targets)
        }
    }
}

/// `SpellTargetData::Read` for build 42597, per the 1.14 wire format.
///
/// This is a branch rather than a translation -- the two layouts share nothing beyond the concept:
///
/// * the flag word is **26 bits**, not a u16, and is followed by four presence bits and a 7-bit
///   name length, all in one bit run;
/// * the unit and item GUIDs are **unconditional**, where vanilla gates each on a flag;
/// * there is no separate gameobject or corpse GUID -- they arrive in the unit slot;
/// * source and destination locations each carry a transport GUID *and* a position;
/// * orientation and map id are new, and hang off presence bits.
///
/// Takes an existing reader rather than owning one: this block is preceded by a much larger
/// preamble on `CMSG_CAST_SPELL` and `CMSG_USE_ITEM` (see [`parse_modern_spell_cast_request`]),
/// and the bit cursor must carry over from there rather than restart at the packet's own byte 0.
fn parse_spell_cast_targets_modern_reader(
    reader: &mut BitReader<'_>,
    caster_guid: ObjectGuid,
) -> Result<SpellCastTargets> {
    let target_flags = reader
        .read_bits(26)
        .ok_or_else(|| anyhow::anyhow!("Failed to read modern target_flags"))?;
    let has_src = reader
        .read_bit()
        .ok_or_else(|| anyhow::anyhow!("Failed to read HasSrcLocation"))?;
    let has_dst = reader
        .read_bit()
        .ok_or_else(|| anyhow::anyhow!("Failed to read HasDstLocation"))?;
    let has_orientation = reader
        .read_bit()
        .ok_or_else(|| anyhow::anyhow!("Failed to read HasOrientation"))?;
    let has_map_id = reader
        .read_bit()
        .ok_or_else(|| anyhow::anyhow!("Failed to read HasMapID"))?;
    let name_length = reader
        .read_bits(7)
        .ok_or_else(|| anyhow::anyhow!("Failed to read target name length"))? as usize;

    let unit = reader
        .read_packed_guid_128()
        .map(|(high, low)| ObjectGuid::from_guid128(high, low))
        .ok_or_else(|| anyhow::anyhow!("Failed to read modern unit target GUID"))?;
    let item = reader
        .read_packed_guid_128()
        .map(|(high, low)| ObjectGuid::from_guid128(high, low))
        .ok_or_else(|| anyhow::anyhow!("Failed to read modern item target GUID"))?;

    let mut read_location = |what: &str| -> Result<(f32, f32, f32)> {
        // The transport GUID is read and discarded: vanilla's location block carries one too and
        // the callers have never used it. It must still be consumed, or the position is read from
        // the GUID's bytes.
        reader
            .read_packed_guid_128()
            .ok_or_else(|| anyhow::anyhow!("Failed to read {what} transport GUID"))?;
        let x = reader
            .read_f32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read {what} x"))?;
        let y = reader
            .read_f32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read {what} y"))?;
        let z = reader
            .read_f32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read {what} z"))?;
        Ok((x, y, z))
    };

    let src_position = has_src.then(|| read_location("src location")).transpose()?;
    let dst_position = has_dst.then(|| read_location("dst location")).transpose()?;

    if has_orientation {
        reader
            .read_f32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read target orientation"))?;
    }
    if has_map_id {
        reader
            .read_u32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read target map id"))?;
    }

    let str_target = (name_length > 0)
        .then(|| {
            reader
                .read_string(name_length)
                .ok_or_else(|| anyhow::anyhow!("Failed to read target name"))
        })
        .transpose()?;

    // A self-cast still arrives with the flag set and an empty unit GUID, so resolve it here the
    // way the vanilla branch does.
    let unit_target_guid = if target_flags == TARGET_FLAG_SELF || unit.is_empty() {
        Some(caster_guid)
    } else {
        Some(unit)
    };

    Ok(SpellCastTargets {
        target_flags,
        unit_target_guid,
        // 1.14 folds gameobject and corpse targets into the single unit slot, so mirror the GUID
        // into whichever field its own type says it belongs in -- callers switch on these.
        gameobject_target_guid: unit.is_game_object().then_some(unit),
        item_target_guid: (!item.is_empty()).then_some(item),
        corpse_target_guid: unit.is_corpse().then_some(unit),
        src_position,
        dst_position,
        str_target,
    })
}

/// The result of parsing a modern `SpellCastRequest` body (`CMSG_CAST_SPELL`, `CMSG_USE_ITEM`).
pub(crate) struct ModernSpellCastRequest {
    /// The cast identity the *client* generated for this press. Modern acknowledges every cast
    /// with `SMSG_SPELL_PREPARE`, echoing this back paired with the server's own cast id -- without
    /// it, the client's locally-tracked pending cast never links to the `CastID` `SMSG_SPELL_START`
    /// / `SMSG_SPELL_GO` carry, and the cast bar/animation state machine never resolves.
    pub cast_id: (u64, u64),
    pub spell_id: u32,
    /// The modern `SpellXSpellVisualID` the client derived from its own DB2 for this press. The
    /// server echoes it back in `SMSG_SPELL_START`/`SMSG_SPELL_GO` so the client plays the cast's
    /// animations -- it is a DB2 row id unrelated to the spell id (Fireball 133 → 236677), so it
    /// cannot be guessed. See `SmsgSpellStart::spell_visual_id`.
    pub spell_visual_id: u32,
    pub targets: SpellCastTargets,
}

/// A hostile or corrupt packet could otherwise claim a huge reagent/currency/weight count and make
/// us loop absurdly before the read naturally fails; real casts never carry more than a handful.
const MAX_SPELL_CAST_LIST_ENTRIES: u32 = 64;

/// Parse the `SpellCastRequest` body shared by `CMSG_CAST_SPELL` and `CMSG_USE_ITEM` on the modern
/// wire, starting at `CastID` and continuing through the target block. Vanilla has nothing
/// resembling this structure -- its callers read `spell_id` (or `bag`/`slot`/`spell_slot`) directly
/// before the target block, so this function has no vanilla counterpart to call instead.
///
/// This was previously not parsed *at all*: both callers read a bare vanilla-shaped preamble
/// regardless of protocol, so on a modern connection the "spell_id" they extracted was actually the
/// low bytes of this block's packed `CastID`, and every read after it was misaligned by the whole
/// width of this structure. Every cast and every item use was affected.
pub(crate) fn parse_modern_spell_cast_request(
    packet: &mut WorldPacket,
    caster_guid: ObjectGuid,
) -> Result<ModernSpellCastRequest> {
    use oxcore_shared::protocol::movement::MovementInfo;

    let mut reader = BitReader::new(packet.contents());

    let cast_id = reader
        .read_packed_guid_128()
        .ok_or_else(|| anyhow::anyhow!("Failed to read CastID"))?;
    reader
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read Misc[0]"))?;
    reader
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read Misc[1]"))?;
    let spell_id = reader
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read SpellID"))?;
    let spell_visual_id = reader
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read SpellXSpellVisualID"))?;

    // MissileTrajectoryRequest: Pitch, Speed.
    reader
        .read_f32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read missile pitch"))?;
    reader
        .read_f32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read missile speed"))?;

    reader
        .read_packed_guid_128()
        .ok_or_else(|| anyhow::anyhow!("Failed to read CraftingNPC"))?;

    // Optional reagents: {ItemID, Slot, Count} (i32 x3) each. Always present for a normal cast,
    // just empty -- crafting orders are the only source of a nonzero count.
    let reagent_count = reader
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read optional reagent count"))?
        .min(MAX_SPELL_CAST_LIST_ENTRIES);
    for _ in 0..reagent_count {
        reader
            .read_u32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read reagent ItemID"))?;
        reader
            .read_u32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read reagent Slot"))?;
        reader
            .read_u32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read reagent Count"))?;
    }

    // Optional currencies: {CurrencyID, Slot, Count} (i32 x3) each.
    let currency_count = reader
        .read_u32()
        .ok_or_else(|| anyhow::anyhow!("Failed to read optional currency count"))?
        .min(MAX_SPELL_CAST_LIST_ENTRIES);
    for _ in 0..currency_count {
        reader
            .read_u32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read currency CurrencyID"))?;
        reader
            .read_u32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read currency Slot"))?;
        reader
            .read_u32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read currency Count"))?;
    }

    // SendCastFlags (5 bits) + HasMoveUpdate (1 bit) + WeightCount (2 bits) = exactly one byte, so
    // the target block below starts byte-aligned without an explicit flush.
    reader
        .read_bits(5)
        .ok_or_else(|| anyhow::anyhow!("Failed to read SendCastFlags"))?;
    let has_move_update = reader
        .read_bit()
        .ok_or_else(|| anyhow::anyhow!("Failed to read HasMoveUpdate"))?;
    let weight_count = reader
        .read_bits(2)
        .ok_or_else(|| anyhow::anyhow!("Failed to read WeightCount"))?
        .min(MAX_SPELL_CAST_LIST_ENTRIES);

    let targets = parse_spell_cast_targets_modern_reader(&mut reader, caster_guid)?;

    // An embedded movement update (the player moved since their last ack) reuses the same layout
    // as a standalone movement opcode, mover GUID included -- read it with the existing modern
    // movement parser rather than a third copy of that logic. It operates on `WorldPacket`, not
    // this reader, so hand off the packet's cursor and reclaim a fresh reader afterwards.
    if has_move_update {
        let consumed = reader.consumed();
        packet.advance(consumed);
        MovementInfo::read_modern(packet)
            .map_err(|e| anyhow::anyhow!("Failed to read embedded move update: {e}"))?;
        reader = BitReader::new(packet.contents());
    }

    // SpellWeight (crafting order priority): {Type: 2 bits, ID: i32, Quantity: u32}, each entry
    // realigned to a byte boundary first regardless of where the previous one (or the target
    // block, for the first entry) left the cursor.
    for _ in 0..weight_count {
        reader.align();
        reader
            .read_bits(2)
            .ok_or_else(|| anyhow::anyhow!("Failed to read SpellWeight Type"))?;
        reader
            .read_u32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read SpellWeight ID"))?;
        reader
            .read_u32()
            .ok_or_else(|| anyhow::anyhow!("Failed to read SpellWeight Quantity"))?;
    }

    let consumed = reader.consumed();
    packet.advance(consumed);

    Ok(ModernSpellCastRequest {
        cast_id,
        spell_id,
        spell_visual_id,
        targets,
    })
}

fn parse_spell_cast_targets_vanilla(
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

    // Corpse target (packed GUID) — read before locations
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
/// The set of implicit targets where the client picks the unit, used to reject
/// negative spells the player explicitly aimed at themselves.
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
    let player_guid = match session.player_guid() {
        Some(guid) => guid,
        None => return Ok(()),
    };

    // Vanilla: spell_id (u32), then SpellCastTargets. Modern's CMSG_CAST_SPELL leads with a much
    // larger SpellCastRequest block (client cast identity, misc fields, missile trajectory,
    // crafting reagents/currencies, then the target block) -- see
    // `parse_modern_spell_cast_request` for why this must be a real branch, not a shared read.
    let (spell_id, targets, client_cast_id, spell_visual_id) = match session.protocol() {
        Protocol::Vanilla => {
            let spell_id = packet
                .read_u32()
                .ok_or_else(|| anyhow::anyhow!("Failed to read spell_id"))?;
            let targets = parse_spell_cast_targets(session.protocol(), packet, player_guid)?;
            (spell_id, targets, None, 0)
        }
        Protocol::Modern => {
            let request = parse_modern_spell_cast_request(packet, player_guid)?;
            (
                request.spell_id,
                request.targets,
                Some(request.cast_id),
                request.spell_visual_id,
            )
        }
    };

    // The modern client keys its pending press on the cast id IT minted; this server must echo it
    // back paired with its own cast id before START/GO (which carry only the server id) will
    // resolve that press. Mint the server id up front and send SMSG_SPELL_PREPARE right away, so a
    // rejection below can reference the same id and the client's cast bar never hangs.
    let server_sequence = client_cast_id.map(|_| next_cast_sequence());
    let server_cast_id =
        server_sequence.map(|seq| cast_guid128(DEFAULT_REALM_ID, 0, spell_id, seq));
    if let (Some(client_id), Some(server_id)) = (client_cast_id, server_cast_id) {
        world.systems.spells.send_spell_prepare(player_guid, client_id, server_id);
    }

    // Unknown spell id: drop the cast, but loudly — a missing
    // spell_template row leaves the client waiting on a cast that never starts.
    let spell_entry = match world.managers.spell_mgr.get(spell_id) {
        Some(entry) => entry,
        None => {
            tracing::warn!(
                "CMSG_CAST_SPELL: spell {} not in spell_template, cast by {:?} dropped",
                spell_id,
                player_guid
            );
            world.systems.spells.send_cast_result_with_cast_id(
                player_guid,
                spell_id,
                crate::game::player::spells::state::SpellCastError::SpellNotKnown,
                server_cast_id,
                world,
            );
            return Ok(());
        }
    };

    // Guard: the player must actually know the spell and it must not be passive.
    // Treat a violation as a cheat attempt — log and drop, no client response.
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
        world.systems.spells.send_cast_result_with_cast_id(
            player_guid,
            spell_id,
            crate::game::player::spells::state::SpellCastError::InvalidTarget,
            server_cast_id,
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
            server_sequence,
            // The modern client echoes its own DB2 visual id; echo it back so the cast's
            // animations actually play. Vanilla has no such concept (0).
            if session.protocol() == Protocol::Modern {
                Some(spell_visual_id)
            } else {
                None
            },
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

    // Guard battery matching the original aura-cancel logic.
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
    // Negative auras can't be cancelled by the client. Exception only for
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
    let Some(pet_guid) = packet.read_guid_for(session.protocol()) else {
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

/// Whether any of a spell's effects is an area-aura effect.
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
/// Returns true if no holder is found (the area-aura caster check only
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
    use oxcore_shared::protocol::bitbuf::BitWriter;
    use oxcore_shared::protocol::Opcode;

    /// Build a well-formed modern `SpellCastRequest` body: a self-cast Fireball with no reagents,
    /// currencies, movement update or crafting weights -- the shape every ordinary player cast
    /// takes. Mirrors the client write side field-for-field, since nothing in this codebase writes
    /// this body (only the server-side reader exists).
    fn modern_cast_spell_body(client_cast_id: (u64, u64), spell_id: u32) -> Vec<u8> {
        let mut w = BitWriter::new();
        w.write_packed_guid_128(client_cast_id.0, client_cast_id.1); // CastID
        w.write_u32(0); // Misc[0]
        w.write_u32(0); // Misc[1]
        w.write_u32(spell_id);
        w.write_u32(0); // SpellXSpellVisualID
        w.write_f32(0.0); // MissileTrajectory.Pitch
        w.write_f32(0.0); // MissileTrajectory.Speed
        w.write_packed_guid_128(0, 0); // CraftingNPC
        w.write_u32(0); // optionalReagents count
        w.write_u32(0); // optionalCurrencies count
        w.write_bits(0, 5); // SendCastFlags
        w.write_bit(false); // HasMoveUpdate
        w.write_bits(0, 2); // weightCount
        w.flush_bits(); // byte-aligned: 5 + 1 + 2 bits is exactly one byte

        // SpellTargetData: all-empty resolves to a self-cast via the unit-GUID-empty fallback,
        // the same way the vanilla TARGET_FLAG_SELF case does.
        w.write_bits(0, 26); // target_flags
        w.write_bit(false); // HasSrcLocation
        w.write_bit(false); // HasDstLocation
        w.write_bit(false); // HasOrientation
        w.write_bit(false); // HasMapID
        w.write_bits(0, 7); // name length
        w.flush_bits();
        w.write_packed_guid_128(0, 0); // unit target
        w.write_packed_guid_128(0, 0); // item target

        w.into_bytes()
    }

    /// The bug this fixes: `handle_cast_spell` used to read a bare vanilla-shaped `spell_id: u32`
    /// regardless of protocol, so on a modern connection it actually read the low bytes of this
    /// body's packed `CastID` -- and everything after that was misaligned by the whole width of
    /// this structure. Every modern cast was affected.
    #[test]
    fn parses_a_modern_cast_spell_request_end_to_end() {
        let caster = ObjectGuid::new_player(7);
        let client_cast_id = (0x1122_3344_5566_7788u64, 0x99AA_BBCC_DDEE_FF00u64);
        let body = modern_cast_spell_body(client_cast_id, 133); // Fireball

        let mut packet = WorldPacket::new(Opcode::CMSG_CAST_SPELL);
        packet.write_bytes(&body);

        let request = parse_modern_spell_cast_request(&mut packet, caster).unwrap();

        assert_eq!(request.spell_id, 133);
        assert_eq!(request.cast_id, client_cast_id);
        assert_eq!(
            request.targets.unit_target(),
            Some(caster),
            "an all-empty target block must resolve to a self-cast"
        );
        // The whole body must be consumed -- a short read here would misalign whatever the caller
        // reads next from the same packet.
        assert_eq!(packet.contents().len(), 0);
    }

    /// Trailing `SpellWeight` entries (crafting order priority) must each realign to a byte
    /// boundary before their 2-bit `Type` field, matching the reference's `ResetBitPos()` -- and
    /// still leave the packet's cursor at the very end, not one entry short.
    #[test]
    fn parses_trailing_spell_weight_entries() {
        let mut w = BitWriter::new();
        w.write_packed_guid_128(0, 1); // CastID
        w.write_u32(0); // Misc[0]
        w.write_u32(0); // Misc[1]
        w.write_u32(133); // SpellID
        w.write_u32(0); // SpellXSpellVisualID
        w.write_f32(0.0);
        w.write_f32(0.0);
        w.write_packed_guid_128(0, 0); // CraftingNPC
        w.write_u32(0); // reagents
        w.write_u32(0); // currencies
        w.write_bits(0, 5); // SendCastFlags
        w.write_bit(false); // HasMoveUpdate
        w.write_bits(2, 2); // weightCount = 2
        w.flush_bits();
        // Target block, all-empty.
        w.write_bits(0, 26);
        w.write_bit(false);
        w.write_bit(false);
        w.write_bit(false);
        w.write_bit(false);
        w.write_bits(0, 7);
        w.flush_bits();
        w.write_packed_guid_128(0, 0);
        w.write_packed_guid_128(0, 0);
        // Two SpellWeight entries, each realigned first.
        for _ in 0..2 {
            w.write_bits(1, 2); // Type
            w.flush_bits();
            w.write_i32(5); // ID
            w.write_u32(3); // Quantity
        }

        let mut packet = WorldPacket::new(Opcode::CMSG_CAST_SPELL);
        packet.write_bytes(&w.into_bytes());

        let caster = ObjectGuid::new_player(1);
        let request = parse_modern_spell_cast_request(&mut packet, caster).unwrap();

        assert_eq!(request.spell_id, 133);
        assert_eq!(
            packet.contents().len(),
            0,
            "both weight entries must be consumed"
        );
    }

    #[test]
    fn parses_self_target_for_item_use_packets() {
        let caster = ObjectGuid::new_player(1);
        let mut packet = WorldPacket::new(Opcode::CMSG_USE_ITEM);
        packet.write_u16(TARGET_FLAG_SELF as u16);

        let targets = parse_spell_cast_targets(Protocol::Vanilla, &mut packet, caster).unwrap();

        assert_eq!(targets.target_flags, TARGET_FLAG_SELF);
        assert_eq!(targets.unit_target(), Some(caster));
    }

    #[test]
    fn explicit_unit_targets() {
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
