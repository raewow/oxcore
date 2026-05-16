// Inventory system types and result enums
//
// This module contains the domain types used by the InventorySystem,
// separate from the lower-level types in game/inventory/types.rs.

use std::sync::Arc;

use crate::shared::protocol::ObjectGuid;

/// Maximum money a player can hold (99999 gold, 99 silver, 99 copper)
pub const MAX_MONEY: u32 = 999_999_999;

/// Result of an add_item operation
#[derive(Debug)]
pub enum AddItemResult {
    /// Item(s) successfully added
    Success {
        /// Items that were added or modified (GUID, new count)
        items_modified: Vec<(ObjectGuid, u32)>,
        /// New items that were created (GUID)
        items_created: Vec<ObjectGuid>,
    },
    /// No space in inventory
    InventoryFull,
    /// Item template not found
    InvalidItem,
    /// Player not found in cache
    PlayerNotLoaded,
    /// Database error
    DatabaseError(String),
}

/// Result of a remove_item operation
#[derive(Debug)]
pub enum RemoveItemResult {
    /// Item count reduced (but item still exists)
    CountReduced {
        item_guid: ObjectGuid,
        new_count: u32,
    },
    /// Item completely removed
    ItemRemoved { item_guid: ObjectGuid },
    /// Not enough items to remove
    InsufficientCount,
    /// Item not found
    ItemNotFound,
    /// Player not found in cache
    PlayerNotLoaded,
    /// Database error
    DatabaseError(String),
}

/// Result of a move_item operation
#[derive(Debug)]
pub enum MoveItemResult {
    /// Item moved successfully (simple move)
    Moved,
    /// Items swapped
    Swapped,
    /// Items merged (stacked)
    Merged {
        /// Source item was fully merged and removed
        source_removed: bool,
    },
    /// Invalid source position
    InvalidSource,
    /// Invalid destination position
    InvalidDestination,
    /// Cannot place item in destination slot
    CannotEquip(crate::world::game::inventory::inventory_types::InventoryResult),
    /// Player not found in cache
    PlayerNotLoaded,
    /// Database error
    DatabaseError(String),
}

/// Result of a split_item operation
#[derive(Debug)]
pub enum SplitItemResult {
    /// Successfully split stack - new item created
    Success {
        source_guid: ObjectGuid,
        new_item_guid: ObjectGuid,
    },
    /// Successfully merged into existing stack at destination
    MergedToExisting {
        source_guid: ObjectGuid,
        dest_guid: ObjectGuid,
    },
    /// Cannot split more than available
    InvalidCount,
    /// Source item not found
    SourceNotFound,
    /// Destination slot not empty and cannot merge
    DestinationOccupied,
    /// Player not found in cache
    PlayerNotLoaded,
    /// Database error
    DatabaseError(String),
}

/// Result of a gold operation
#[derive(Debug)]
pub enum GoldResult {
    /// Success with new balance
    Success { new_balance: u32 },
    /// Not enough gold
    InsufficientFunds,
    /// Would exceed max gold cap
    CapExceeded,
    /// Player not found
    PlayerNotLoaded,
    /// Database error
    DatabaseError(String),
}

/// Item properties to preserve during transfer (trade, mail, etc.)
#[derive(Debug, Clone)]
pub struct ItemTransferData {
    pub entry: u32,
    pub count: u32,
    pub durability: u32,
    pub max_durability: u32,
    pub enchantments: Vec<(u32, u32, u32)>,
    pub random_property_id: i32,
    pub creator_guid: Option<crate::shared::protocol::ObjectGuid>,
    pub gift_creator_guid: Option<crate::shared::protocol::ObjectGuid>,
    pub duration: u32,
    pub spell_charges: [i32; 5],
    pub flags: u32,
}

/// Result of a transfer_item operation
#[derive(Debug)]
pub enum TransferItemResult {
    /// Success with new item GUID in target inventory
    Success {
        new_item_guid: crate::shared::protocol::ObjectGuid,
    },
    /// Item not found in source inventory
    ItemNotFound,
    /// Failed to remove item from source
    RemoveFailed,
    /// Target inventory is full
    TargetInventoryFull,
    /// Database error
    DatabaseError(String),
}

/// Result of equip/unequip operations
#[derive(Debug)]
pub enum EquipResult {
    /// Successfully equipped
    Equipped,
    /// Successfully unequipped
    Unequipped,
    /// Successfully swapped (equipped new, unequipped old)
    Swapped {
        unequipped_to_bag: u8,
        unequipped_to_slot: u8,
    },
    /// Cannot equip - level too low
    LevelTooLow,
    /// Cannot equip - wrong class
    WrongClass,
    /// Cannot equip - missing skill/proficiency
    MissingProficiency,
    /// Cannot equip - item doesn't go in that slot
    WrongSlot,
    /// No inventory space to unequip to
    InventoryFull,
    /// Item not found
    ItemNotFound,
    /// Player not found in cache
    PlayerNotLoaded,
    /// Inventory error code
    InventoryError(crate::world::game::inventory::inventory_types::InventoryResult),
    /// Database error
    DatabaseError(String),
}

/// Result of a durability operation
#[derive(Debug)]
pub enum DurabilityResult {
    /// Successfully updated durability
    Success {
        new_durability: u32,
        is_broken: bool,
    },
    /// Item doesn't have durability
    NoDurability,
    /// Item not found
    ItemNotFound,
    /// Player not found in cache
    PlayerNotLoaded,
    /// Database error
    DatabaseError(String),
}

/// Result of a repair operation
#[derive(Debug)]
pub enum RepairResult {
    /// Successfully repaired
    Success {
        /// Total cost paid
        cost: u32,
        /// Number of items repaired
        items_repaired: u32,
    },
    /// Not enough gold to pay for repairs
    InsufficientFunds,
    /// No items need repair
    NothingToRepair,
    /// Item not found
    ItemNotFound,
    /// Player not found in cache
    PlayerNotLoaded,
    /// Database error
    DatabaseError(String),
}

/// Result of an enchantment operation
#[derive(Debug)]
pub enum EnchantResult {
    /// Successfully applied/removed enchantment
    Success,
    /// Invalid enchantment slot
    InvalidSlot,
    /// Enchantment not found in DBC
    EnchantmentNotFound,
    /// Item not found
    ItemNotFound,
    /// Player not found in cache
    PlayerNotLoaded,
    /// Database error
    DatabaseError(String),
}

/// Result of a spell charge operation
#[derive(Debug)]
pub enum ChargeResult {
    /// Successfully consumed charge
    Success {
        /// Remaining charges (-1 for unlimited)
        remaining: i32,
    },
    /// No charges available
    NoCharges,
    /// Invalid charge index
    InvalidIndex,
    /// Item not found
    ItemNotFound,
    /// Player not found in cache
    PlayerNotLoaded,
    /// Database error
    DatabaseError(String),
}

/// Information about a stackable item slot for merging
#[derive(Debug, Clone)]
pub struct StackSlotInfo {
    pub item_guid: ObjectGuid,
    pub bag: u8,
    pub slot: u8,
    pub current_count: u32,
    pub available_space: u32,
}

/// Item position for add operations
#[derive(Debug, Clone, Copy)]
pub struct ItemPosition {
    pub bag: u8,
    pub slot: u8,
}

impl ItemPosition {
    pub fn new(bag: u8, slot: u8) -> Self {
        Self { bag, slot }
    }

    /// Create from main inventory slot (bag 255)
    pub fn main_inventory(slot: u8) -> Self {
        Self {
            bag: crate::world::game::inventory::inventory_types::INVENTORY_SLOT_BAG_0,
            slot,
        }
    }
}

/// Result of a buyback operation
pub enum BuybackResult {
    /// Successfully added to buyback
    Added { slot: u8, item_guid: ObjectGuid },
    /// Successfully retrieved from buyback
    Retrieved { item_guid: ObjectGuid, price: u32 },
    /// Buyback slot not found
    SlotNotFound,
    /// Item not found
    ItemNotFound,
    /// Player not found
    PlayerNotLoaded,
    /// Database error
    DatabaseError(String),
}

/// Result of item modifier operations
#[derive(Debug)]
pub enum ItemModResult {
    /// Successfully applied/removed modifiers
    Success,
    /// Item not equipped
    NotEquipped,
    /// Item not found
    ItemNotFound,
    /// Player not found
    PlayerNotLoaded,
}
