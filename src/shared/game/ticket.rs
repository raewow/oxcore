//! GM Ticket system game types and enums
//!
//! This module contains enums and types for the GM ticket system,
//! matching the vanilla WoW 1.12.x protocol.

/// Response codes sent in SMSG packets
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GmTicketResponse {
    NotExist = 0,
    AlreadyExist = 1,
    CreateSuccess = 2,
    CreateError = 3,
    UpdateSuccess = 4,
    UpdateError = 5,
    TicketDeleted = 9,
}

/// Ticket category/type
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GmTicketType {
    Stuck = 1,
    BehaviorHarassment = 2,
    Guild = 3,
    Item = 4,
    Environmental = 5,
    NonquestCreep = 6,
    QuestQuestnpc = 7,
    Technical = 8,
    AccountBilling = 9,
    Character = 10,
}

/// Ticket status in SMSG_GMTICKET_GETTICKET
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GmTicketStatus {
    DbError = 0x00,
    HasText = 0x06,
    Default = 0x0A,
}

/// GM ticket escalation status
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GmTicketEscalationStatus {
    NotAssigned = 0,
    Assigned = 1,
    Escalated = 2,
}

/// Ticket system availability status
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GmTicketSystemStatus {
    Disabled = 0,
    Enabled = 1,
}
