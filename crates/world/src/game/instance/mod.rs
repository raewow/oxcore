// Instance system - handles dungeon/raid instance management

pub mod manager;

// Re-export all types, constants, and structs
pub use oxcore_shared::game::instance::{
    BossEncounter, InstanceBind, InstanceBinding, InstanceResetFailReason, InstanceResetMethod,
    InstanceResetWarningType, InstanceSave, InstanceState,
};

// Re-export manager
pub use manager::InstanceMgr;
