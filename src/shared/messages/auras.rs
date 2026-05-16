//! Aura-related server messages

use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::opcodes::Opcode;
use crate::shared::protocol::packet::WorldPacketGuidExt;
use crate::shared::protocol::ObjectGuid;
use crate::shared::protocol::WorldPacket;

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
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
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
    fn to_world_packet(&self) -> WorldPacket {
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
