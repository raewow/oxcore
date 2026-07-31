//! Ticket system message structs
//!
//! This module contains type-safe message structures for all ticket-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgGmTicketSystemStatus`] - Status of the GM ticket system
//! - [`SmsgGmTicketCreate`] - Response to ticket creation
//! - [`SmsgGmTicketGetTicket`] - Response to ticket query
//! - [`SmsgGmTicketUpdateText`] - Response to ticket text update
//! - [`SmsgGmTicketDeleteTicket`] - Response to ticket deletion
//!
//! ## None of these are ported to 1.14
//!
//! 1.14 replaced the GM ticket system outright rather than reshaping it. A 1.14 client does not
//! open a ticket and then poll it; it submits a one-shot support complaint
//! (`CMSG_SUPPORT_TICKET_SUBMIT_COMPLAINT`, carrying the reported player, a complaint category and
//! a note) and afterwards asks only for a case status. The create / query / update / delete
//! round-trip that these five messages answer has no client on the modern side to receive it —
//! the client never sends the requests that produce them, so encoding the responses would be
//! encoding for nobody.
//!
//! Of the vanilla opcodes only the system-status one still exists in the 1.14 table. Its 1.14 body
//! could not be established from a known-good 1.14 encoding, and it is a packet that tells the
//! client whether to *offer* the ticket UI at all — so writing a plausible one and being wrong
//! would open a ticket window that no other message in this module can service. It is left
//! unported for that reason, not because the field is complicated.
//!
//! Porting this system properly means handling the complaint opcode on the way in and answering
//! with a case status on the way out; it is not a re-encoding of these bodies.

use crate::messages::ToWorldPacket;
use crate::protocol::Opcode;
use crate::protocol::WorldPacket;

/// SMSG_GMTICKET_SYSTEMSTATUS - Status of the GM ticket system
///
/// Sent to the player to indicate the current status of the GM ticket system.
#[derive(Debug, Clone)]
pub struct SmsgGmTicketSystemStatus {
    /// System status (0 = unavailable, 1 = available)
    pub status: u32,
}

/// Unported. The opcode survives into 1.14 but its body is unverified, and this packet decides
/// whether the client offers a ticket UI that nothing else here can serve. See the module note.
impl ToWorldPacket for SmsgGmTicketSystemStatus {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GMTICKET_SYSTEMSTATUS);
        packet.write_u32(self.status);
        packet
    }
}

/// SMSG_GMTICKET_CREATE - Response to ticket creation
///
/// Sent to the player in response to creating a new GM ticket.
#[derive(Debug, Clone)]
pub struct SmsgGmTicketCreate {
    /// Ticket creation result (0 = success)
    pub result: u32,
}

/// Unported: no 1.14 opcode. A 1.14 client submits a support complaint and is never told a ticket
/// id, so there is nothing for this reply to answer. See the module note.
impl ToWorldPacket for SmsgGmTicketCreate {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GMTICKET_CREATE);
        packet.write_u32(self.result);
        packet
    }
}

/// Ticket data included in SMSG_GMTICKET_GETTICKET when a ticket exists
#[derive(Debug, Clone)]
pub struct TicketData {
    pub text: String,
    pub ticket_type: u8,
    pub days_since_creation: f32,
    pub days_since_oldest: f32,
    pub days_since_last_update: f32,
    pub escalation_status: u8,
    pub read_by_gm: bool,
}

/// SMSG_GMTICKET_GETTICKET - Response to ticket query
///
/// Sent to the player in response to querying their active GM ticket.
#[derive(Debug, Clone)]
pub struct SmsgGmTicketGetTicket {
    /// Ticket status (GmTicketStatus enum value)
    pub status: u32,
    /// Ticket data if the player has an active ticket
    pub ticket: Option<TicketData>,
}

/// Unported: no 1.14 opcode. 1.14 does not let a player read back an open ticket — there is no
/// stored ticket to read, only a submitted complaint. See the module note.
impl ToWorldPacket for SmsgGmTicketGetTicket {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GMTICKET_GETTICKET);
        packet.write_u32(self.status);

        if let Some(ticket) = &self.ticket {
            packet.write_cstring(&ticket.text);
            packet.write_u8(ticket.ticket_type);
            packet.write_f32(ticket.days_since_creation);
            packet.write_f32(ticket.days_since_oldest);
            packet.write_f32(ticket.days_since_last_update);
            packet.write_u8(ticket.escalation_status);
            packet.write_u8(if ticket.read_by_gm { 1 } else { 0 });
        }

        packet
    }
}

/// SMSG_GMTICKET_UPDATETEXT - Response to ticket text update
///
/// Sent to the player in response to updating their active GM ticket's text.
#[derive(Debug, Clone)]
pub struct SmsgGmTicketUpdateText {
    /// Update result (0 = success)
    pub result: u32,
}

/// Unported: no 1.14 opcode. A 1.14 complaint is submitted once and cannot be edited, so there is
/// no update to acknowledge. See the module note.
impl ToWorldPacket for SmsgGmTicketUpdateText {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GMTICKET_UPDATETEXT);
        packet.write_u32(self.result);
        packet
    }
}

/// SMSG_GMTICKET_DELETETICKET - Response to ticket deletion
///
/// Sent to the player in response to deleting their active GM ticket.
#[derive(Debug, Clone)]
pub struct SmsgGmTicketDeleteTicket {
    /// Deletion result (0 = success)
    pub result: u32,
}

/// Unported: no 1.14 opcode. Nothing on the modern side can request a ticket deletion, so nothing
/// can receive the reply. See the module note.
impl ToWorldPacket for SmsgGmTicketDeleteTicket {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_GMTICKET_DELETETICKET);
        packet.write_u32(self.result);
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Opcode;

    #[test]
    fn test_smsg_gm_ticket_system_status() {
        let msg = SmsgGmTicketSystemStatus { status: 1 };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_GMTICKET_SYSTEMSTATUS);
    }

    #[test]
    fn test_smsg_gm_ticket_create() {
        let msg = SmsgGmTicketCreate { result: 0 };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_GMTICKET_CREATE);
    }

    #[test]
    fn test_smsg_gm_ticket_get_ticket() {
        let msg = SmsgGmTicketGetTicket {
            status: 0x06, // HasText status
            ticket: Some(TicketData {
                text: "Test ticket".to_string(),
                ticket_type: 1, // Stuck
                days_since_creation: 0.5,
                days_since_oldest: 1.0,
                days_since_last_update: 0.25,
                escalation_status: 0, // NotAssigned
                read_by_gm: false,
            }),
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_GMTICKET_GETTICKET);
    }

    #[test]
    fn test_smsg_gm_ticket_get_ticket_no_ticket() {
        let msg = SmsgGmTicketGetTicket {
            status: 0x0A, // Default status
            ticket: None,
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_GMTICKET_GETTICKET);
    }

    #[test]
    fn test_smsg_gm_ticket_update_text() {
        let msg = SmsgGmTicketUpdateText { result: 0 };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_GMTICKET_UPDATETEXT);
    }

    #[test]
    fn test_smsg_gm_ticket_delete_ticket() {
        let msg = SmsgGmTicketDeleteTicket { result: 0 };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_GMTICKET_DELETETICKET);
    }
}
