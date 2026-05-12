use crate::shared::protocol::{ObjectGuid, Opcode, WorldPacket};
use crate::shared::protocol::packet::WorldPacketGuidExt;

/// SMSG_SPELL_START (opcode 0x0131)
/// Broadcast when a spell cast begins (cast bar appears).
pub struct SmsgSpellStart {
    pub caster_guid: ObjectGuid,
    pub caster_guid_pack: ObjectGuid,
    pub spell_id: u32,
    pub cast_flags: u16,
    pub cast_time_ms: u32,
    pub target_guid: Option<ObjectGuid>,
    /// When the spell is cast from an item, this is the item's GUID. The client
    /// expects the item GUID as the first packed GUID in the packet instead of
    /// the caster's GUID (MaNGOS: `data << m_CastItem->GetPackGUID()`).
    pub cast_item_guid: Option<ObjectGuid>,
}

impl SmsgSpellStart {
    pub fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SPELL_START);
        // First packed GUID: item GUID if cast from an item, else caster GUID
        packet.write_packed_guid(self.cast_item_guid.unwrap_or(self.caster_guid));
        packet.write_packed_guid(self.caster_guid_pack);
        packet.write_u32(self.spell_id);
        packet.write_u16(self.cast_flags);
        packet.write_u32(self.cast_time_ms);

        // SpellCastTargets section (vanilla 1.12.x requires this)
        write_spell_cast_targets(&mut packet, self.target_guid);

        tracing::info!("[SPELL_START] spell={} caster={:?} target={:?} cast_flags=0x{:04X} cast_time={} packet_len={} bytes={:02X?}",
            self.spell_id, self.caster_guid, self.target_guid, self.cast_flags, self.cast_time_ms,
            packet.data().as_ref().len(), packet.data().as_ref());
        packet
    }
}

/// SMSG_SPELL_GO (opcode 0x0132)
/// Broadcast when a spell fires (effects apply).
pub struct SmsgSpellGo {
    pub caster_guid: ObjectGuid,
    pub caster_guid_pack: ObjectGuid,
    pub spell_id: u32,
    pub cast_flags: u16,
    pub hit_targets: Vec<ObjectGuid>,
    pub miss_targets: Vec<ObjectGuid>,
    pub target_guid: Option<ObjectGuid>,
    /// When the spell is cast from an item, this is the item's GUID. The client
    /// expects the item GUID as the first packed GUID and CAST_FLAG_UNKNOWN7
    /// (0x0040) set in cast_flags (MaNGOS: `data << m_CastItem->GetPackGUID()`
    /// and `castFlags |= CAST_FLAG_UNKNOWN7`).
    pub cast_item_guid: Option<ObjectGuid>,
}

// CAST_FLAG_UNKNOWN7: signals that the first packed GUID is a cast item, not
// the caster. The client uses this to track which item was used.
const CAST_FLAG_ITEM_CAST: u16 = 0x0040;

impl SmsgSpellGo {
    pub fn to_world_packet(&self) -> WorldPacket {
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
            packet.write_guid(*target); // Full 8-byte GUID
            packet.write_u8(0); // Miss reason (0 = MISS_NONE, placeholder)
        }

        // SpellCastTargets section (vanilla 1.12.x requires this)
        write_spell_cast_targets(&mut packet, self.target_guid);

        tracing::info!("[SPELL_GO] spell={} caster={:?} item={:?} target={:?} cast_flags=0x{:04X} hits={} misses={} packet_len={} bytes={:02X?}",
            self.spell_id, self.caster_guid, self.cast_item_guid, self.target_guid, cast_flags,
            self.hit_targets.len(), self.miss_targets.len(),
            packet.data().as_ref().len(), packet.data().as_ref());
        packet
    }
}

/// SMSG_PLAY_SPELL_VISUAL (opcode 0x01F3)
/// Plays a SpellVisualKit animation on a unit, visible to nearby players.
pub struct SmsgPlaySpellVisual {
    pub caster_guid: ObjectGuid,
    pub spell_visual_kit_id: u32,
}

impl SmsgPlaySpellVisual {
    pub fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_PLAY_SPELL_VISUAL);
        packet.write_u64(self.caster_guid.raw());
        packet.write_u32(self.spell_visual_kit_id);
        packet
    }
}

#[cfg(test)]
mod spell_packet_tests {
    use super::*;
    use bytes::Buf;
    use crate::shared::protocol::HighGuid;

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
            cast_flags: 0x0100,
            hit_targets: vec![],
            miss_targets: vec![],
            target_guid: None,
            cast_item_guid: None,
        };
        let pkt = msg.to_world_packet();
        let mut data = pkt.data().clone();

        let first_guid = read_packed_guid(&mut data);
        assert_eq!(first_guid, caster.raw(), "first GUID must be caster when no item");

        let _second_guid = read_packed_guid(&mut data);
        let _spell_id = {
            let bytes = [data[0], data[1], data[2], data[3]];
            data.advance(4);
            u32::from_le_bytes(bytes)
        };
        let flags = read_u16_le(&mut data);
        assert_eq!(flags & 0x0040, 0, "CAST_FLAG_ITEM_CAST must NOT be set without an item");
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
            cast_flags: 0x0002,
            hit_targets: vec![],
            miss_targets: vec![],
            target_guid: None,
            cast_item_guid: Some(item),
        };
        let pkt = msg.to_world_packet();
        let mut data = pkt.data().clone();

        let first_guid = read_packed_guid(&mut data);
        assert_eq!(first_guid, item.raw(), "first GUID must be item GUID when cast from item");

        let _second_guid = read_packed_guid(&mut data);
        let _spell_id = {
            let bytes = [data[0], data[1], data[2], data[3]];
            data.advance(4);
            u32::from_le_bytes(bytes)
        };
        let flags = read_u16_le(&mut data);
        assert_ne!(flags & 0x0040, 0, "CAST_FLAG_ITEM_CAST (0x0040) must be set when cast from item");
        assert_ne!(flags & 0x0002, 0, "original cast_flags bits must be preserved");
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
            cast_flags: 0x0002,
            cast_time_ms: 10000,
            target_guid: None,
            cast_item_guid: None,
        };
        let pkt = msg.to_world_packet();
        let mut data = pkt.data().clone();

        let first_guid = read_packed_guid(&mut data);
        assert_eq!(first_guid, caster.raw(), "first GUID must be caster when no item");
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
            cast_flags: 0x0002,
            cast_time_ms: 10000,
            target_guid: None,
            cast_item_guid: Some(item),
        };
        let pkt = msg.to_world_packet();
        let mut data = pkt.data().clone();

        let first_guid = read_packed_guid(&mut data);
        assert_eq!(first_guid, item.raw(), "first GUID must be item GUID when cast from item");
    }
}

/// Write the SpellCastTargets section to a spell packet.
/// Matches MaNGOS SpellCastTargets::write() for vanilla 1.12.x.
fn write_spell_cast_targets(packet: &mut WorldPacket, target_guid: Option<ObjectGuid>) {
    match target_guid {
        Some(guid) if guid.raw() != 0 => {
            // TARGET_FLAG_UNIT
            packet.write_u16(0x0002);
            packet.write_packed_guid(guid);
        }
        _ => {
            // TARGET_FLAG_SELF (0x0000) — no additional target data
            packet.write_u16(0x0000);
            packet.write_u8(0); // empty packed GUID
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

impl SmsgSpellFailure {
    pub fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SPELL_FAILURE);
        packet.write_guid(self.caster_guid);
        packet.write_u32(self.spell_id);
        packet.write_u8(self.result);
        packet
    }
}

/// SMSG_CAST_RESULT (opcode 0x0130)
/// Sent to the caster with the result of their cast attempt.
pub struct SmsgCastResult;

impl SmsgCastResult {
    /// Build a success SMSG_CAST_RESULT: spell_id(u32) + status(u8=0)
    pub fn success(spell_id: u32) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_CAST_RESULT);
        packet.write_u32(spell_id);
        packet.write_u8(0); // Success
        packet
    }

    /// Build a failure SMSG_CAST_RESULT: spell_id(u32) + status(u8=2) + error_code(u8)
    pub fn failure(spell_id: u32, error_code: u8) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_CAST_RESULT);
        packet.write_u32(spell_id);
        packet.write_u8(2); // Failed
        packet.write_u8(error_code);
        packet
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

impl SmsgSpellCooldown {
    pub fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SPELL_COOLDOWN);
        packet.write_packed_guid(self.caster_guid);
        for (spell_id, cooldown_ms) in &self.cooldowns {
            packet.write_u32(*spell_id);
            packet.write_u32(*cooldown_ms);
        }
        tracing::info!("[SPELL_COOLDOWN] caster={:?} cooldowns={:?} packet_len={} bytes={:02X?}",
            self.caster_guid, self.cooldowns,
            packet.data().as_ref().len(), packet.data().as_ref());
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

impl SmsgInitialSpells {
    pub fn to_world_packet(&self) -> WorldPacket {
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

impl SmsgLearnedSpell {
    pub fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_LEARNED_SPELL);
        packet.write_u32(self.spell_id);
        packet.write_u16(0); // unknown
        packet
    }
}

/// SMSG_REMOVED_SPELL (opcode 0x0203)
pub struct SmsgRemovedSpell {
    pub spell_id: u32,
}

impl SmsgRemovedSpell {
    pub fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_REMOVED_SPELL);
        packet.write_u32(self.spell_id);
        packet
    }
}

/// SMSG_CLEAR_COOLDOWN (opcode 0x01DE)
pub struct SmsgClearCooldown {
    pub spell_id: u32,
    pub caster_guid: ObjectGuid,
}

impl SmsgClearCooldown {
    pub fn to_world_packet(&self) -> WorldPacket {
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

impl SmsgSpellDelayed {
    pub fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SPELL_DELAYED);
        packet.write_guid(self.caster_guid);
        packet.write_u32(self.delay_ms);
        packet
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
