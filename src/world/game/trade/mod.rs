//! Trade System for world
//!
//! This module implements player-to-player trading functionality following
//! the world patterns (thin handlers, system-owned state, DashMap caching).
//!
//! ## Architecture
//!
//! - **types.rs**: Constants, enums (TradeState, TradeError), and TradeData struct
//! - **cache.rs**: TradeCache with DashMap<ObjectGuid, Arc<ActiveTrade>>
//! - **system.rs**: TradeSystem implementing the System trait
//! - **tests.rs**: Integration tests
//!
//! ## Trade Flow
//!
//! 1. Player A sends CMSG_INITIATE_TRADE with target GUID
//! 2. Target B receives SMSG_TRADE_STATUS(BeginTrade)
//! 3. Target B sends CMSG_BEGIN_TRADE to accept
//! 4. Both receive SMSG_TRADE_STATUS(OpenWindow) and SMSG_TRADE_STATUS_EXTENDED
//! 5. Players add items/gold, receiving SMSG_TRADE_STATUS_EXTENDED updates
//! 6. Players accept with CMSG_ACCEPT_TRADE (200ms scam prevention delay enforced)
//! 7. When both accept, items/gold transfer, both receive SMSG_TRADE_STATUS(TradeComplete)

pub mod cache;
pub mod system;
pub mod types;

#[cfg(test)]
mod tests;

pub use cache::{ActiveTrade, TradeCache};
pub use system::TradeSystem;
pub use types::{TradeData, TradeError, TradeSlotInfo, TradeState, TradeStatus};
