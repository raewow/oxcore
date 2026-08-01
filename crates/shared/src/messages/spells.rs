use crate::messages::update::DEFAULT_REALM_ID;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::guid::cast_guid128;
use crate::protocol::packet::WorldPacketGuidExt;
use crate::protocol::{ObjectGuid, Opcode, WorldPacket};
use std::sync::atomic::{AtomicU64, Ordering};

/// SMSG_SPELL_PREPARE (modern-only; 1.14 build 42597 wire number 0x2C38)
///
/// The modern client acknowledges every cast press with a locally-tracked pending cast keyed on
/// the `CastID` *it* minted (sent in `CMSG_CAST_SPELL`/`CMSG_USE_ITEM`). This packet links that
/// client-generated id to the server's own cast id, which `SMSG_SPELL_START`/`SMSG_SPELL_GO`
/// then carry — without the link, the client never resolves its pending press and the cast
/// bar/animation state machine hangs.
///
/// Vanilla has no such message: a 1.12 client keys casts directly on the ids the server already
/// sends. This message is therefore never sent to a vanilla session, and `to_vanilla` is
/// unreachable in practice; it exists only to satisfy the trait.
pub struct SmsgSpellPrepare {
    /// The cast id the *client* minted and echoed in its `CMSG_CAST_SPELL`/`CMSG_USE_ITEM`.
    pub client_cast_id: (u64, u64),
    /// The server's own cast id (a modern `Cast` GUID128, see [`cast_guid128`]). Must be the same
    /// value the following `SMSG_SPELL_START`/`SMSG_SPELL_GO` carry.
    pub server_cast_id: (u64, u64),
}

impl ToWorldPacket for SmsgSpellPrepare {
    fn to_vanilla(&self) -> WorldPacket {
        // Unreachable for vanilla sessions — see the struct doc. Return an empty packet rather
        // than write a body that has no meaning on the 1.12 wire.
        WorldPacket::new(Opcode::SMSG_SPELL_PREPARE)
    }

    /// Two packed GUID128s: `ClientCastID` then `ServerCastID`, per JimsProxy's `SpellPrepare`.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.client_cast_id;
        writer.write_packed_guid_128(high, low);
        let (high, low) = self.server_cast_id;
        writer.write_packed_guid_128(high, low);
        Some(writer.finish(Opcode::SMSG_SPELL_PREPARE))
    }
}

/// SMSG_SPELL_START (opcode 0x0131)
/// Broadcast when a spell cast begins (cast bar appears).
pub struct SmsgSpellStart {
    pub caster_guid: ObjectGuid,
    pub caster_guid_pack: ObjectGuid,
    pub spell_id: u32,
    /// The modern `SpellXSpellVisualID`: the DB2 visual id the client resolves to play the cast's
    /// visuals. For a modern client press this is the value it echoed in `CMSG_CAST_SPELL`
    /// (`ModernSpellCastRequest::spell_visual_id`); `0` (no modern origin) plays no animation.
    pub spell_visual_id: u32,
    pub cast_flags: u16,
    pub cast_time_ms: u32,
    pub target_guid: Option<ObjectGuid>,
    /// When the spell is cast from an item, this is the item's GUID. The client
    /// expects the item GUID as the first packed GUID in the packet instead of
    /// the caster's GUID (`cast_item_guid` overrides the first GUID slot).
    pub cast_item_guid: Option<ObjectGuid>,
    /// Projectile ammo display info: ammoDisplayID.
    /// 0 = no projectile.
    pub ammo_display_id: u32,
    /// Inventory type of the projectile (e.g. INVTYPE_THROWN, INVTYPE_AMMO).
    /// 0 = no projectile.
    pub ammo_inventory_type: u32,
    /// Identifies this cast, for modern's `CastID`. See [`next_cast_sequence`].
    ///
    /// `SMSG_SPELL_START` and `SMSG_SPELL_GO` for the *same* cast must carry the same value: the
    /// client opens its cast bar under START's `CastID` and expects GO to close that same bar, not
    /// announce an unrelated one. The caller must compute this once per cast (via
    /// [`next_cast_sequence`]) and pass it to both messages -- neither message may mint its own,
    /// which is what made every instant cast register as two different, un-paired casts.
    pub cast_sequence: u64,
}

// =============================================================================
// Modern spell cast encoding
// =============================================================================

/// Cast sequence counter, so every cast gets a distinct `Cast` GUID.
///
/// The 1.14 client keys its cast bar, sounds and visual chunks on the cast GUID and assumes it is
/// unique per cast; see [`cast_guid128`] for what happens when it is not. A process-global counter
/// is enough — the GUID also carries the spell id and map, so collisions would need the same spell on
/// the same map 2^40 casts apart.
static CAST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Take the next cast-sequence value.
///
/// Call this **once per cast**, not once per message: `SmsgSpellStart` and `SmsgSpellGo` both carry
/// a `cast_sequence` field precisely so the caller can pass the same value to both and the client
/// sees one cast, not two.
pub fn next_cast_sequence() -> u64 {
    CAST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
}

/// The spell-cast data block, per the 1.14 wire format.
///
/// Shared by `SMSG_SPELL_START` and `SMSG_SPELL_GO`, which differ only in a trailing log-data bit.
/// Almost nothing survives from the vanilla body unchanged:
///
/// * four GUIDs lead, not two — 1.14 adds a **cast identity** and an original-cast id;
/// * the cast flags widen from u16 to two u32 words;
/// * the counts move into one bit run at mixed widths (16/16/16/9/1/16/1/1);
/// * hit and miss targets become guid128 lists *after* the target block, not before;
/// * missile trajectory, immunities, heal prediction and power costs are all new.
///
/// Fields with no vanilla source are written as defaults. `SpellXSpellVisualID` is the exception:
/// it is what makes the 1.14 client actually *play* the cast's visuals. The client resolves it from
/// its own DB2 and sends it back in `CMSG_CAST_SPELL`, so a native server echoes that value (see
/// `handlers/spells.rs`). Sending zero — the value this function gets for casts with no modern
/// origin — is correct-but-silent: the cast registers and the damage applies, the flourish is
/// missing. A guessed spell-id would be worse than zero, since the visual id is a DB2 row id
/// unrelated to the spell id (e.g. Fireball 133 → 236677).
#[allow(clippy::too_many_arguments)]
fn write_modern_spell_cast_data(
    writer: &mut BitWriter,
    caster_guid: ObjectGuid,
    cast_item_guid: Option<ObjectGuid>,
    spell_id: u32,
    spell_visual_id: u32,
    cast_time_ms: u32,
    target_guid: Option<ObjectGuid>,
    hit_targets: &[ObjectGuid],
    miss_targets: &[SpellMissTarget],
    cast_sequence: u64,
) {
    // Vanilla puts the cast item in the first GUID slot and flags it; 1.14 keeps the caster in both
    // and carries the item in the target block instead, so the item does not displace the caster.
    let (high, low) = caster_guid.to_guid128(DEFAULT_REALM_ID);
    writer.write_packed_guid_128(high, low); // CasterGUID
    writer.write_packed_guid_128(high, low); // CasterUnit

    // Map 0 is a placeholder: the cast GUID's map field is identity, not routing, and the message
    // does not carry the caster's map. A wrong map makes the id no less unique.
    let (cast_high, cast_low) = cast_guid128(DEFAULT_REALM_ID, 0, spell_id, cast_sequence);
    writer.write_packed_guid_128(cast_high, cast_low); // CastID
    writer.write_packed_guid_128(0, 0); // OriginalCastID -- not a triggered cast

    writer.write_i32(spell_id as i32);
    writer.write_u32(spell_visual_id); // SpellXSpellVisualID -- see above
    writer.write_u32(0); // CastFlags
    writer.write_u32(0); // CastFlagsEx
    writer.write_u32(cast_time_ms);

    // MissileTrajectoryResult
    writer.write_u32(0); // TravelTime
    writer.write_f32(0.0); // Pitch

    writer.write_u8(0); // DestLocSpellCastIndex

    // CreatureImmunities
    writer.write_u32(0); // School
    writer.write_u32(0); // Value

    // SpellHealPrediction
    writer.write_u32(0); // Points
    writer.write_u8(0); // Type
    writer.write_packed_guid_128(0, 0); // BeaconGUID

    writer.write_bits(hit_targets.len() as u32, 16);
    writer.write_bits(miss_targets.len() as u32, 16);
    writer.write_bits(miss_targets.len() as u32, 16); // MissStatus, one per miss target
    writer.write_bits(0, 9); // RemainingPower count
    writer.write_bit(false); // HasRemainingRunes -- death knights only
    writer.write_bits(0, 16); // TargetPoints count
    writer.write_bit(false); // HasAmmoDisplayId
    writer.write_bit(false); // HasAmmoInventoryType
    writer.flush_bits();

    // Miss statuses are bit-packed and come before the target block.
    for miss in miss_targets {
        writer.write_bits(u32::from(miss.reason) & 0xF, 4);
        if miss.reason == SPELL_MISS_REFLECT {
            writer.write_bits(u32::from(miss.reflect_result) & 0xF, 4);
        }
    }
    writer.flush_bits();

    write_modern_spell_target_data(writer, target_guid, cast_item_guid);

    for target in hit_targets {
        let (high, low) = target.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
    }
    for miss in miss_targets {
        let (high, low) = miss.guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
    }
    // No power costs, runes, target points or ammo: every one of those was declared absent above.
}

/// The spell-target data block, per the 1.14 wire format.
///
/// The write counterpart of the read in `crates/world/src/handlers/spells.rs`
/// (`parse_spell_cast_targets_modern`). They are in different crates because one is a message body
/// and the other a handler; if you change one, change both — the layouts must stay mirrored.
fn write_modern_spell_target_data(
    writer: &mut BitWriter,
    unit_target: Option<ObjectGuid>,
    item_target: Option<ObjectGuid>,
) {
    /// `TARGET_FLAG_UNIT` and `TARGET_FLAG_ITEM`, which keep their vanilla values in 1.14.
    const TARGET_FLAG_UNIT: u32 = 0x0002;
    const TARGET_FLAG_ITEM: u32 = 0x0010;

    let mut flags = 0;
    if unit_target.is_some() {
        flags |= TARGET_FLAG_UNIT;
    }
    if item_target.is_some() {
        flags |= TARGET_FLAG_ITEM;
    }

    writer.write_bits(flags, 26);
    writer.write_bit(false); // HasSrcLocation
    writer.write_bit(false); // HasDstLocation
    writer.write_bit(false); // HasOrientation
    writer.write_bit(false); // HasMapID
    writer.write_bits(0, 7); // Name length
    writer.flush_bits();

    // Both GUIDs are unconditional here, where vanilla gates each on its flag.
    let (high, low) = unit_target.unwrap_or_default().to_guid128(DEFAULT_REALM_ID);
    writer.write_packed_guid_128(high, low);
    let (high, low) = item_target.unwrap_or_default().to_guid128(DEFAULT_REALM_ID);
    writer.write_packed_guid_128(high, low);
}

impl ToWorldPacket for SmsgSpellStart {
    fn to_vanilla(&self) -> WorldPacket {
        const CAST_FLAG_AMMO: u16 = 0x0020;

        let mut packet = WorldPacket::new(Opcode::SMSG_SPELL_START);
        // First packed GUID: item GUID if cast from an item, else caster GUID
        packet.write_packed_guid(self.cast_item_guid.unwrap_or(self.caster_guid));
        packet.write_packed_guid(self.caster_guid_pack);
        packet.write_u32(self.spell_id);
        packet.write_u16(self.cast_flags);
        packet.write_u32(self.cast_time_ms);

        // SpellCastTargets section (vanilla 1.12.x requires this)
        write_spell_cast_targets(&mut packet, self.target_guid);

        // Ammo projectile info is present only for ranged casts.
        if self.cast_flags & CAST_FLAG_AMMO != 0 {
            packet.write_u32(self.ammo_display_id);
            packet.write_u32(self.ammo_inventory_type);
        }

        tracing::info!("[SPELL_START] spell={} caster={:?} target={:?} cast_flags=0x{:04X} cast_time={} packet_len={} bytes={:02X?}",
            self.spell_id, self.caster_guid, self.target_guid, self.cast_flags, self.cast_time_ms,
            packet.data().as_ref().len(), packet.data().as_ref());
        packet
    }

    /// For build 42597: the shared
    /// cast block and nothing else.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        write_modern_spell_cast_data(
            &mut writer,
            self.caster_guid,
            self.cast_item_guid,
            self.spell_id,
            self.spell_visual_id,
            self.cast_time_ms,
            self.target_guid,
            &[],
            &[],
            self.cast_sequence,
        );
        Some(writer.finish(Opcode::SMSG_SPELL_START))
    }
}

/// SMSG_SPELL_GO (opcode 0x0132)
/// Broadcast when a spell fires (effects apply).
pub struct SmsgSpellGo {
    pub caster_guid: ObjectGuid,
    pub caster_guid_pack: ObjectGuid,
    pub spell_id: u32,
    /// Same semantics as `SmsgSpellStart::spell_visual_id`; must match the value START carried so
    /// the completion resolves the same visual the cast bar opened under.
    pub spell_visual_id: u32,
    pub cast_flags: u16,
    pub hit_targets: Vec<ObjectGuid>,
    pub miss_targets: Vec<SpellMissTarget>,
    pub target_guid: Option<ObjectGuid>,
    /// When the spell is cast from an item, this is the item's GUID. The client
    /// expects the item GUID as the first packed GUID and CAST_FLAG_UNKNOWN7
    /// (0x0040) set in cast_flags.
    pub cast_item_guid: Option<ObjectGuid>,
    /// Projectile ammo display info: ammoDisplayID.
    /// 0 = no projectile.
    pub ammo_display_id: u32,
    /// Inventory type of the projectile (e.g. INVTYPE_THROWN, INVTYPE_AMMO).
    /// 0 = no projectile.
    pub ammo_inventory_type: u32,
    /// Must equal the `cast_sequence` of the `SmsgSpellStart` this GO completes. See that field's
    /// doc comment.
    pub cast_sequence: u64,
}

/// A target omitted from the successful-hit list and its spell miss result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellMissTarget {
    pub guid: ObjectGuid,
    pub reason: u8,
    /// Result of the reflected cast on the original caster. The client reads this extra
    /// byte only when `reason` is `SPELL_MISS_REFLECT` (11); it is ignored otherwise.
    pub reflect_result: u8,
}

/// SPELL_MISS_REFLECT — the only miss reason carrying a trailing result byte.
const SPELL_MISS_REFLECT: u8 = 11;

// CAST_FLAG_UNKNOWN7: signals that the first packed GUID is a cast item, not
// the caster. The client uses this to track which item was used.
const CAST_FLAG_ITEM_CAST: u16 = 0x0040;

impl ToWorldPacket for SmsgSpellGo {
    fn to_vanilla(&self) -> WorldPacket {
        const CAST_FLAG_AMMO: u16 = 0x0020;
        let mut packet = WorldPacket::new(Opcode::SMSG_SPELL_GO);
        // First packed GUID: item GUID if cast from an item, else caster GUID
        packet.write_packed_guid(self.cast_item_guid.unwrap_or(self.caster_guid));
        packet.write_packed_guid(self.caster_guid_pack);
        packet.write_u32(self.spell_id);
        let cast_flags = if self.cast_item_guid.is_some() {
            self.cast_flags | CAST_FLAG_ITEM_CAST
        } else {
            self.cast_flags
        };
        packet.write_u16(cast_flags);

        // Hit targets
        packet.write_u8(self.hit_targets.len() as u8);
        for target in &self.hit_targets {
            packet.write_guid(*target); // Full 8-byte GUIDs in hit list
        }

        // Miss targets
        packet.write_u8(self.miss_targets.len() as u8);
        for target in &self.miss_targets {
            packet.write_guid(target.guid); // Full 8-byte GUID
            packet.write_u8(target.reason);
            if target.reason == SPELL_MISS_REFLECT {
                packet.write_u8(target.reflect_result);
            }
        }

        // SpellCastTargets section (vanilla 1.12.x requires this)
        write_spell_cast_targets(&mut packet, self.target_guid);

        // Ammo projectile info is present only for ranged casts.
        if cast_flags & CAST_FLAG_AMMO != 0 {
            packet.write_u32(self.ammo_display_id);
            packet.write_u32(self.ammo_inventory_type);
        }

        tracing::info!("[SPELL_GO] spell={} caster={:?} item={:?} target={:?} cast_flags=0x{:04X} hits={} misses={} packet_len={} bytes={:02X?}",
            self.spell_id, self.caster_guid, self.cast_item_guid, self.target_guid, cast_flags,
            self.hit_targets.len(), self.miss_targets.len(),
            packet.data().as_ref().len(), packet.data().as_ref());
        packet
    }

    /// For build 42597: the shared cast
    /// block, then a `HasLogData` bit and a flush.
    ///
    /// Note the flush comes *after* the optional log data, so an absent log block
    /// still costs the byte the bit is padded into — dropping it would leave the client one byte
    /// short.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        write_modern_spell_cast_data(
            &mut writer,
            self.caster_guid,
            self.cast_item_guid,
            self.spell_id,
            self.spell_visual_id,
            0, // SPELL_GO carries no cast time; the cast has already completed
            self.target_guid,
            &self.hit_targets,
            &self.miss_targets,
            self.cast_sequence,
        );
        writer.write_bit(false); // HasLogData
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_SPELL_GO))
    }
}

/// SMSG_PLAY_SPELL_VISUAL (opcode 0x01F3)
/// Plays a SpellVisualKit animation on a unit, visible to nearby players.
pub struct SmsgPlaySpellVisual {
    pub caster_guid: ObjectGuid,
    pub spell_visual_kit_id: u32,
}

impl ToWorldPacket for SmsgPlaySpellVisual {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_PLAY_SPELL_VISUAL);
        packet.write_u64(self.caster_guid.raw());
        packet.write_u32(self.spell_visual_kit_id);
        packet
    }

    /// Note the opcode changes message: vanilla's `SMSG_PLAY_SPELL_VISUAL` carries a visual *kit* id,
    /// so the proxy forwards it as 1.14's `SMSG_PLAY_SPELL_VISUAL_KIT` rather than the
    /// similarly-named `SMSG_PLAY_SPELL_VISUAL`. The opcode table
    /// records that under the vanilla name.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.caster_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // Unit
        writer.write_u32(self.spell_visual_kit_id); // KitRecID
        writer.write_u32(0); // KitType
        writer.write_u32(0); // Duration -- 0 lets the kit decide
        writer.write_bit(false); // MountedVisual
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_PLAY_SPELL_VISUAL))
    }
}

#[cfg(test)]
mod spell_packet_tests {
    use super::*;
    use crate::protocol::HighGuid;
    use bytes::Buf;

    fn player_guid(id: u32) -> ObjectGuid {
        ObjectGuid::new_without_entry(HighGuid::Player, id)
    }

    fn item_guid(id: u32) -> ObjectGuid {
        ObjectGuid::new_without_entry(HighGuid::Item, id)
    }

    fn read_packed_guid(data: &mut bytes::BytesMut) -> u64 {
        // packed GUID: mask byte, then one byte per set bit
        let mask = data[0] as u64;
        let mut result = 0u64;
        let mut pos = 1usize;
        for bit in 0..8u64 {
            if mask & (1 << bit) != 0 {
                result |= (data[pos] as u64) << (bit * 8);
                pos += 1;
            }
        }
        // advance past the packed guid bytes
        data.advance(pos);
        result
    }

    /// Byte length of the SpellCastTargets tail when there is no explicit target:
    /// a single TARGET_FLAG_SELF u16.
    fn empty_spell_cast_targets_len() -> usize {
        2
    }

    fn read_u16_le(data: &mut bytes::BytesMut) -> u16 {
        let v = u16::from_le_bytes([data[0], data[1]]);
        data.advance(2);
        v
    }

    // ===== SMSG_SPELL_GO tests =====

    /// When cast_item_guid is None the packet opens with the caster packed GUID
    /// and cast_flags has no CAST_FLAG_ITEM_CAST bit set.
    #[test]
    fn test_spell_go_no_item_uses_caster_guid() {
        let caster = player_guid(10);
        let msg = SmsgSpellGo {
            caster_guid: caster,
            caster_guid_pack: caster,
            spell_id: 1234,
            spell_visual_id: 0,
            cast_flags: 0x0100,
            hit_targets: vec![],
            miss_targets: vec![],
            target_guid: None,
            cast_item_guid: None,
            ammo_display_id: 0,
            ammo_inventory_type: 0,
            cast_sequence: 0,
        };
        let pkt = msg.to_vanilla();
        let mut data = pkt.data().clone();

        let first_guid = read_packed_guid(&mut data);
        assert_eq!(
            first_guid,
            caster.raw(),
            "first GUID must be caster when no item"
        );

        let _second_guid = read_packed_guid(&mut data);
        let _spell_id = {
            let bytes = [data[0], data[1], data[2], data[3]];
            data.advance(4);
            u32::from_le_bytes(bytes)
        };
        let flags = read_u16_le(&mut data);
        assert_eq!(
            flags & 0x0040,
            0,
            "CAST_FLAG_ITEM_CAST must NOT be set without an item"
        );
    }

    /// When cast_item_guid is Some the packet opens with the item packed GUID
    /// and CAST_FLAG_ITEM_CAST (0x0040) is OR'd into cast_flags.
    /// This is what prevents the 1.12 client from sending CMSG_DESTROYITEM.
    #[test]
    fn test_spell_go_with_item_uses_item_guid_and_sets_flag() {
        let caster = player_guid(10);
        let item = item_guid(42);
        let msg = SmsgSpellGo {
            caster_guid: caster,
            caster_guid_pack: caster,
            spell_id: 8690,
            spell_visual_id: 0,
            cast_flags: 0x0002,
            hit_targets: vec![],
            miss_targets: vec![],
            target_guid: None,
            cast_item_guid: Some(item),
            ammo_display_id: 0,
            ammo_inventory_type: 0,
            cast_sequence: 0,
        };
        let pkt = msg.to_vanilla();
        let mut data = pkt.data().clone();

        let first_guid = read_packed_guid(&mut data);
        assert_eq!(
            first_guid,
            item.raw(),
            "first GUID must be item GUID when cast from item"
        );

        let _second_guid = read_packed_guid(&mut data);
        let _spell_id = {
            let bytes = [data[0], data[1], data[2], data[3]];
            data.advance(4);
            u32::from_le_bytes(bytes)
        };
        let flags = read_u16_le(&mut data);
        assert_ne!(
            flags & 0x0040,
            0,
            "CAST_FLAG_ITEM_CAST (0x0040) must be set when cast from item"
        );
        assert_ne!(
            flags & 0x0002,
            0,
            "original cast_flags bits must be preserved"
        );
    }

    #[test]
    fn test_spell_go_serializes_each_miss_reason() {
        let caster = player_guid(10);
        let missed = player_guid(20);
        let msg = SmsgSpellGo {
            caster_guid: caster,
            caster_guid_pack: caster,
            spell_id: 1234,
            spell_visual_id: 0,
            cast_flags: 0,
            hit_targets: vec![],
            miss_targets: vec![SpellMissTarget {
                guid: missed,
                reason: 7,
                reflect_result: 0,
            }],
            target_guid: None,
            cast_item_guid: None,
            ammo_display_id: 0,
            ammo_inventory_type: 0,
            cast_sequence: 0,
        };
        let mut data = msg.to_vanilla().data().clone();

        let _ = read_packed_guid(&mut data);
        let _ = read_packed_guid(&mut data);
        data.advance(4 + 2);
        assert_eq!(data.get_u8(), 0); // hit targets
        assert_eq!(data.get_u8(), 1); // miss targets
        assert_eq!(data.get_u64_le(), missed.raw());
        assert_eq!(data.get_u8(), 7);
        // Only the (empty) SpellCastTargets tail is left: a non-reflect miss writes no
        // reflect-result byte.
        let tail_len = data.remaining();
        assert_eq!(
            tail_len,
            empty_spell_cast_targets_len(),
            "a non-reflect miss must not emit a reflect-result byte"
        );
    }

    /// A reflected target carries an extra reflect-result byte after the miss reason,
    /// and only that target does.
    #[test]
    fn test_spell_go_reflect_miss_appends_reflect_result() {
        let caster = player_guid(10);
        let reflector = player_guid(20);
        let immune = player_guid(30);
        let msg = SmsgSpellGo {
            caster_guid: caster,
            caster_guid_pack: caster,
            spell_id: 1234,
            spell_visual_id: 0,
            cast_flags: 0,
            hit_targets: vec![],
            miss_targets: vec![
                SpellMissTarget {
                    guid: reflector,
                    reason: SPELL_MISS_REFLECT,
                    reflect_result: 0, // SPELL_MISS_NONE: the reflected spell landed
                },
                SpellMissTarget {
                    guid: immune,
                    reason: 7,
                    reflect_result: 0,
                },
            ],
            target_guid: None,
            cast_item_guid: None,
            ammo_display_id: 0,
            ammo_inventory_type: 0,
            cast_sequence: 0,
        };
        let mut data = msg.to_vanilla().data().clone();

        let _ = read_packed_guid(&mut data);
        let _ = read_packed_guid(&mut data);
        data.advance(4 + 2);
        assert_eq!(data.get_u8(), 0); // hit targets
        assert_eq!(data.get_u8(), 2); // miss targets

        assert_eq!(data.get_u64_le(), reflector.raw());
        assert_eq!(data.get_u8(), SPELL_MISS_REFLECT);
        assert_eq!(
            data.get_u8(),
            0,
            "reflect result byte follows a reflect miss"
        );

        assert_eq!(data.get_u64_le(), immune.raw());
        assert_eq!(data.get_u8(), 7);
    }

    #[test]
    fn test_spell_go_writes_ammo_only_with_ammo_flag() {
        let caster = player_guid(10);
        let go = |cast_flags| SmsgSpellGo {
            caster_guid: caster,
            caster_guid_pack: caster,
            spell_id: 8690,
            spell_visual_id: 0,
            cast_flags,
            hit_targets: vec![],
            miss_targets: vec![],
            target_guid: None,
            cast_item_guid: None,
            ammo_display_id: 123,
            ammo_inventory_type: 26,
            cast_sequence: 0,
        };

        let without_ammo = go(0x0002).to_vanilla();
        let with_ammo = go(0x0022).to_vanilla();

        assert_eq!(with_ammo.data().len(), without_ammo.data().len() + 8);
    }

    // ===== SMSG_SPELL_START tests =====

    /// Without an item, SMSG_SPELL_START opens with the caster's packed GUID.
    #[test]
    fn test_spell_start_no_item_uses_caster_guid() {
        let caster = player_guid(10);
        let msg = SmsgSpellStart {
            caster_guid: caster,
            caster_guid_pack: caster,
            spell_id: 8690,
            spell_visual_id: 0,
            cast_flags: 0x0002,
            cast_time_ms: 10000,
            target_guid: None,
            cast_item_guid: None,
            ammo_display_id: 0,
            ammo_inventory_type: 0,
            cast_sequence: 0,
        };
        let pkt = msg.to_vanilla();
        let mut data = pkt.data().clone();

        let first_guid = read_packed_guid(&mut data);
        assert_eq!(
            first_guid,
            caster.raw(),
            "first GUID must be caster when no item"
        );
    }

    /// With an item, SMSG_SPELL_START opens with the item's packed GUID.
    #[test]
    fn test_spell_start_with_item_uses_item_guid() {
        let caster = player_guid(10);
        let item = item_guid(42);
        let msg = SmsgSpellStart {
            caster_guid: caster,
            caster_guid_pack: caster,
            spell_id: 8690,
            spell_visual_id: 0,
            cast_flags: 0x0002,
            cast_time_ms: 10000,
            target_guid: None,
            cast_item_guid: Some(item),
            ammo_display_id: 0,
            ammo_inventory_type: 0,
            cast_sequence: 0,
        };
        let pkt = msg.to_vanilla();
        let mut data = pkt.data().clone();

        let first_guid = read_packed_guid(&mut data);
        assert_eq!(
            first_guid,
            item.raw(),
            "first GUID must be item GUID when cast from item"
        );
    }

    #[test]
    fn test_spell_start_writes_ammo_only_with_ammo_flag() {
        let caster = player_guid(10);
        let start = |cast_flags| SmsgSpellStart {
            caster_guid: caster,
            caster_guid_pack: caster,
            spell_id: 8690,
            spell_visual_id: 0,
            cast_flags,
            cast_time_ms: 10000,
            target_guid: None,
            cast_item_guid: None,
            ammo_display_id: 123,
            ammo_inventory_type: 26,
            cast_sequence: 0,
        };

        let without_ammo = start(0x0002).to_vanilla();
        let with_ammo = start(0x0022).to_vanilla();

        assert_eq!(with_ammo.data().len(), without_ammo.data().len() + 8);
    }

    #[test]
    fn test_spell_failed_other_contains_caster_and_spell() {
        let caster = player_guid(10);
        let packet = SmsgSpellFailedOther {
            caster_guid: caster,
            spell_id: 8690,
        }
        .to_vanilla();

        assert_eq!(packet.opcode(), Opcode::SMSG_SPELL_FAILED_OTHER);
        let mut data = packet.data().clone();
        assert_eq!(data.get_u64_le(), caster.raw());
        assert_eq!(data.get_u32_le(), 8690);
        assert!(data.is_empty());
    }

    #[test]
    fn test_spell_failure_contains_interrupted_result() {
        let caster = player_guid(10);
        let packet = SmsgSpellFailure {
            caster_guid: caster,
            spell_id: 8690,
            result: 0x23,
        }
        .to_vanilla();

        assert_eq!(packet.opcode(), Opcode::SMSG_SPELL_FAILURE);
        let mut data = packet.data().clone();
        assert_eq!(data.get_u64_le(), caster.raw());
        assert_eq!(data.get_u32_le(), 8690);
        assert_eq!(data.get_u8(), 0x23);
        assert!(data.is_empty());
    }

    #[test]
    fn test_channel_update_contains_remaining_duration() {
        let packet = MsgChannelUpdate {
            remaining_ms: 1200,
            caster_guid: player_guid(1),
        }
        .to_vanilla();
        assert_eq!(packet.opcode(), Opcode::MSG_CHANNEL_UPDATE);
        let mut data = packet.data().clone();
        assert_eq!(data.get_u32_le(), 1200);
        assert!(data.is_empty());
    }
}

/// Write the SpellCastTargets section to a spell packet for vanilla 1.12.x.
fn write_spell_cast_targets(packet: &mut WorldPacket, target_guid: Option<ObjectGuid>) {
    match target_guid {
        Some(guid) if guid.raw() != 0 => {
            // TARGET_FLAG_UNIT
            packet.write_u16(0x0002);
            packet.write_packed_guid(guid);
        }
        _ => {
            // TARGET_FLAG_SELF (0x0000) — no additional data
            packet.write_u16(0x0000);
        }
    }
}

/// SMSG_SPELL_FAILURE (opcode 0x0133)
/// Broadcast when a spell cast fails or is interrupted.
pub struct SmsgSpellFailure {
    pub caster_guid: ObjectGuid,
    pub spell_id: u32,
    pub result: u8,
}

impl ToWorldPacket for SmsgSpellFailure {
    /// For build 42597: the caster,
    /// a cast identity, the spell, its visual, then the reason **widened to u16**.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        self.write_modern_prefix(&mut writer);
        writer.write_u16(u16::from(self.result));
        Some(writer.finish(Opcode::SMSG_SPELL_FAILURE))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SPELL_FAILURE);
        packet.write_guid(self.caster_guid);
        packet.write_u32(self.spell_id);
        packet.write_u8(self.result);
        packet
    }
}

impl SmsgSpellFailure {
    fn write_modern_prefix(&self, writer: &mut BitWriter) {
        let (high, low) = self.caster_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // CasterUnit
        let (high, low) = cast_guid128(DEFAULT_REALM_ID, 0, self.spell_id, next_cast_sequence());
        writer.write_packed_guid_128(high, low); // CastID
        writer.write_u32(self.spell_id);
        writer.write_u32(0); // SpellXSpellVisualID -- no 1.12 source
    }
}

/// SMSG_SPELL_FAILED_OTHER (opcode 0x02A6)
/// Broadcast when other clients need to stop displaying a caster's spell.
pub struct SmsgSpellFailedOther {
    pub caster_guid: ObjectGuid,
    pub spell_id: u32,
}

/// MSG_CHANNEL_UPDATE (opcode 0x013A)
/// Updates the remaining duration of a channeled spell; zero interrupts it.
pub struct MsgChannelUpdate {
    pub remaining_ms: u32,
    /// The channelling unit. Vanilla leaves it implicit — the packet goes only to the caster — but
    /// 1.14 names it, so observers can be told about someone else's channel.
    pub caster_guid: ObjectGuid,
}

impl ToWorldPacket for MsgChannelUpdate {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::MSG_CHANNEL_UPDATE);
        packet.write_u32(self.remaining_ms);
        packet
    }

    /// 1.14 names the channelling unit,
    /// where vanilla relies on the packet only ever going to the caster.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.caster_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_i32(self.remaining_ms as i32);
        Some(writer.finish(Opcode::MSG_CHANNEL_UPDATE))
    }
}

/// SMSG_SPELL_UPDATE_CHAIN_TARGETS for channeled spell beam visuals.
pub struct SmsgSpellUpdateChainTargets {
    pub caster_guid: ObjectGuid,
    pub spell_id: u32,
    pub targets: Vec<ObjectGuid>,
}

impl ToWorldPacket for SmsgSpellUpdateChainTargets {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SPELL_UPDATE_CHAIN_TARGETS);
        packet.write_guid(self.caster_guid);
        packet.write_u32(self.spell_id);
        packet.write_u32(self.targets.len() as u32);
        for target in &self.targets {
            packet.write_guid(*target);
        }
        packet
    }
}

impl ToWorldPacket for SmsgSpellFailedOther {
    /// Identical to `SMSG_SPELL_FAILURE`
    /// except the reason stays a u8. Vanilla carries no reason at all here, so zero it.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.caster_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low); // CasterUnit
        let (high, low) = cast_guid128(DEFAULT_REALM_ID, 0, self.spell_id, next_cast_sequence());
        writer.write_packed_guid_128(high, low); // CastID
        writer.write_u32(self.spell_id);
        writer.write_u32(0); // SpellXSpellVisualID
        writer.write_u8(0); // Reason -- vanilla sends none
        Some(writer.finish(Opcode::SMSG_SPELL_FAILED_OTHER))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SPELL_FAILED_OTHER);
        packet.write_guid(self.caster_guid);
        packet.write_u32(self.spell_id);
        packet
    }
}

/// SMSG_CAST_RESULT (opcode 0x0130)
/// Sent to the caster with the result of their cast attempt.
pub struct SmsgCastResult;

/// SPELL_RESULT_STATUS_OKAY — cast accepted (or failure suppressed).
pub const SPELL_RESULT_STATUS_OKAY: u8 = 0;
/// SPELL_RESULT_STATUS_FAIL — cast rejected; a failure reason (and optional args) follow.
pub const SPELL_RESULT_STATUS_FAIL: u8 = 2;

impl SmsgCastResult {
    /// General builder for the CastResult body:
    /// `spell_id(u32) + result(u8)`, and on FAIL `failure_reason(u8)` followed by
    /// `arg1(u32)` when either argument is present, then `arg2(u32)` when present.
    pub fn build(
        spell_id: u32,
        result_status: u8,
        failure_reason: u8,
        arg1: Option<u32>,
        arg2: Option<u32>,
    ) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_CAST_RESULT);
        packet.write_u32(spell_id);
        packet.write_u8(result_status);
        if result_status == SPELL_RESULT_STATUS_FAIL {
            packet.write_u8(failure_reason);
            // The client reads a single arg1 if *either* optional is present.
            if arg1.is_some() || arg2.is_some() {
                packet.write_u32(arg1.unwrap_or(0));
            }
            if let Some(a2) = arg2 {
                packet.write_u32(a2);
            }
        }
        packet
    }

    /// Build a success SMSG_CAST_RESULT: spell_id(u32) + status(u8=0)
    pub fn success(spell_id: u32) -> WorldPacket {
        Self::build(spell_id, SPELL_RESULT_STATUS_OKAY, 0, None, None)
    }

    /// Build a plain failure SMSG_CAST_RESULT with no extra arguments.
    pub fn failure(spell_id: u32, error_code: u8) -> WorldPacket {
        Self::build(spell_id, SPELL_RESULT_STATUS_FAIL, error_code, None, None)
    }
}

/// SMSG_CAST_FAILED (1.14 build 42597 wire number 0x2C57, the same value our table calls
/// `SMSG_CAST_RESULT`)
///
/// The modern failure body is a different shape from vanilla's `SMSG_CAST_RESULT`: it leads with
/// the cast's `CastID` (the same id the client's pending press was acknowledged under, see
/// [`SmsgSpellPrepare`]), then the spell, its visual, the reason widened to u32, and the two
/// optional failure arguments as signed i32 always present (`-1` = none). `SmsgCastResult` can't
/// express that — it builds a vanilla-shaped body and is dropped unported on the modern wire — so
/// a modern rejection leaves the client's pending press unresolved and the cast bar/button stuck.
/// This type exists to send a real modern failure.
pub struct SmsgCastFailed {
    /// The cast identity this failure resolves. Must be the id the client's pending press was
    /// linked to: the server's own cast id if `SMSG_SPELL_PREPARE` was sent, or the client's own
    /// cast id when rejecting before any PREPARE (see JimsProxy's `SendCastFailedWithoutPrepare`).
    pub cast_id: (u64, u64),
    pub spell_id: u32,
    pub failure_reason: u8,
    pub arg1: Option<u32>,
    pub arg2: Option<u32>,
}

impl ToWorldPacket for SmsgCastFailed {
    /// Reuse the vanilla `SMSG_CAST_RESULT` shape (spell + status + reason + optional args).
    fn to_vanilla(&self) -> WorldPacket {
        SmsgCastResult::build(
            self.spell_id,
            SPELL_RESULT_STATUS_FAIL,
            self.failure_reason,
            self.arg1,
            self.arg2,
        )
    }

    /// `CastID(guid128) + SpellID(u32) + SpellXSpellVisualID(u32) + Reason(u32) + FailedArg1(i32) +
    /// FailedArg2(i32)`, per JimsProxy's `CastFailed`. Both args are always written; absent ones are
    /// `-1`.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.cast_id;
        writer.write_packed_guid_128(high, low); // CastID
        writer.write_u32(self.spell_id);
        writer.write_u32(0); // SpellXSpellVisualID -- no 1.12 source
        writer.write_u32(u32::from(self.failure_reason)); // Reason
        writer.write_i32(self.arg1.map(|v| v as i32).unwrap_or(-1)); // FailedArg1
        writer.write_i32(self.arg2.map(|v| v as i32).unwrap_or(-1)); // FailedArg2
        Some(writer.finish(Opcode::SMSG_CAST_RESULT))
    }
}

/// SMSG_SPELL_COOLDOWN (opcode 0x0134)
/// Sent when spells go on cooldown.
/// Vanilla 1.12.x format: packed_guid + [spell_id(u32) + cooldown_ms(u32)]...
/// No flags byte, no count — client reads entries until packet ends.
pub struct SmsgSpellCooldown {
    pub caster_guid: ObjectGuid,
    pub cooldowns: Vec<(u32, u32)>, // (spell_id, cooldown_ms)
}

impl ToWorldPacket for SmsgSpellCooldown {
    /// Vanilla streams `(spell, ms)` pairs to the end of the packet with no count; 1.14 prefixes a
    /// flags byte and an explicit count, and each entry gains a `ModRate` float. `ModRate` is the
    /// cooldown-rate multiplier and **must be 1.0, not 0.0** — the client divides by it, so a zero
    /// makes every cooldown either infinite or NaN.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.caster_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_u8(0); // Flags
        writer.write_i32(self.cooldowns.len() as i32);
        for (spell_id, cooldown_ms) in &self.cooldowns {
            writer.write_u32(*spell_id);
            writer.write_u32(*cooldown_ms); // ForcedCooldown
            writer.write_f32(1.0); // ModRate -- a divisor; see above
        }
        Some(writer.finish(Opcode::SMSG_SPELL_COOLDOWN))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SPELL_COOLDOWN);
        packet.write_packed_guid(self.caster_guid);
        for (spell_id, cooldown_ms) in &self.cooldowns {
            packet.write_u32(*spell_id);
            packet.write_u32(*cooldown_ms);
        }
        tracing::info!(
            "[SPELL_COOLDOWN] caster={:?} cooldowns={:?} packet_len={} bytes={:02X?}",
            self.caster_guid,
            self.cooldowns,
            packet.data().as_ref().len(),
            packet.data().as_ref()
        );
        packet
    }
}

/// SMSG_INITIAL_SPELLS (opcode 0x012A)
/// Sent on login with the full spellbook and active cooldowns.
pub struct SmsgInitialSpells {
    pub cast_count: u8,
    pub spells: Vec<u32>,
    pub cooldowns: Vec<InitialSpellCooldown>,
}

impl ToWorldPacket for SmsgInitialSpells {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_INITIAL_SPELLS);
        packet.write_u8(self.cast_count);
        packet.write_u16(self.spells.len() as u16);
        for spell_id in &self.spells {
            packet.write_u16(*spell_id as u16);
            packet.write_u16(0); // unknown
        }
        packet.write_u16(self.cooldowns.len() as u16);
        for cd in &self.cooldowns {
            packet.write_u32(cd.spell_id);
            packet.write_u16(cd.item_id);
            packet.write_u16(cd.category);
            packet.write_u32(cd.spell_cooldown_ms);
            packet.write_u32(cd.category_cooldown_ms);
        }
        packet
    }

    /// See [`crate::messages::login::SmsgInitialSpellsRef::to_modern`] — same body. Cooldowns are
    /// not part of the modern message and are dropped; the client gets them from
    /// `SMSG_SPELL_COOLDOWN`.
    fn to_modern(&self) -> Option<WorldPacket> {
        crate::messages::login::write_modern_known_spells(&self.spells).into()
    }
}

pub struct InitialSpellCooldown {
    pub spell_id: u32,
    pub item_id: u16,
    pub category: u16,
    pub spell_cooldown_ms: u32,
    pub category_cooldown_ms: u32,
}

/// SMSG_LEARNED_SPELL (opcode 0x012B)
pub struct SmsgLearnedSpell {
    pub spell_id: u32,
}

impl ToWorldPacket for SmsgLearnedSpell {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LEARNED_SPELL);
        packet.write_u32(self.spell_id);
        packet.write_u16(0); // unknown
        packet
    }

    /// 1.14 sends a *list*, so a single learned
    /// spell is a list of one, plus an empty favourites list and a specialization id Classic has no
    /// concept of.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_i32(1); // Spells count
        writer.write_i32(0); // FavoriteSpellID count
        writer.write_u32(0); // SpecializationID
        writer.write_u32(self.spell_id);
        writer.write_bit(false); // SuppressMessaging -- let the client announce the new spell
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_LEARNED_SPELL))
    }
}

/// SMSG_REMOVED_SPELL (opcode 0x0203)
pub struct SmsgRemovedSpell {
    pub spell_id: u32,
}

impl ToWorldPacket for SmsgRemovedSpell {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_REMOVED_SPELL);
        packet.write_u32(self.spell_id);
        packet
    }

    /// 1.14 calls this message UnlearnedSpells. A count,
    /// the list, then a suppress-messaging bit.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_i32(1); // Spells count
        writer.write_u32(self.spell_id);
        writer.write_bit(false); // SuppressMessaging
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_REMOVED_SPELL))
    }
}

/// SMSG_CLEAR_COOLDOWN (opcode 0x01DE)
pub struct SmsgClearCooldown {
    pub spell_id: u32,
    pub caster_guid: ObjectGuid,
}

impl ToWorldPacket for SmsgClearCooldown {
    /// The caster GUID vanilla carries is gone:
    /// 1.14 sends this only to the owner, and distinguishes pet cooldowns with a bit instead.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u32(self.spell_id);
        writer.write_bit(false); // ClearOnHold
        writer.write_bit(false); // IsPet
        writer.flush_bits();
        Some(writer.finish(Opcode::SMSG_CLEAR_COOLDOWN))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_CLEAR_COOLDOWN);
        packet.write_u32(self.spell_id);
        packet.write_guid(self.caster_guid);
        packet
    }
}

/// SMSG_SPELL_DELAYED (opcode 0x0135)
/// Sent when a spell cast is delayed (e.g., from pushback).
pub struct SmsgSpellDelayed {
    pub caster_guid: ObjectGuid,
    pub delay_ms: u32,
}

impl ToWorldPacket for SmsgSpellDelayed {
    /// The same two fields, guid128 and i32.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        let (high, low) = self.caster_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_i32(self.delay_ms as i32);
        Some(writer.finish(Opcode::SMSG_SPELL_DELAYED))
    }

    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SPELL_DELAYED);
        packet.write_guid(self.caster_guid);
        packet.write_u32(self.delay_ms);
        packet
    }
}

/// Per-effect execute-log data for SMSG_SPELLLOGEXECUTE.
#[derive(Debug, Clone)]
pub struct ExecuteLogInfo {
    pub target_guid: ObjectGuid,
    pub data: ExecuteLogData,
}

/// Per-effect-type payload for SMSG_SPELLLOGEXECUTE.
#[derive(Debug, Clone)]
pub enum ExecuteLogData {
    PowerDrain {
        amount: u32,
        power: u32,
        multiplier: f32,
    },
    Heal {
        amount: u32,
        critical: bool,
    },
    Energize {
        amount: u32,
        power_type: u32,
    },
    ExtraAttacks {
        count: u32,
    },
    CreateItem {
        item_entry: u32,
    },
    InterruptCast {
        spell_id: u32,
    },
    FeedPet {
        item_entry: u32,
    },
    DurabilityDamage {
        item_entry: i32,
        unk: i32,
    },
    TargetOnly,
}

/// SMSG_SPELLLOGEXECUTE (opcode 0x024C)
/// Sent after a cast completes when the spell has non-trivial side effects
/// that the client needs to be told about (summons, dispels, drains, etc.).
pub struct SmsgSpellLogExecute;

impl SmsgSpellLogExecute {
    /// Build the packet. Returns `None` when every effect index has an empty log
    /// (nothing is sent in that case).
    pub fn build(
        caster_guid: ObjectGuid,
        spell_id: u32,
        effect_types: [u32; 3],
        execute_log: [&[ExecuteLogInfo]; 3],
    ) -> Option<WorldPacket> {
        let effect_count = execute_log.iter().filter(|v| !v.is_empty()).count();
        if effect_count == 0 {
            return None;
        }

        let mut packet = WorldPacket::new(Opcode::SMSG_SPELLLOGEXECUTE);
        // Vanilla 1.12 > 1.8.4 — uses packed GUIDs
        packet.write_packed_guid(caster_guid);
        packet.write_u32(spell_id);
        packet.write_u32(effect_count as u32);

        for i in 0..3 {
            let entries = execute_log[i];
            if entries.is_empty() {
                continue;
            }

            packet.write_u32(effect_types[i]);
            packet.write_u32(entries.len() as u32);

            for info in entries {
                match &info.data {
                    ExecuteLogData::PowerDrain {
                        amount,
                        power,
                        multiplier,
                    } => {
                        packet.write_guid(info.target_guid);
                        packet.write_u32(*amount);
                        packet.write_u32(*power);
                        packet.write_f32(*multiplier);
                    }
                    ExecuteLogData::Heal { amount, critical } => {
                        packet.write_guid(info.target_guid);
                        packet.write_u32(*amount);
                        packet.write_u8(if *critical { 1 } else { 0 });
                    }
                    ExecuteLogData::Energize { amount, power_type } => {
                        packet.write_guid(info.target_guid);
                        packet.write_u32(*amount);
                        packet.write_u32(*power_type);
                    }
                    ExecuteLogData::ExtraAttacks { count } => {
                        packet.write_guid(info.target_guid);
                        packet.write_u32(*count);
                    }
                    ExecuteLogData::CreateItem { item_entry } => {
                        packet.write_u32(*item_entry);
                    }
                    ExecuteLogData::InterruptCast { spell_id } => {
                        packet.write_guid(info.target_guid);
                        packet.write_u32(*spell_id);
                    }
                    ExecuteLogData::FeedPet { item_entry } => {
                        packet.write_u32(*item_entry);
                    }
                    ExecuteLogData::DurabilityDamage { item_entry, unk } => {
                        packet.write_guid(info.target_guid);
                        packet.write_u32(*item_entry as u32);
                        packet.write_u32(*unk as u32);
                    }
                    ExecuteLogData::TargetOnly => {
                        packet.write_guid(info.target_guid);
                    }
                }
            }
        }

        Some(packet)
    }
}

/// Result of a spell cast attempt (sent to client in SMSG_CAST_RESULT).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpellCastResult {
    Success = 0,
    FailureAffectingCombat = 1,
    AlreadyAtFullHealth = 2,
    AlreadyAtFullMana = 3,
    AlreadyAtFullPower = 4,
    AuraBounced = 5,
    AuraNotCancelled = 6,
    AlreadyHaveCharm = 7,
    AlreadyHaveSummon = 8,
    AlreadyHavePet = 9,
    AlreadyPickedThatTalent = 10,
    AmmoScrewedUp = 11,
    AuraInCombat = 12,
    CantBeCharmed = 13,
    CantBeDisenchanted = 14,
    CantBeDisenchantedSkill = 15,
    CantBeMilled = 16,
    CantBeOpened = 17,
    CantCastOnTapped = 18,
    CantDuelWhileStealthed = 19,
    CantStealth = 20,
    CasterAurastate = 21,
    CasterDead = 22,
    Charmed = 23,
    ChestInUse = 24,
    TargetConfused = 25,
    DontReport = 26,
    EquipItemOnlyShapeshift = 27,
    EquipNotReady = 28,
    FarsightFocus = 29,
    Fear = 30,
    Fleeing = 31,
    FoodLowlevel = 32,
    FoodHighlevel = 33,
    FriendlyTargetDuel = 34,
    FriendlyTarget = 35,
    GroundNotMounted = 36,
    Highlevel = 37,
    HungerSatiated = 38,
    Immune = 39,
    IncorrectArea = 40,
    Interrupted = 41,
    InterruptedCombat = 42,
    ItemAlreadyEnchanted = 43,
    ItemGone = 44,
    ItemNotFound = 45,
    ItemNotReady = 46,
    LevelRequirement = 47,
    LineOfSight = 48,
    LowCastlevel = 49,
    Lowlevel = 50,
    MainhandEmpty = 51,
    Moving = 52,
    NeedAmmo = 53,
    NeedAmmoPouch = 54,
    NeedExoticAmmo = 55,
    NeedMoreItems = 56,
    NoChampion = 57,
    NoComboPoints = 58,
    NoDueling = 59,
    NoEndurance = 60,
    NoFish = 61,
    NoItemsWhileShapeshifted = 62,
    NoMountsAllowed = 63,
    NoPet = 64,
    NoPower = 65,
    NothingToDispel = 66,
    NothingToSteal = 67,
    OnlyAbovewater = 68,
    OnlyIndoors = 69,
    OnlyMountedflying = 70,
    OnlyNighttime = 71,
    OnlyOutdoors = 72,
    OnlyShapeshift = 73,
    OnlyStealthed = 74,
    OnlyUnderwater = 75,
    OutOfRange = 76,
    Pacified = 77,
    Possessed = 78,
    Reagents = 79,
    RequiresArea = 80,
    RequiresSpellFocus = 81,
    Root = 82,
    Silenced = 83,
    SpellInProgress = 84,
    SpellLearned = 85,
    SpellUnavailable = 86,
    Stunned = 87,
    Submerged = 88,
    Swimming = 89,
    TargetAurastate = 90,
    TargetDueling = 91,
    TargetEnemy = 92,
    TargetEngaged = 93,
    TargetFleeing = 94,
    TargetInCombat = 95,
    TargetIsPet = 96,
    TargetNotDead = 97,
    TargetNotInParty = 98,
    TargetNotLooted = 99,
    TargetNotPlayer = 100,
    TargetNoPockets = 101,
    TargetNoWeapons = 102,
    TargetNoRangedWeapons = 103,
    TargetUnskinnable = 104,
    TargetTooFar = 105,
    ThirstSatiated = 106,
    TooClose = 107,
    TooManyOfItem = 108,
    TotemCategory = 109,
    Totems = 110,
    TrainingPoints = 111,
    TryAgain = 112,
    UnitNotBehind = 113,
    UnitNotInfront = 114,
    WrongPetFood = 115,
    NotWhileFatigued = 116,
    TargetNotInInstance = 117,
    AffectingCombat = 118,
    TargetFriendly = 119,
    TargetIsPlayerControlled = 120,
    TargetIsPlayer = 121,
    NpcCustomization = 122,
    AlreadyInGroup = 123,
    TargetNotInRaid = 124,
    Unknown = 125,
    // Additional results
    NotReady = 126,      // GCD active
    OnCooldown = 127,    // Spell on cooldown
    SchoolLockout = 128, // School locked from interrupt
    SpellNotKnown = 129, // Don't know the spell
    TargetNotInLineOfSight = 130,
    AlreadyCasting = 131,
    NotWhileGhost = 132,
    NotOnTaxi = 133,
    NoChargesRemain = 134,
    TargetNotGhost = 135,
}

impl SpellCastResult {
    /// Convert to the u8 value expected by the client.
    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

/// Spell schools in vanilla WoW.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpellSchool {
    Physical = 0,
    Holy = 1,
    Fire = 2,
    Nature = 3,
    Frost = 4,
    Shadow = 5,
    Arcane = 6,
}

/// Spell effect types (from Spell.dbc Effect field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SpellEffectType {
    None = 0,
    Instakill = 1,
    SchoolDamage = 2,
    Dummy = 3,
    PortalTeleport = 4,
    TeleportUnits = 5,
    ApplyAura = 6,
    EnvironmentalDamage = 7,
    PowerDrain = 8,
    HealthLeech = 9,
    Heal = 10,
    Bind = 11,
    Portal = 12,
    RitualBase = 13,
    RitualSpecialize = 14,
    RitualActivatePortal = 15,
    QuestComplete = 16,
    WeaponDamageNoschool = 17,
    Resurrect = 18,
    AddExtraAttacks = 19,
    Dodge = 20,
    Evade = 21,
    Parry = 22,
    Block = 23,
    CreateItem = 24,
    Weapon = 25,
    Defense = 26,
    PersistentAreaAura = 27,
    Summon = 28,
    Leap = 29,
    Energize = 30,
    WeaponPercentDamage = 31,
    TriggerMissile = 32,
    OpenLock = 33,
    SummonChangeItem = 34,
    ApplyPartyAura = 35,
    LearnSpell = 36,
    SpellDefense = 37,
    Dispel = 38,
    Language = 39,
    DualWield = 40,
    Jump = 41,
    JumpDest = 42,
    TeleportGraveyard = 43,
    IncreasedMovementSpeed = 44,
    IncreasedSwimSpeed = 45,
    DecreaseMovementSpeed = 46,
    DecreaseSwimSpeed = 47,
    Transform = 48,
    Fear = 49,
    FearGt = 50,
    CreateTamedPet = 51,
    SummonPet = 52,
    LearnPetSpell = 53,
    WeaponDamage = 54,
    CreateRandomItem = 55,
    Proficiency = 56,
    SendEvent = 57,
    PowerBurn = 58,
    Threat = 59,
    TriggerSpell = 60,
    ApplyRaidAura = 61,
    RestoreItemCharges = 62,
    HealMaxHealth = 63,
    InterruptCast = 64,
    Distract = 65,
    Pull = 66,
    Pickpocket = 67,
    AddFarsight = 68,
    UntrainTalents = 69,
    ApplyGlyph = 70,
    HealMechanical = 71,
    SummonObjectWild = 72,
    ScriptEffect = 73,
    Attack = 74,
    Sanctuary = 75,
    AddComboPoints = 76,
    PushAbilityToActionBar = 77,
    BindSight = 78,
    Duplicity = 79,
    Stealth = 80,
    Detect = 81,
    SummonObject = 82,
    SummonParty = 83,
    SummonRaid = 84,
    SummonFriend = 85,
    SummonCritter = 86,
    SummonPetComplex = 87,
    SummonObjectSlot1 = 88,
    SummonObjectSlot2 = 89,
    SummonObjectSlot3 = 90,
    SummonObjectSlot4 = 91,
    ThreatAll = 92,
    EnchantItem = 93,
    EnchantItemTemporary = 94,
    Tamecreature = 95,
    SummonPet2 = 96,
    LearnPetSpell2 = 97,
    WeaponDamageColossus = 98,
    CreateItem2 = 99,
    Milling = 100,
    AllowRenamePet = 101,
    AllowRenameParty = 102,
    SummonPet3 = 103,
    SummonFriend2 = 104,
    TeleportUnitsFaceCaster = 105,
    Skill = 106,
    ApplyPetAura = 107,
    DummyMelee = 108,
    DummyRanged = 109,
    Charge = 110,
    CastButton = 111,
    KnockBack = 112,
    Disenchant = 113,
    Inebriate = 114,
    FeedPet = 115,
    DismissPet = 116,
    Reputation = 117,
    SummonObjectSlot5 = 118,
    SummonObjectSlot6 = 119,
    SummonObjectSlot7 = 120,
    NormalizedWeaponDmg = 121,
    NormalizedWeaponDmgPlus = 122,
}

/// Convert u32 to SpellEffectType
pub fn spell_effect_type_from_u32(value: u32) -> Option<SpellEffectType> {
    match value {
        0 => Some(SpellEffectType::None),
        1 => Some(SpellEffectType::Instakill),
        2 => Some(SpellEffectType::SchoolDamage),
        3 => Some(SpellEffectType::Dummy),
        4 => Some(SpellEffectType::PortalTeleport),
        5 => Some(SpellEffectType::TeleportUnits),
        6 => Some(SpellEffectType::ApplyAura),
        7 => Some(SpellEffectType::EnvironmentalDamage),
        8 => Some(SpellEffectType::PowerDrain),
        9 => Some(SpellEffectType::HealthLeech),
        10 => Some(SpellEffectType::Heal),
        11 => Some(SpellEffectType::Bind),
        12 => Some(SpellEffectType::Portal),
        13 => Some(SpellEffectType::RitualBase),
        14 => Some(SpellEffectType::RitualSpecialize),
        15 => Some(SpellEffectType::RitualActivatePortal),
        16 => Some(SpellEffectType::QuestComplete),
        17 => Some(SpellEffectType::WeaponDamageNoschool),
        18 => Some(SpellEffectType::Resurrect),
        19 => Some(SpellEffectType::AddExtraAttacks),
        20 => Some(SpellEffectType::Dodge),
        21 => Some(SpellEffectType::Evade),
        22 => Some(SpellEffectType::Parry),
        23 => Some(SpellEffectType::Block),
        24 => Some(SpellEffectType::CreateItem),
        25 => Some(SpellEffectType::Weapon),
        26 => Some(SpellEffectType::Defense),
        27 => Some(SpellEffectType::PersistentAreaAura),
        28 => Some(SpellEffectType::Summon),
        29 => Some(SpellEffectType::Leap),
        30 => Some(SpellEffectType::Energize),
        31 => Some(SpellEffectType::WeaponPercentDamage),
        32 => Some(SpellEffectType::TriggerMissile),
        33 => Some(SpellEffectType::OpenLock),
        34 => Some(SpellEffectType::SummonChangeItem),
        35 => Some(SpellEffectType::ApplyPartyAura),
        36 => Some(SpellEffectType::LearnSpell),
        37 => Some(SpellEffectType::SpellDefense),
        38 => Some(SpellEffectType::Dispel),
        39 => Some(SpellEffectType::Language),
        40 => Some(SpellEffectType::DualWield),
        41 => Some(SpellEffectType::Jump),
        42 => Some(SpellEffectType::JumpDest),
        43 => Some(SpellEffectType::TeleportGraveyard),
        44 => Some(SpellEffectType::IncreasedMovementSpeed),
        45 => Some(SpellEffectType::IncreasedSwimSpeed),
        46 => Some(SpellEffectType::DecreaseMovementSpeed),
        47 => Some(SpellEffectType::DecreaseSwimSpeed),
        48 => Some(SpellEffectType::Transform),
        49 => Some(SpellEffectType::Fear),
        50 => Some(SpellEffectType::FearGt),
        51 => Some(SpellEffectType::CreateTamedPet),
        52 => Some(SpellEffectType::SummonPet),
        53 => Some(SpellEffectType::LearnPetSpell),
        54 => Some(SpellEffectType::WeaponDamage),
        55 => Some(SpellEffectType::CreateRandomItem),
        56 => Some(SpellEffectType::Proficiency),
        57 => Some(SpellEffectType::SendEvent),
        58 => Some(SpellEffectType::PowerBurn),
        59 => Some(SpellEffectType::Threat),
        60 => Some(SpellEffectType::TriggerSpell),
        61 => Some(SpellEffectType::ApplyRaidAura),
        62 => Some(SpellEffectType::RestoreItemCharges),
        63 => Some(SpellEffectType::HealMaxHealth),
        64 => Some(SpellEffectType::InterruptCast),
        65 => Some(SpellEffectType::Distract),
        66 => Some(SpellEffectType::Pull),
        67 => Some(SpellEffectType::Pickpocket),
        68 => Some(SpellEffectType::AddFarsight),
        69 => Some(SpellEffectType::UntrainTalents),
        70 => Some(SpellEffectType::ApplyGlyph),
        71 => Some(SpellEffectType::HealMechanical),
        72 => Some(SpellEffectType::SummonObjectWild),
        73 => Some(SpellEffectType::ScriptEffect),
        74 => Some(SpellEffectType::Attack),
        75 => Some(SpellEffectType::Sanctuary),
        76 => Some(SpellEffectType::AddComboPoints),
        77 => Some(SpellEffectType::PushAbilityToActionBar),
        78 => Some(SpellEffectType::BindSight),
        79 => Some(SpellEffectType::Duplicity),
        80 => Some(SpellEffectType::Stealth),
        81 => Some(SpellEffectType::Detect),
        82 => Some(SpellEffectType::SummonObject),
        83 => Some(SpellEffectType::SummonParty),
        84 => Some(SpellEffectType::SummonRaid),
        85 => Some(SpellEffectType::SummonFriend),
        86 => Some(SpellEffectType::SummonCritter),
        87 => Some(SpellEffectType::SummonPetComplex),
        88 => Some(SpellEffectType::SummonObjectSlot1),
        89 => Some(SpellEffectType::SummonObjectSlot2),
        90 => Some(SpellEffectType::SummonObjectSlot3),
        91 => Some(SpellEffectType::SummonObjectSlot4),
        92 => Some(SpellEffectType::ThreatAll),
        93 => Some(SpellEffectType::EnchantItem),
        94 => Some(SpellEffectType::EnchantItemTemporary),
        95 => Some(SpellEffectType::Tamecreature),
        96 => Some(SpellEffectType::SummonPet2),
        97 => Some(SpellEffectType::LearnPetSpell2),
        98 => Some(SpellEffectType::WeaponDamageColossus),
        99 => Some(SpellEffectType::CreateItem2),
        100 => Some(SpellEffectType::Milling),
        101 => Some(SpellEffectType::AllowRenamePet),
        102 => Some(SpellEffectType::AllowRenameParty),
        103 => Some(SpellEffectType::SummonPet3),
        104 => Some(SpellEffectType::SummonFriend2),
        105 => Some(SpellEffectType::TeleportUnitsFaceCaster),
        106 => Some(SpellEffectType::Skill),
        107 => Some(SpellEffectType::ApplyPetAura),
        108 => Some(SpellEffectType::DummyMelee),
        109 => Some(SpellEffectType::DummyRanged),
        110 => Some(SpellEffectType::Charge),
        111 => Some(SpellEffectType::CastButton),
        112 => Some(SpellEffectType::KnockBack),
        113 => Some(SpellEffectType::Disenchant),
        114 => Some(SpellEffectType::Inebriate),
        115 => Some(SpellEffectType::FeedPet),
        116 => Some(SpellEffectType::DismissPet),
        117 => Some(SpellEffectType::Reputation),
        118 => Some(SpellEffectType::SummonObjectSlot5),
        119 => Some(SpellEffectType::SummonObjectSlot6),
        120 => Some(SpellEffectType::SummonObjectSlot7),
        121 => Some(SpellEffectType::NormalizedWeaponDmg),
        122 => Some(SpellEffectType::NormalizedWeaponDmgPlus),
        _ => None,
    }
}

#[cfg(test)]
mod smsg_spell_log_execute_tests {
    use super::*;

    #[test]
    fn empty_log_returns_none() {
        let result =
            SmsgSpellLogExecute::build(ObjectGuid::from_raw(1), 1234, [0, 0, 0], [&[], &[], &[]]);
        assert!(result.is_none());
    }

    #[test]
    fn single_target_only_effect() {
        let guid = ObjectGuid::from_raw(0xC0_0000_0000_0001);
        let packet = SmsgSpellLogExecute::build(
            guid,
            1234,
            [64, 0, 0],
            [
                &[ExecuteLogInfo {
                    target_guid: guid,
                    data: ExecuteLogData::TargetOnly,
                }],
                &[],
                &[],
            ],
        )
        .expect("should build");
        let data = packet.data();
        let inner = data.as_ref();
        assert!(!inner.is_empty());
    }

    #[test]
    fn heal_log_encodes_correctly() {
        let caster = ObjectGuid::from_raw(0xC0_00_00_00_00_00_00_01);
        let target = ObjectGuid::from_raw(0xC0_00_00_00_00_00_00_02);
        let packet = SmsgSpellLogExecute::build(
            caster,
            4321,
            [8, 0, 0],
            [
                &[ExecuteLogInfo {
                    target_guid: target,
                    data: ExecuteLogData::Heal {
                        amount: 500,
                        critical: true,
                    },
                }],
                &[],
                &[],
            ],
        )
        .expect("should build");
        let data = packet.data();
        let inner = data.as_ref();
        assert!(!inner.is_empty());
    }

    #[test]
    fn power_drain_log() {
        let caster = ObjectGuid::from_raw(1);
        let target = ObjectGuid::from_raw(2);
        let packet = SmsgSpellLogExecute::build(
            caster,
            42,
            [32, 0, 0],
            [
                &[ExecuteLogInfo {
                    target_guid: target,
                    data: ExecuteLogData::PowerDrain {
                        amount: 100,
                        power: 0,
                        multiplier: 1.0,
                    },
                }],
                &[],
                &[],
            ],
        )
        .expect("should build");
        let inner = packet.data();
        let inner = inner.as_ref();
        assert!(!inner.is_empty());
    }
}

#[cfg(test)]
mod modern_spell_tests {
    use super::*;

    fn caster() -> ObjectGuid {
        ObjectGuid::new_player(4)
    }

    fn go(hits: Vec<ObjectGuid>, misses: Vec<SpellMissTarget>, cast_sequence: u64) -> WorldPacket {
        SmsgSpellGo {
            caster_guid: caster(),
            caster_guid_pack: caster(),
            spell_id: 133,
            spell_visual_id: 0,
            cast_flags: 0,
            hit_targets: hits,
            miss_targets: misses,
            target_guid: Some(ObjectGuid::new_creature(299, 464)),
            cast_item_guid: None,
            ammo_display_id: 0,
            ammo_inventory_type: 0,
            cast_sequence,
        }
        .to_modern()
        .expect("spell go must encode for modern")
    }

    /// Every cast needs a distinct `Cast` GUID: the client keys its cast bar, sounds and visual
    /// chunks on it, and a reused id makes visuals drift and interrupts fail to dismiss the bar.
    /// The message itself no longer guarantees this -- the caller must pass a fresh
    /// `next_cast_sequence()` value per cast, which this asserts produces distinct GUIDs.
    #[test]
    fn each_cast_gets_a_distinct_cast_guid() {
        let first = go(vec![], vec![], next_cast_sequence());
        let second = go(vec![], vec![], next_cast_sequence());
        assert_ne!(
            first.contents(),
            second.contents(),
            "two casts of the same spell must not share a cast GUID"
        );
    }

    /// The bug this fixes: `SmsgSpellStart::to_modern` and `SmsgSpellGo::to_modern` used to mint
    /// their own cast sequence independently, so a spell's START and GO never shared a `CastID`.
    /// The client opens its cast bar under START's id and expects GO to close that same bar --
    /// a mismatched id meant GO did not register as completing the cast START began.
    #[test]
    fn start_and_go_share_the_cast_id_when_given_the_same_sequence() {
        use crate::protocol::bitbuf::BitReader;

        let sequence = next_cast_sequence();
        let start = SmsgSpellStart {
            caster_guid: caster(),
            caster_guid_pack: caster(),
            spell_id: 133,
            spell_visual_id: 0,
            cast_flags: 0,
            cast_time_ms: 0,
            target_guid: Some(ObjectGuid::new_creature(299, 464)),
            cast_item_guid: None,
            ammo_display_id: 0,
            ammo_inventory_type: 0,
            cast_sequence: sequence,
        }
        .to_modern()
        .expect("spell start must encode for modern");
        let go = go(vec![], vec![], sequence);

        // The cast block opens with CasterGUID, CasterUnit, then CastID -- skip the first two to
        // reach the one that must match.
        let cast_id = |packet: &crate::protocol::WorldPacket| {
            let mut reader = BitReader::new(packet.contents());
            reader.read_packed_guid_128().expect("CasterGUID");
            reader.read_packed_guid_128().expect("CasterUnit");
            reader.read_packed_guid_128().expect("CastID")
        };
        assert_eq!(
            cast_id(&start),
            cast_id(&go),
            "SPELL_START and SPELL_GO for the same cast must carry the same CastID"
        );
    }

    /// The cast GUID must carry the `Cast` high type (47) and the spell id in its entry field, or the
    /// client cannot associate the cast with anything.
    #[test]
    fn the_cast_guid_has_the_cast_high_type_and_spell_id() {
        let (high, _) = cast_guid128(1, 0, 133, 7);
        assert_eq!(high >> 58, 47, "HighGuidType703::Cast");
        assert_eq!((high >> 6) & 0x7F_FFFF, 133, "spell id in the entry field");
        assert_eq!(high & 0x3F, 3, "SpellCastSource::Normal");
    }

    /// A reflected miss carries an extra 4-bit result; every other reason does not. Both are inside
    /// one bit run, so getting the condition wrong shifts the target lists that follow.
    #[test]
    fn a_reflected_miss_carries_its_result_nibble() {
        let target = ObjectGuid::new_creature(299, 464);
        let plain = go(
            vec![],
            vec![SpellMissTarget {
                guid: target,
                reason: 1,
                reflect_result: 0,
            }],
            1,
        );
        let reflected = go(
            vec![],
            vec![SpellMissTarget {
                guid: target,
                reason: SPELL_MISS_REFLECT,
                reflect_result: 2,
            }],
            1,
        );
        // Both flush to a whole byte, so the sizes match; the bytes must not.
        assert_ne!(plain.contents(), reflected.contents());
    }

    /// Hit and miss GUIDs follow the target block, and each is a packed guid128 rather than a raw u64.
    #[test]
    fn hit_targets_lengthen_the_body() {
        let none = go(vec![], vec![], 1).size();
        let one = go(vec![ObjectGuid::new_creature(299, 464)], vec![], 1).size();
        assert!(
            one > none,
            "a hit target must add its packed GUID to the body"
        );
    }

    // ===== SMSG_SPELL_PREPARE tests =====

    /// The modern PREPARE body is two packed GUID128s — ClientCastID then ServerCastID. It must
    /// round-trip: the client sends the *client* id in CMSG_CAST_SPELL and the server echoes it
    /// here paired with its own id, so a parse of this body must recover both.
    #[test]
    fn spell_prepare_round_trips_both_cast_ids() {
        let client_cast_id = (0x1122_3344_5566_7788u64, 0x99AA_BBCC_DDEE_FF00u64);
        let (server_high, server_low) = cast_guid128(DEFAULT_REALM_ID, 0, 133, 0xDEAD_BEEF);
        let msg = SmsgSpellPrepare {
            client_cast_id,
            server_cast_id: (server_high, server_low),
        };

        let pkt = msg.to_modern().expect("modern encoding must exist");
        assert_eq!(pkt.opcode(), Opcode::SMSG_SPELL_PREPARE);

        let mut reader = crate::protocol::bitbuf::BitReader::new(pkt.contents());
        assert_eq!(
            reader.read_packed_guid_128(),
            Some(client_cast_id),
            "the client's own cast id must lead the body"
        );
        assert_eq!(
            reader.read_packed_guid_128(),
            Some((server_high, server_low)),
            "the server cast id must follow the client's"
        );
        assert_eq!(
            reader.consumed(),
            pkt.contents().len(),
            "the whole body must be consumed"
        );
    }

    /// A vanilla session must never receive PREPARE — there is no such message on the 1.12 wire —
    /// so `to_vanilla` is a placeholder. Guard against regressions that would send a real body.
    #[test]
    fn spell_prepare_has_no_vanilla_form() {
        let msg = SmsgSpellPrepare {
            client_cast_id: (1, 2),
            server_cast_id: (3, 4),
        };
        assert!(!Opcode::SMSG_SPELL_PREPARE.has_vanilla());
        let pkt = msg.to_vanilla();
        assert_eq!(pkt.contents().len(), 0, "vanilla body must be empty");
    }
}
