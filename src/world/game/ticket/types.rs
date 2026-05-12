//! Ticket system types
//!
//! In-memory representations of GM tickets.

use crate::shared::game::ticket::{GmTicketEscalationStatus, GmTicketType};
use crate::shared::protocol::ObjectGuid;

/// In-memory ticket entry (cached from database)
#[derive(Debug, Clone)]
pub struct TicketEntry {
    pub ticket_id: u32,
    pub player_guid: ObjectGuid,
    pub player_name: String,
    pub message: String,
    pub ticket_type: GmTicketType,
    pub map_id: u16,
    pub position: (f32, f32, f32),
    pub create_time: u64,
    pub last_modified_time: u64,
    pub escalated_status: GmTicketEscalationStatus,
    pub viewed: bool,
}
