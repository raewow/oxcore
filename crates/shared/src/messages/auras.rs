//! Aura-related server messages

use crate::messages::update::DEFAULT_REALM_ID;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::guid::cast_guid128;
use crate::protocol::opcodes::Opcode;
use crate::protocol::packet::WorldPacketGuidExt;
use crate::protocol::ObjectGuid;
use crate::protocol::WorldPacket;
use std::sync::atomic::{AtomicU64, Ordering};

/// Cast-identity counter for aura applications, kept separate from the spell-cast one so the two
/// cannot exhaust each other's sequence space.
static AURA_CAST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Translate `encode_aura_flags_vanilla`'s byte to modern's `AuraFlags` bit positions.
///
/// Same hazard as NPC flags, unit dynamic flags, movement flags and hit info elsewhere in this
/// port: the flag *bits* renumber between protocols, not just the field. Vanilla's `Negative` is
/// `0x02`, which is modern's `Cancelable`; modern's own `Negative` is `0x10`. A raw copy would not
/// merely mislabel the aura -- it would show a debuff as cancelable-by-the-player and never mark it
/// negative, changing how the client colors and right-click-cancels the icon.
///
/// The input collapses passive/hidden auras to `0` before this ever sees them (see
/// `encode_aura_flags_vanilla`), so modern's `Passive` bit cannot be recovered here -- that
/// distinction is lost upstream of this translator, not by it.
///
/// `has_duration` raises modern's `Duration` bit (0x04): a timed aura without it shows no timer and
/// can be skipped from the buff bar entirely. `NoCaster` (0x01) is always set -- vanilla's aura
/// packet carries no caster, so the modern client must be told the cast unit is absent rather than
/// left expecting one it will never receive.
fn to_modern_aura_flags(vanilla: u8, has_duration: bool) -> u16 {
    const POSITIVE: u16 = 0x100;
    const NEGATIVE: u16 = 0x10;
    const CANCELABLE: u16 = 0x02;
    const DURATION: u16 = 0x04;
    const NO_CASTER: u16 = 0x01;

    let mut modern = NO_CASTER;
    if vanilla & 0x02 != 0 {
        modern |= NEGATIVE;
    } else if vanilla & 0x01 != 0 {
        modern |= POSITIVE;
    }
    if vanilla & 0x08 != 0 {
        modern |= CANCELABLE;
    }
    if has_duration {
        modern |= DURATION;
    }
    modern
}

/// SMSG_AURA_UPDATE (opcode 0x0495)
///
/// Sent when an aura is applied, updated, or removed on a target.
/// The client uses this to update the buff/debuff bar.
#[derive(Debug, Clone)]
pub struct SmsgAuraUpdate {
    /// Target unit GUID (PackGuid format)
    pub target_guid: ObjectGuid,
    /// Aura slot (0-63)
    pub slot: u8,
    /// Spell ID (0 = slot is cleared)
    pub spell_id: u32,
    /// Aura flags bitmask:
    /// 0x01 = positive, 0x02 = negative, 0x04 = passive, 0x08 = cancellable
    pub aura_flags: u8,
    /// Caster level (for scaling display)
    pub level: u8,
    /// Stack count (displayed as number on icon)
    pub stacks: u8,
    /// Remaining duration in ms (None = no duration bar)
    pub duration_ms: Option<u32>,
    /// Max duration in ms (for progress bar)
    pub max_duration_ms: Option<u32>,
}

impl ToWorldPacket for SmsgAuraUpdate {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_AURA_UPDATE);
        packet.write_packed_guid(self.target_guid);
        packet.write_u8(self.slot);

        if self.spell_id == 0 {
            // Slot cleared - just write spell_id = 0
            packet.write_u32(0);
            return packet;
        }

        packet.write_u32(self.spell_id);
        packet.write_u8(self.aura_flags);
        packet.write_u8(self.level);
        packet.write_u8(self.stacks);

        // Duration info (only if flag indicates duration)
        if let (Some(duration), Some(max_duration)) = (self.duration_ms, self.max_duration_ms) {
            packet.write_u32(max_duration);
            packet.write_u32(duration);
        }

        packet
    }

    /// `AuraUpdate::Write` with the nested `AuraInfo` and `AuraDataInfo`, per the 1.14 wire format.
    ///
    /// Reordered end to end: the **unit GUID goes last**, where vanilla leads with it, and an
    /// `UpdateAll` bit plus a 9-bit aura count come first. A cleared slot is expressed by the
    /// `HasAuraData` bit being false rather than by a zero spell id, and the duration pair swaps
    /// order — 1.14 sends total then remaining, both behind their own presence bits.
    ///
    /// `UpdateAll` is false: this message carries one slot, and claiming to be a full refresh would
    /// make the client drop every aura not mentioned.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_bit(false); // UpdateAll -- a single-slot delta, not a full refresh
        writer.write_bits(1, 9); // Auras count

        // --- AuraInfo ---
        writer.write_u8(self.slot);
        // A zero spell id is vanilla's "slot cleared"; 1.14 says it by omitting the data block.
        let has_data = self.spell_id != 0;
        writer.write_bit(has_data);
        writer.flush_bits();

        if has_data {
            // --- AuraDataInfo ---
            // The aura's own cast identity. Distinct per aura application for the same reason casts
            // need one; see `cast_guid128`.
            let sequence = AURA_CAST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let (high, low) = cast_guid128(DEFAULT_REALM_ID, 0, self.spell_id, sequence);
            writer.write_packed_guid_128(high, low); // CastID
            writer.write_u32(self.spell_id);
            writer.write_u32(0); // SpellXSpellVisualID -- no 1.12 source, see SmsgSpellGo
            writer.write_u16(to_modern_aura_flags(
                self.aura_flags,
                self.max_duration_ms.is_some(),
            ));
            // Vanilla carries no per-effect data, but the 1.14 client treats ActiveFlags=0 as "this
            // aura has no active effects" and skips it from the visible aura list (JimsProxy
            // clamps to effect 0 for the same reason). Mark the primary effect active.
            writer.write_u32(1); // ActiveFlags
            writer.write_u16(u16::from(self.level)); // CastLevel, widens from u8
            writer.write_u8(self.stacks); // Applications
            writer.write_i32(0); // ContentTuningID

            writer.write_bit(false); // HasCastUnit -- vanilla carries no caster here
            writer.write_bit(self.max_duration_ms.is_some()); // HasDuration
            writer.write_bit(self.duration_ms.is_some()); // HasRemaining
            writer.write_bit(false); // HasTimeMod
            writer.write_bits(0, 6); // Points count
            writer.write_bits(0, 6); // EstimatedPoints count
            writer.write_bit(false); // HasContentTuning

            // 1.14 sends total duration then remaining; vanilla sends max then current, so the pair
            // maps across in the same order despite the rename.
            if let Some(max_duration) = self.max_duration_ms {
                writer.write_i32(max_duration as i32);
            }
            if let Some(duration) = self.duration_ms {
                writer.write_i32(duration as i32);
            }
        }

        // The unit GUID trails the aura list, where vanilla leads with it.
        let (high, low) = self.target_guid.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);

        Some(writer.finish(Opcode::SMSG_AURA_UPDATE))
    }
}

/// SMSG_UPDATE_AURA_DURATION (opcode 0x0137)
///
/// Vanilla 1.12.1 packet — sent to the player to display buff timer.
/// Format: slot (u8) + duration_ms (u32)
#[derive(Debug, Clone)]
pub struct SmsgUpdateAuraDuration {
    pub slot: u8,
    pub duration_ms: u32,
}

impl ToWorldPacket for SmsgUpdateAuraDuration {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_UPDATE_AURA_DURATION);
        packet.write_u8(self.slot);
        packet.write_u32(self.duration_ms);
        packet
    }
}

/// SMSG_PERIODICAURALOG (opcode 0x024E)
///
/// Sent for each periodic tick (DoT, HoT, energize).
/// The client shows combat log text like "Corruption ticks for 120 damage".
#[derive(Debug, Clone)]
pub struct SmsgPeriodicAuraLog {
    pub target_guid: ObjectGuid,
    pub caster_guid: ObjectGuid,
    pub spell_id: u32,
    pub aura_type: u32,
    pub damage: u32,
    pub school: u8,
}

impl ToWorldPacket for SmsgPeriodicAuraLog {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_PERIODICAURALOG);
        packet.write_packed_guid(self.target_guid);
        packet.write_packed_guid(self.caster_guid);
        packet.write_u32(self.spell_id);
        packet.write_u32(self.damage);
        // Aura type determines the rest of the packet structure
        // This is a simplified implementation
        packet.write_u8(self.school);
        packet
    }
}

/// SMSG_AURA_UPDATE_ALL (opcode 0x0496)
///
/// Sent on login to sync all aura slots at once.
#[derive(Debug, Clone)]
pub struct SmsgAuraUpdateAll {
    pub target_guid: ObjectGuid,
    pub auras: Vec<AuraSlotData>,
}

/// Data for a single aura slot in SMSG_AURA_UPDATE_ALL.
#[derive(Debug, Clone)]
pub struct AuraSlotData {
    pub slot: u8,
    pub spell_id: u32,
    pub aura_flags: u8,
    pub level: u8,
    pub stacks: u8,
    pub duration_ms: Option<u32>,
    pub max_duration_ms: Option<u32>,
}

impl ToWorldPacket for SmsgAuraUpdateAll {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_AURA_UPDATE_ALL);
        packet.write_packed_guid(self.target_guid);

        // Write count of auras
        packet.write_u8(self.auras.len() as u8);

        for aura in &self.auras {
            packet.write_u8(aura.slot);
            packet.write_u32(aura.spell_id);

            if aura.spell_id != 0 {
                packet.write_u8(aura.aura_flags);
                packet.write_u8(aura.level);
                packet.write_u8(aura.stacks);

                if let (Some(duration), Some(max_duration)) =
                    (aura.duration_ms, aura.max_duration_ms)
                {
                    packet.write_u32(max_duration);
                    packet.write_u32(duration);
                }
            }
        }

        packet
    }
}

#[cfg(test)]
mod modern_aura_tests {
    use super::*;

    fn aura(spell_id: u32, duration: Option<u32>, max: Option<u32>) -> WorldPacket {
        SmsgAuraUpdate {
            target_guid: ObjectGuid::new_player(4),
            slot: 3,
            spell_id,
            aura_flags: 0x09,
            level: 60,
            stacks: 2,
            duration_ms: duration,
            max_duration_ms: max,
        }
        .to_modern()
        .expect("an aura update must encode for modern")
    }

    /// Vanilla clears a slot with a zero spell id; 1.14 says it by omitting the data block. Sending a
    /// zero-id data block instead would leave a phantom icon on the buff bar.
    #[test]
    fn a_cleared_slot_omits_the_data_block() {
        let applied = aura(1459, Some(1000), Some(1800));
        let cleared = aura(0, None, None);
        assert!(
            cleared.size() < applied.size(),
            "clearing must not carry aura data"
        );
    }

    /// Both duration fields hang off their own presence bits, so an aura with no timer must not
    /// reserve the space -- the target GUID follows immediately after.
    #[test]
    fn durations_follow_their_presence_bits() {
        let timed = aura(1459, Some(1000), Some(1800)).size();
        let permanent = aura(1459, None, None).size();
        assert_eq!(timed, permanent + 8, "two i32s behind two bits");
    }

    /// The unit GUID trails the aura list in 1.14, where vanilla leads with it. Getting this backwards
    /// makes the client attribute every aura to the wrong unit.
    #[test]
    fn the_unit_guid_is_written_last() {
        let packet = aura(1459, None, None);
        let bytes = packet.contents();
        let (high, low) = ObjectGuid::new_player(4).to_guid128(DEFAULT_REALM_ID);

        let mut expected = BitWriter::new();
        expected.write_packed_guid_128(high, low);
        let tail = expected.into_bytes();

        assert!(
            bytes.ends_with(&tail),
            "the packed unit GUID must be the last thing in the body"
        );
    }

    /// The bug this fixes: vanilla's `Negative` bit (`0x02`) sits where modern's `Cancelable` bit
    /// does, so a raw copy would show a debuff as player-cancelable and never mark it negative.
    #[test]
    fn negative_moves_to_its_modern_bit() {
        assert_eq!(to_modern_aura_flags(0x02, false), 0x10 | 0x01);
    }

    /// A cancelable positive buff (oxcore's `0x09` = `EF_FLAG_0 | CANCELABLE`) must set both
    /// modern bits, not just one -- losing `Cancelable` would make the client refuse a right-click
    /// cancel; losing `Positive` would mis-color the icon.
    #[test]
    fn a_cancelable_positive_buff_sets_both_modern_bits() {
        assert_eq!(to_modern_aura_flags(0x09, false), 0x100 | 0x02 | 0x01);
    }

    /// A plain positive buff with no cancel bit must not spuriously set `Cancelable`.
    #[test]
    fn a_plain_positive_buff_sets_only_positive() {
        assert_eq!(to_modern_aura_flags(0x01, false), 0x100 | 0x01);
    }

    /// A timed aura must set modern's `Duration` bit or the client shows no timer (and can skip the
    /// aura from the buff bar entirely); a permanent one must not.
    #[test]
    fn duration_flag_follows_duration_presence() {
        assert_ne!(to_modern_aura_flags(0x09, true) & 0x04, 0);
        assert_eq!(to_modern_aura_flags(0x09, false) & 0x04, 0);
    }

    /// Vanilla's aura packet carries no caster, so the modern client must be told the cast unit is
    /// absent via `NoCaster`, or it waits on a GUID that never arrives.
    #[test]
    fn no_caster_is_always_set() {
        assert_ne!(to_modern_aura_flags(0x00, false) & 0x01, 0);
    }

    /// ActiveFlags=0 means "this aura has no active effects" to the 1.14 client, which then skips it
    /// from the visible aura list -- the very symptom of "aura applied server-side but no icon".
    /// The body must carry at least the primary effect as active, per JimsProxy's clamp.
    #[test]
    fn active_flags_never_zero_for_a_live_aura() {
        let packet = aura(1459, None, None);
        let mut reader = crate::protocol::bitbuf::BitReader::new(packet.contents());
        reader.read_bit(); // UpdateAll
        assert_eq!(reader.read_bits(9), Some(1), "one aura in the list");
        let slot = reader.read_u8();
        assert_eq!(slot, Some(3));
        assert_eq!(reader.read_bit(), Some(true), "HasAuraData");
        reader.read_packed_guid_128(); // CastID
        reader.read_u32(); // SpellID
        reader.read_u32(); // SpellXSpellVisualID
        reader.read_bits(16); // Flags
        assert_eq!(reader.read_u32(), Some(1), "ActiveFlags must be nonzero");
    }

    /// Passive/hidden auras collapse to zero before reaching this function (see
    /// `encode_aura_flags_vanilla`), so zero must translate to nothing but the always-present
    /// `NoCaster` — never to an inferred positive/negative/cancelable bit.
    #[test]
    fn zero_flags_translate_to_no_caster_only() {
        assert_eq!(to_modern_aura_flags(0x00, false), 0x01);
    }
}
