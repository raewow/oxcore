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
