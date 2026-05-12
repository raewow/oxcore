//! Vendor data structures
//!
//! Defines the core data types for vendor items, stock tracking, and extended costs.
//! These structures mirror the database tables: npc_vendor, npc_vendor_template.

/// Vendor item entry (from database)
#[derive(Debug, Clone)]
pub struct VendorItem {
    /// Item entry ID
    pub item_entry: u32,
    /// Max stock count (0 = unlimited)
    pub max_count: u8,
    /// Restock interval in seconds
    pub incr_time: u32,
    /// Condition ID for item visibility
    pub condition_id: u32,
    /// Vendor item flags
    pub itemflags: u32,
}

/// Vendor item flags
pub mod vendor_item_flags {
    /// Random restock (80-120% variance)
    pub const RANDOM_RESTOCK: u32 = 0x01;
    /// Dynamic restock (scale with server population)
    pub const DYNAMIC_RESTOCK: u32 = 0x02;
}

/// Runtime stock tracking (per creature GUID)
#[derive(Debug, Clone)]
pub struct VendorItemCount {
    /// Item entry ID
    pub item_entry: u32,
    /// Current stock count
    pub count: u32,
    /// Last restock timestamp (Unix epoch)
    pub last_increment: u64,
    /// Current restock delay (may vary with flags)
    pub restock_delay: u32,
}

/// Extended cost (from DBC file - ItemExtendedCost.dbc)
#[derive(Debug, Clone)]
pub struct ItemExtendedCost {
    /// Extended cost ID
    pub id: u32,
    /// Honor points cost
    pub honor_cost: u32,
    /// Arena points cost
    pub arena_cost: u32,
    /// Required item entries (up to 5)
    pub req_item: [u32; 5],
    /// Required item counts
    pub req_item_count: [u32; 5],
}

/// Reputation rank for discount calculation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ReputationRank {
    /// Hated
    Hated = 0,
    /// Hostile
    Hostile = 1,
    /// Unfriendly
    Unfriendly = 2,
    /// Neutral
    Neutral = 3,
    /// Friendly
    Friendly = 4,
    /// Honored
    Honored = 5,
    /// Revered
    Revered = 6,
    /// Exalted
    Exalted = 7,
}

impl ReputationRank {
    /// Get discount multiplier for this reputation rank
    pub fn discount_multiplier(&self) -> f32 {
        match self {
            ReputationRank::Exalted => 0.80, // 20% discount
            ReputationRank::Revered => 0.90, // 10% discount
            ReputationRank::Honored => 0.95, // 5% discount
            _ => 1.0,                        // No discount
        }
    }
}
