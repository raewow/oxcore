//! Stable master and hunter pet message structs
//!
//! ## Server Messages (SMSG)
//! - [`MsgListStabledPets`] - The player's pets, with and without them, at a stable master
//! - [`SmsgStableResult`] - Outcome of a stable action
//! - [`SmsgPetTameFailure`] - Why taming or calling a pet failed

use crate::messages::update::DEFAULT_REALM_ID;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::ObjectGuid;
use crate::protocol::{Opcode, WorldPacket};

/// One pet in the stable window.
#[derive(Debug, Clone, PartialEq)]
pub struct StabledPetInfo {
    pub pet_number: u32,
    pub entry: u32,
    pub display_id: u32,
    pub level: u32,
    pub name: String,
    pub loyalty: u32,
    /// 1 for the pet with the player, otherwise the stable slot plus one.
    pub slot: u8,
}

/// MSG_LIST_STABLED_PETS - the stable window: the player's pet and their stabled pets.
#[derive(Debug, Clone)]
pub struct MsgListStabledPets {
    pub stable_master: ObjectGuid,
    pub stable_slots: u8,
    pub pets: Vec<StabledPetInfo>,
}

impl ToWorldPacket for MsgListStabledPets {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::MSG_LIST_STABLED_PETS);
        packet.write_u64(self.stable_master.raw());
        packet.write_u8(self.pets.len() as u8);
        packet.write_u8(self.stable_slots);
        for pet in &self.pets {
            packet.write_u32(pet.pet_number);
            packet.write_u32(pet.entry);
            packet.write_u32(pet.level);
            packet.write_cstring(&pet.name);
            packet.write_u32(pet.loyalty);
            packet.write_u8(pet.slot);
        }
        packet
    }

    /// `SMSG_PET_STABLE_LIST` in 1.14: an i32 count ahead of the slot count, each pet gains
    /// its display id, loyalty narrows to a byte, the slot becomes a flag (1 with the
    /// player, 3 stabled) and the name moves to the end behind an 8-bit length.
    fn to_modern(&self) -> Option<WorldPacket> {
        const WITH_PLAYER: u8 = 1;
        const STABLED: u8 = 3;
        let mut writer = BitWriter::new();
        let (high, low) = self.stable_master.to_guid128(DEFAULT_REALM_ID);
        writer.write_packed_guid_128(high, low);
        writer.write_i32(self.pets.len() as i32);
        writer.write_u8(self.stable_slots);
        for pet in &self.pets {
            writer.write_u32(pet.pet_number);
            writer.write_u32(pet.entry);
            writer.write_u32(pet.display_id);
            writer.write_u32(pet.level);
            writer.write_u8(pet.loyalty as u8);
            writer.write_u8(if pet.slot == 1 { WITH_PLAYER } else { STABLED });
            let name = pet.name.as_bytes();
            writer.write_bits(name.len() as u32, 8);
            writer.flush_bits();
            writer.write_bytes(name);
        }
        Some(writer.finish(Opcode::SMSG_PET_STABLE_LIST))
    }
}

/// Stable action outcomes, numbered the same by both clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StableResult {
    NotEnoughMoney = 0x01,
    Failed = 0x06,
    Stabled = 0x08,
    Unstabled = 0x09,
    BoughtSlot = 0x0A,
}

/// SMSG_STABLE_RESULT - the outcome of a stable action.
#[derive(Debug, Clone)]
pub struct SmsgStableResult {
    pub result: StableResult,
}

impl ToWorldPacket for SmsgStableResult {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_STABLE_RESULT);
        packet.write_u8(self.result as u8);
        packet
    }

    /// The same single byte in 1.14.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u8(self.result as u8);
        Some(writer.finish(Opcode::SMSG_STABLE_RESULT))
    }
}

/// Why a pet could not be tamed or called, numbered the same by both clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetTameFailure {
    InvalidCreature = 1,
    CreatureAlreadyOwned = 3,
    NotTameable = 4,
    AnotherSummonActive = 5,
    UnitsCantTame = 6,
    NoPetAvailable = 7,
    TooHighLevel = 9,
    Dead = 10,
}

/// SMSG_PET_TAME_FAILURE
#[derive(Debug, Clone)]
pub struct SmsgPetTameFailure {
    pub reason: PetTameFailure,
}

impl ToWorldPacket for SmsgPetTameFailure {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_PET_TAME_FAILURE);
        packet.write_u8(self.reason as u8);
        packet
    }

    /// The same single byte in 1.14.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u8(self.reason as u8);
        Some(writer.finish(Opcode::SMSG_PET_TAME_FAILURE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wolf(slot: u8) -> StabledPetInfo {
        StabledPetInfo {
            pet_number: 7,
            entry: 1131,
            display_id: 903,
            level: 12,
            name: "Wolf".to_string(),
            loyalty: 1,
            slot,
        }
    }

    #[test]
    fn vanilla_stable_list_layout() {
        let packet = MsgListStabledPets {
            stable_master: ObjectGuid::from_raw(0xF130_0000_1234_0001),
            stable_slots: 1,
            pets: vec![wolf(1)],
        }
        .to_vanilla();
        let data = packet.data();
        assert_eq!(data[8], 1); // count
        assert_eq!(data[9], 1); // slots
        assert_eq!(&data[10..14], &7u32.to_le_bytes());
        assert_eq!(&data[18..22], &12u32.to_le_bytes());
        assert_eq!(&data[22..27], b"Wolf\0");
        assert_eq!(&data[27..31], &1u32.to_le_bytes());
        assert_eq!(data[31], 1);
        assert_eq!(data.len(), 32);
    }

    #[test]
    fn modern_stable_list_marks_stabled_pets() {
        let packet = MsgListStabledPets {
            stable_master: ObjectGuid::empty(),
            stable_slots: 2,
            pets: vec![wolf(2)],
        }
        .to_modern()
        .unwrap();
        assert_eq!(packet.opcode(), Opcode::SMSG_PET_STABLE_LIST);
        let data = packet.data();
        // empty packed guid (2 mask bytes), count, slots, number, entry, display, level
        let pet = 2 + 4 + 1;
        assert_eq!(&data[pet + 12..pet + 16], &12u32.to_le_bytes());
        assert_eq!(data[pet + 17], 3); // stabled
        assert_eq!(data[pet + 18], 4); // name length
        assert_eq!(&data[pet + 19..], b"Wolf");
    }
}
