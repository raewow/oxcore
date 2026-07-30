use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum HighGuid {
    Item = 0x4000,
    Player = 0x0000,
    GameObject = 0xF110,
    Transport = 0xF120,
    Unit = 0xF130,
    Pet = 0xF140,
    DynamicObject = 0xF100,
    Corpse = 0xF101,
    MoTransport = 0x1FC0,
}

impl HighGuid {
    pub fn has_entry(self) -> bool {
        matches!(
            self,
            HighGuid::GameObject | HighGuid::Transport | HighGuid::Unit | HighGuid::Pet
        )
    }

    pub fn type_name(self) -> &'static str {
        match self {
            HighGuid::Item => "Item",
            HighGuid::Player => "Player",
            HighGuid::GameObject => "Gameobject",
            HighGuid::Transport => "Transport",
            HighGuid::Unit => "Creature",
            HighGuid::Pet => "Pet",
            HighGuid::DynamicObject => "DynObject",
            HighGuid::Corpse => "Corpse",
            HighGuid::MoTransport => "MoTransport",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectGuid {
    guid: u64,
}

impl ObjectGuid {
    pub const fn empty() -> Self {
        Self { guid: 0 }
    }

    pub const fn from_raw(guid: u64) -> Self {
        Self { guid }
    }

    pub fn new_with_entry(high: HighGuid, entry: u32, counter: u32) -> Self {
        let entry_24 = entry & 0x00FFFFFF;
        let counter_24 = counter & 0x00FFFFFF;

        Self {
            guid: (counter_24 as u64) | ((entry_24 as u64) << 24) | ((high as u16 as u64) << 48),
        }
    }

    pub fn new_without_entry(high: HighGuid, counter: u32) -> Self {
        Self {
            guid: (counter as u64) | ((high as u16 as u64) << 48),
        }
    }

    pub fn new_player(counter: u32) -> Self {
        Self::new_without_entry(HighGuid::Player, counter)
    }

    pub fn new_creature(entry: u32, counter: u32) -> Self {
        Self::new_with_entry(HighGuid::Unit, entry, counter)
    }

    pub fn new_pet(entry: u32, counter: u32) -> Self {
        Self::new_with_entry(HighGuid::Pet, entry, counter)
    }

    pub fn new_item(counter: u32) -> Self {
        Self::new_without_entry(HighGuid::Item, counter)
    }

    pub fn new_gameobject(entry: u32, counter: u32) -> Self {
        Self::new_with_entry(HighGuid::GameObject, entry, counter)
    }

    pub fn new_corpse(counter: u32) -> Self {
        Self::new_without_entry(HighGuid::Corpse, counter)
    }

    pub fn new_dynamic_object(counter: u32) -> Self {
        Self::new_without_entry(HighGuid::DynamicObject, counter)
    }

    pub const fn raw(&self) -> u64 {
        self.guid
    }

    /// Convert this legacy GUID to the 1.14 `ObjectGuid` high/low pair.
    ///
    /// Legacy GUIDs do not carry a map or server ID, so map-specific GUIDs use zero for both.
    pub fn to_guid128(&self, realm_id: u16) -> (u64, u64) {
        let counter = self.counter() as u64;
        let high_type = match self.high() {
            HighGuid::Player => 2,
            HighGuid::Item => 3,
            HighGuid::Transport | HighGuid::MoTransport => 6,
            HighGuid::Unit => 8,
            HighGuid::Pet => 10,
            HighGuid::GameObject => 11,
            HighGuid::DynamicObject => 12,
            HighGuid::Corpse => 14,
        };

        if self.is_empty() {
            return (0, 0);
        }
        if matches!(self.high(), HighGuid::Transport | HighGuid::MoTransport) {
            return (
                (high_type as u64) << 58 | (counter << 38) | self.entry() as u64,
                0,
            );
        }

        let high = if matches!(self.high(), HighGuid::Player | HighGuid::Item) {
            (high_type as u64) << 58 | ((realm_id as u64 & 0x1FFF) << 42)
        } else {
            (high_type as u64) << 58
                | ((realm_id as u64 & 0x1FFF) << 42)
                | ((self.entry() as u64 & 0x7FFFFF) << 6)
        };
        (high, counter)
    }

    /// Rebuild a legacy GUID from a 1.14 `ObjectGuid` high/low pair -- the inverse of
    /// [`ObjectGuid::to_guid128`].
    ///
    /// Needed because every inbound modern packet names its target as a packed guid128, and the
    /// whole server -- creature manager, gameobject manager, visibility, combat -- is keyed on the
    /// legacy 64-bit form. Reading only the low half looks like it works, because a *player's*
    /// legacy GUID is exactly its counter; for a creature it silently drops both the `Unit` high
    /// bits and the entry, leaving a bare counter that matches nothing.
    ///
    /// The 1.14 field layout:
    ///
    /// ```text
    /// highType = high >> 58        entry   = (high >> 6) & 0x7FFFFF
    /// counter  = low               (Transport packs both into `high` instead)
    /// ```
    ///
    /// Unknown or unrepresentable high types return [`ObjectGuid::empty`] rather than guessing:
    /// this parses attacker-controlled input, and an empty GUID fails the caller's existence check
    /// where a fabricated one might hit an unrelated object.
    pub fn from_guid128(high: u64, low: u64) -> Self {
        if high == 0 && low == 0 {
            return Self::empty();
        }

        // `HighGuidType703`, per the 1.14 wire format.
        const PLAYER: u64 = 2;
        const ITEM: u64 = 3;
        const TRANSPORT: u64 = 6;
        const CREATURE: u64 = 8;
        const VEHICLE: u64 = 9;
        const PET: u64 = 10;
        const GAME_OBJECT: u64 = 11;
        const DYNAMIC_OBJECT: u64 = 12;
        const CORPSE: u64 = 14;

        let high_type = high >> 58;

        // Transport is its own layout -- `type << 58 | counter << 38 | entry`, with an empty low
        // half -- so it cannot share the extraction below. A zero entry means `MoTransport`, which
        // is how the reference tells the two apart.
        if high_type == TRANSPORT {
            let counter = ((high >> 38) & 0x000F_FFFF) as u32;
            let entry = (high & 0xFFFF_FFFF) as u32;
            return if entry != 0 {
                Self::new_with_entry(HighGuid::Transport, entry, counter)
            } else {
                Self::new_without_entry(HighGuid::MoTransport, counter)
            };
        }

        let entry = ((high >> 6) & 0x007F_FFFF) as u32;
        // The legacy counter is 32 bits at most, and only 24 for the types that carry an entry.
        // Truncating is what the reference does (`(uint)guid.GetCounter()`).
        let counter = (low & 0xFFFF_FFFF) as u32;

        match high_type {
            PLAYER => Self::new_without_entry(HighGuid::Player, counter),
            ITEM => Self::new_without_entry(HighGuid::Item, counter),
            // 1.14 splits vehicles out of creatures; 1.12 has no such type, so they fold back
            // together the way the reference folds them.
            CREATURE | VEHICLE => Self::new_with_entry(HighGuid::Unit, entry, counter),
            PET => Self::new_with_entry(HighGuid::Pet, entry, counter),
            GAME_OBJECT => Self::new_with_entry(HighGuid::GameObject, entry, counter),
            // Vanilla stores no entry for these two, so `to_guid128` wrote a zero and there is
            // nothing to read back.
            DYNAMIC_OBJECT => Self::new_without_entry(HighGuid::DynamicObject, counter),
            CORPSE => Self::new_without_entry(HighGuid::Corpse, counter),
            _ => Self::empty(),
        }
    }

    pub fn high(&self) -> HighGuid {
        let high_16 = ((self.guid >> 48) & 0xFFFF) as u16;

        match high_16 {
            0x4000 => HighGuid::Item,
            0x0000 => HighGuid::Player,
            0xF110 => HighGuid::GameObject,
            0xF120 => HighGuid::Transport,
            0xF130 => HighGuid::Unit,
            0xF140 => HighGuid::Pet,
            0xF100 => HighGuid::DynamicObject,
            0xF101 => HighGuid::Corpse,
            0x1FC0 => HighGuid::MoTransport,
            _ => HighGuid::Player,
        }
    }

    pub fn entry(&self) -> u32 {
        if self.high().has_entry() {
            ((self.guid >> 24) & 0x00FFFFFF) as u32
        } else {
            0
        }
    }

    pub fn counter(&self) -> u32 {
        let high_type = self.high();
        if high_type.has_entry() {
            (self.guid & 0x00FFFFFF) as u32
        } else {
            (self.guid & 0xFFFFFFFF) as u32
        }
    }

    pub const fn is_empty(&self) -> bool {
        self.guid == 0
    }

    pub fn is_player(&self) -> bool {
        !self.is_empty() && self.high() == HighGuid::Player
    }

    pub fn is_creature(&self) -> bool {
        self.high() == HighGuid::Unit
    }

    pub fn is_pet(&self) -> bool {
        self.high() == HighGuid::Pet
    }

    pub fn is_unit(&self) -> bool {
        self.is_creature() || self.is_pet() || self.is_player()
    }

    pub fn is_creature_or_pet(&self) -> bool {
        self.is_creature() || self.is_pet()
    }

    pub fn is_item(&self) -> bool {
        self.high() == HighGuid::Item
    }

    pub fn is_game_object(&self) -> bool {
        self.high() == HighGuid::GameObject || self.high() == HighGuid::Transport
    }

    pub fn is_dynamic_object(&self) -> bool {
        self.high() == HighGuid::DynamicObject
    }

    pub fn is_corpse(&self) -> bool {
        self.high() == HighGuid::Corpse
    }

    pub fn is_transport(&self) -> bool {
        self.high() == HighGuid::Transport
    }

    pub fn is_mo_transport(&self) -> bool {
        self.high() == HighGuid::MoTransport
    }

    pub fn low(&self) -> u32 {
        self.counter()
    }

    pub fn high_u32(&self) -> u32 {
        (self.guid >> 32) as u32
    }

    pub fn from_low(low: u32) -> Self {
        Self::new_without_entry(HighGuid::Player, low)
    }

    pub fn clear(&mut self) {
        self.guid = 0;
    }

    pub fn max_counter(&self) -> u32 {
        if self.high().has_entry() {
            0x00FFFFFF
        } else {
            0xFFFFFFFF
        }
    }

    pub fn clamp_player_guid(&mut self) {
        if self.high() == HighGuid::Player {
            let counter = self.counter();
            self.guid = counter as u64;
        }
    }

    pub fn type_name(&self) -> &'static str {
        if self.is_empty() {
            "None"
        } else {
            self.high().type_name()
        }
    }

    pub fn to_string_debug(&self) -> String {
        if self.is_empty() {
            return "None".to_string();
        }

        let type_name = self.high().type_name();
        let mut result = type_name.to_string();

        result.push_str(" (");
        if self.high().has_entry() {
            if self.is_pet() {
                result.push_str("Petnumber: ");
            } else {
                result.push_str("Entry: ");
            }
            result.push_str(&self.entry().to_string());
            result.push(' ');
        }
        result.push_str("Guid: ");
        result.push_str(&self.counter().to_string());
        result.push(')');

        result
    }
}

/// Build a 1.14 `Cast` GUID: the identity of one spell cast.
///
/// Not a conversion — vanilla has no cast identity at all, so there is no 64-bit form to convert
/// from and this returns the `(high, low)` pair directly.
///
/// The 1.14 client keys its cast bar, sounds and visual chunks on this GUID and assumes it is
/// **unique per cast**. the 1.14 reference learned that the hard way: reusing a deterministic id per
/// (spell, caster) made "visual chunks drift, sounds clip, and target-frame cast bars ignore the
/// dismiss on Kick interrupts". So
/// `sequence` must differ for every cast, not merely for every spell.
///
/// The layout is the map-specific one, with the spell id in the entry field and the cast source as
/// the sub-type.
pub fn cast_guid128(realm_id: u16, map_id: u16, spell_id: u32, sequence: u64) -> (u64, u64) {
    /// `HighGuidType703::Cast`.
    const CAST: u64 = 47;
    /// `SpellCastSource::Normal`.
    const SOURCE_NORMAL: u64 = 3;

    let high = (CAST << 58)
        | ((u64::from(realm_id) & 0x1FFF) << 42)
        | ((u64::from(map_id) & 0x1FFF) << 29)
        | ((u64::from(spell_id) & 0x7F_FFFF) << 6)
        | SOURCE_NORMAL;
    // The server-id field above the counter stays zero, as it does for every other GUID we build.
    (high, sequence & 0xFF_FFFF_FFFF)
}

impl Default for ObjectGuid {
    fn default() -> Self {
        Self::empty()
    }
}

impl Hash for ObjectGuid {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.guid.hash(state);
    }
}

impl fmt::Display for ObjectGuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string_debug())
    }
}

impl From<u64> for ObjectGuid {
    fn from(guid: u64) -> Self {
        Self::from_raw(guid)
    }
}

impl From<ObjectGuid> for u64 {
    fn from(guid: ObjectGuid) -> Self {
        guid.raw()
    }
}

#[cfg(test)]
mod guid128_tests {
    use super::*;

    const REALM: u16 = 1;

    /// One test pins both directions, so `to_guid128` and `from_guid128` cannot drift apart.
    #[test]
    fn guid128_round_trips_for_every_representable_type() {
        let cases = [
            ObjectGuid::new_player(4),
            ObjectGuid::new_item(0x00AB_CDEF),
            ObjectGuid::new_creature(299, 0x0012_3456),
            ObjectGuid::new_pet(517, 42),
            ObjectGuid::new_gameobject(151955, 8171),
            ObjectGuid::new_dynamic_object(77),
            ObjectGuid::new_corpse(1234),
            ObjectGuid::new_with_entry(HighGuid::Transport, 176080, 999),
            ObjectGuid::new_without_entry(HighGuid::MoTransport, 555),
        ];

        for original in cases {
            let (high, low) = original.to_guid128(REALM);
            assert_eq!(
                ObjectGuid::from_guid128(high, low),
                original,
                "{original} did not survive the 128-bit round trip"
            );
        }
    }

    #[test]
    fn empty_round_trips_as_empty() {
        let (high, low) = ObjectGuid::empty().to_guid128(REALM);
        assert_eq!((high, low), (0, 0));
        assert!(ObjectGuid::from_guid128(0, 0).is_empty());
    }

    /// The bug this function exists to fix: taking the low half alone loses the type and the entry.
    #[test]
    fn low_half_alone_is_not_a_creature_guid() {
        let creature = ObjectGuid::new_creature(299, 464);
        let (high, low) = creature.to_guid128(REALM);

        assert_eq!(ObjectGuid::from_raw(low).raw(), 464, "the observed symptom");
        assert!(!ObjectGuid::from_raw(low).is_creature());

        let decoded = ObjectGuid::from_guid128(high, low);
        assert!(decoded.is_creature());
        assert_eq!(decoded.entry(), 299);
        assert_eq!(decoded.counter(), 464);
    }

    /// 1.14 has a `Vehicle` high type that 1.12 does not; it must land on `Unit`, not be dropped.
    #[test]
    fn vehicle_folds_into_unit() {
        let vehicle_high = 9u64 << 58 | (u64::from(REALM) << 42) | (299u64 << 6);
        let decoded = ObjectGuid::from_guid128(vehicle_high, 464);
        assert!(decoded.is_creature());
        assert_eq!(decoded.entry(), 299);
    }

    /// Attacker-controlled input: an unmapped high type must produce an empty GUID, never a
    /// fabricated one that could collide with a real object.
    #[test]
    fn unknown_high_types_decode_to_empty() {
        for high_type in [0u64, 1, 4, 5, 7, 13, 15, 33, 63] {
            let high = high_type << 58 | (299u64 << 6);
            assert!(
                ObjectGuid::from_guid128(high, 464).is_empty(),
                "high type {high_type} should not decode to a usable GUID"
            );
        }
    }
}

pub struct ObjectGuidGenerator {
    next_guid: u32,
    freed_guids: Vec<u32>,
}

impl ObjectGuidGenerator {
    pub fn new(start: u32) -> Self {
        Self {
            next_guid: start,
            freed_guids: Vec::new(),
        }
    }

    pub fn generate(&mut self) -> u32 {
        if let Some(guid) = self.freed_guids.pop() {
            guid
        } else {
            let guid = self.next_guid;
            self.next_guid = self.next_guid.wrapping_add(1);
            guid
        }
    }

    pub fn free(&mut self, guid: u32) {
        self.freed_guids.push(guid);
    }

    pub fn peek_next(&self) -> u32 {
        self.next_guid
    }

    pub fn freed_count(&self) -> usize {
        self.freed_guids.len()
    }

    pub fn next(&self) -> u32 {
        self.next_guid
    }

    pub fn set_max_used(&mut self, max: u32) {
        self.next_guid = max.wrapping_add(1);
    }

    pub fn freed_guids_sort_unstable(&mut self) {
        self.freed_guids.sort_unstable();
    }

    pub fn freed_guids_reverse(&mut self) {
        self.freed_guids.reverse();
    }
}
