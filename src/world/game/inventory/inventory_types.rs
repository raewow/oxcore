/// Inventory system types, enums, and constants

/// Inventory result codes (equipment errors)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryResult {
    Ok = 0,
    CantEquipLevelI = 1,
    CantEquipSkill = 2,
    ItemDoesntGoToSlot = 3,
    BagFull = 4,
    NonEmptyBagOverOtherBag = 5,
    CantTradeEquipBags = 6,
    OnlyAmmoCanGoHere = 7,
    NoRequiredProficiency = 8,
    NoEquipmentSlotAvailable = 9,
    YouCanNeverUseThatItem = 10,
    YouCanNeverUseThatItem2 = 11,
    NoEquipmentSlotAvailable2 = 12,
    CantEquipWithTwohanded = 13,
    CantDualWield = 14,
    ItemDoesntGoIntoBag = 15,
    ItemDoesntGoIntoBag2 = 16,
    CantCarryMoreOfThis = 17,
    NoEquipmentSlotAvailable3 = 18,
    ItemCantStack = 19,
    ItemCantBeEquipped = 20,
    ItemsCantBeSwapped = 21,
    SlotIsEmpty = 22,
    ItemNotFound = 23,
    CantDropSoulbound = 24,
    OutOfRange = 25,
    TriedToSplitMoreThanCount = 26,
    CouldntSplitItems = 27,
    MissingReagent = 28,
    NotEnoughMoney = 29,
    NotABag = 30,
    CanOnlyDoWithEmptyBags = 31,
    DontOwnThatItem = 32,
    CanEquipOnly1Quiver = 33,
    MustPurchaseThatBagSlot = 34,
    TooFarAwayFromBank = 35,
    ItemLocked = 36,
    YouAreStunned = 37,
    YouAreDead = 38,
    CantDoRightNow = 39,
    IntBagError = 40,
    CanEquipOnly1Bolt = 41,
    CanEquipOnly1Ammopouch = 42,
    StackableCantBeWrapped = 43,
    EquippedCantBeWrapped = 44,
    WrappedCantBeWrapped = 45,
    BoundCantBeWrapped = 46,
    UniqueCantBeWrapped = 47,
    BagsCantBeWrapped = 48,
    AlreadyLooted = 49,
    InventoryFull = 50,
    BankFull = 51,
    ItemIsCurrentlySoldOut = 52,
    BagFull3 = 53,
    ItemNotFound2 = 54,
    ItemCantStack2 = 55,
    BagFull4 = 56,
    ItemSoldOut = 57,
    ObjectIsBusy = 58,
    None = 59,
    NotInCombat = 60,
    NotWhileDisarmed = 61,
    BagFull6 = 62,
    CantEquipRank = 63,
    CantEquipReputation = 64,
    TooManySpecialBags = 65,
    LootCantLootThatNow = 66,
}

/// Item update state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemUpdateState {
    Unchanged = 0,
    Changed = 1,
    New = 2,
    Removed = 3,
}

/// Item loot update state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemLootUpdateState {
    None = 0,
    Temporary = 1,
    Unchanged = 2,
    Changed = 3,
    New = 4,
    Removed = 5,
}

/// Enchantment slot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnchantmentSlot {
    Perm = 0,
    Temp = 1,
    MaxInspected = 2,
    Prop0 = 3,
    Prop1 = 4,
    Prop2 = 5,
    Prop3 = 6,
    Max = 7,
}

impl EnchantmentSlot {
    /// Convert to u8 index
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Create from u8 index
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => EnchantmentSlot::Perm,
            1 => EnchantmentSlot::Temp,
            2 => EnchantmentSlot::MaxInspected,
            3 => EnchantmentSlot::Prop0,
            4 => EnchantmentSlot::Prop1,
            5 => EnchantmentSlot::Prop2,
            6 => EnchantmentSlot::Prop3,
            _ => EnchantmentSlot::Perm,
        }
    }
}

pub const MAX_ENCHANTMENT_SLOT: usize = 7;
pub const MAX_ENCHANTMENT_OFFSET: usize = 3;

/// Enchantment offset within slot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnchantmentOffset {
    Id = 0,
    Duration = 1,
    Charges = 2,
}

/// Item dynamic flags
pub mod item_flags {
    pub const BOUND: u32 = 0x00000001;
    pub const TRANSLATED: u32 = 0x00000002;
    pub const UNLOCKED: u32 = 0x00000004;
    pub const WRAPPED: u32 = 0x00000008;
    pub const READABLE: u32 = 0x00000200;
}

/// Inventory slot constants
pub const INVENTORY_SLOT_BAG_0: u8 = 255;
pub const NULL_BAG: u8 = 0;
pub const NULL_SLOT: u8 = 255;

/// Equipment slots (0-18)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EquipmentSlot {
    Head = 0,
    Neck = 1,
    Shoulders = 2,
    Body = 3,
    Chest = 4,
    Waist = 5,
    Legs = 6,
    Feet = 7,
    Wrists = 8,
    Hands = 9,
    Finger1 = 10,
    Finger2 = 11,
    Trinket1 = 12,
    Trinket2 = 13,
    Back = 14,
    Mainhand = 15,
    Offhand = 16,
    Ranged = 17,
    Tabard = 18,
}

impl EquipmentSlot {
    pub const START: u8 = 0;
    pub const END: u8 = 19;

    pub fn from_u8(slot: u8) -> Option<Self> {
        match slot {
            0 => Some(Self::Head),
            1 => Some(Self::Neck),
            2 => Some(Self::Shoulders),
            3 => Some(Self::Body),
            4 => Some(Self::Chest),
            5 => Some(Self::Waist),
            6 => Some(Self::Legs),
            7 => Some(Self::Feet),
            8 => Some(Self::Wrists),
            9 => Some(Self::Hands),
            10 => Some(Self::Finger1),
            11 => Some(Self::Finger2),
            12 => Some(Self::Trinket1),
            13 => Some(Self::Trinket2),
            14 => Some(Self::Back),
            15 => Some(Self::Mainhand),
            16 => Some(Self::Offhand),
            17 => Some(Self::Ranged),
            18 => Some(Self::Tabard),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Inventory bag slots (19-22)
pub const INVENTORY_SLOT_BAG_START: u8 = 19;
pub const INVENTORY_SLOT_BAG_END: u8 = 23;

/// Inventory item slots (23-38)
pub const INVENTORY_SLOT_ITEM_START: u8 = 23;
pub const INVENTORY_SLOT_ITEM_END: u8 = 39;

/// Bank item slots (39-62)
pub const BANK_SLOT_ITEM_START: u8 = 39;
pub const BANK_SLOT_ITEM_END: u8 = 63;

/// Bank bag slots (63-68)
pub const BANK_SLOT_BAG_START: u8 = 63;
pub const BANK_SLOT_BAG_END: u8 = 69;

/// Buyback slots (69-80)
pub const BUYBACK_SLOT_START: u8 = 69;
pub const BUYBACK_SLOT_END: u8 = 81;

/// Keyring slots (81-96)
pub const KEYRING_SLOT_START: u8 = 81;
pub const KEYRING_SLOT_END: u8 = 97;

/// Maximum bag size (36 slots)
pub const MAX_BAG_SIZE: usize = 36;

/// Position encoding: (bag << 8) | slot
pub fn encode_position(bag: u8, slot: u8) -> u16 {
    ((bag as u16) << 8) | (slot as u16)
}

/// Decode position to bag and slot
pub fn decode_position(pos: u16) -> (u8, u8) {
    let bag = ((pos >> 8) & 0xFF) as u8;
    let slot = (pos & 0xFF) as u8;
    (bag, slot)
}

/// Check if position is equipment slot
pub fn is_equipment_pos(bag: u8, slot: u8) -> bool {
    bag == INVENTORY_SLOT_BAG_0 && slot < EquipmentSlot::END
}

/// Check if position is inventory slot
pub fn is_inventory_pos(bag: u8, slot: u8) -> bool {
    (bag == INVENTORY_SLOT_BAG_0
        && slot >= INVENTORY_SLOT_ITEM_START
        && slot < INVENTORY_SLOT_ITEM_END)
        || (bag >= INVENTORY_SLOT_BAG_START && bag < INVENTORY_SLOT_BAG_END)
}

/// Check if position is bag slot
pub fn is_bag_pos(bag: u8, slot: u8) -> bool {
    bag == INVENTORY_SLOT_BAG_0 && slot >= INVENTORY_SLOT_BAG_START && slot < INVENTORY_SLOT_BAG_END
}

/// Check if position is bank position
pub fn is_bank_pos(bag: u8, slot: u8) -> bool {
    (bag == INVENTORY_SLOT_BAG_0 && slot >= BANK_SLOT_ITEM_START && slot < BANK_SLOT_ITEM_END)
        || (bag >= BANK_SLOT_BAG_START && bag < BANK_SLOT_BAG_END)
}

/// Item position and count for storage operations
#[derive(Debug, Clone, Copy)]
pub struct ItemPosCount {
    pub pos: u16,
    pub count: u8,
}

impl ItemPosCount {
    pub fn new(pos: u16, count: u8) -> Self {
        Self { pos, count }
    }

    pub fn bag(&self) -> u8 {
        decode_position(self.pos).0
    }

    pub fn slot(&self) -> u8 {
        decode_position(self.pos).1
    }
}

pub type ItemPosCountVec = Vec<ItemPosCount>;
