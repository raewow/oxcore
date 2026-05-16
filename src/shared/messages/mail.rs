//! Mail system message structs
//!
//! This module contains type-safe message structures for all mail-related server packets.
//! These messages implement the `ToWorldPacket` trait for serialization.
//!
//! ## Server Messages (SMSG)
//! - [`SmsgSendMailResult`] - Result of sending mail
//! - [`SmsgMailListResult`] - List of mails in player's mailbox
//! - [`SmsgReceivedMail`] - Notification that mail was received
//! - [`SmsgItemTextQueryResponse`] - Response to item text query

use crate::shared::game::mail::{
    Mail, MailCheckMask, MailMessageType, MailResponseResult, MailResponseType, MailStationery,
};
use crate::shared::messages::ToWorldPacket;
use crate::shared::protocol::guid::ObjectGuid;
use crate::shared::protocol::Opcode;
use crate::shared::protocol::WorldPacket;

/// SMSG_SEND_MAIL_RESULT - Result of sending mail
///
/// Sent after attempting to send mail, indicates success or failure.
/// May include additional fields based on result type.
#[derive(Debug, Clone)]
pub struct SmsgSendMailResult {
    /// Unique mail ID
    pub mail_id: u32,
    /// Type of response (Send, MoneyTaken, ItemTaken, etc.)
    pub response_type: MailResponseType,
    /// Result code (Ok, EquipError, NotEnoughMoney, etc.)
    pub result: MailResponseResult,
    /// Equipment error code (only if result is EquipError)
    pub equip_error: Option<u32>,
    /// Item GUID that was taken (only if response_type is ItemTaken)
    pub item_guid: Option<u32>,
    /// Count of items taken (only if response_type is ItemTaken)
    pub item_count: Option<u32>,
}

impl ToWorldPacket for SmsgSendMailResult {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_SEND_MAIL_RESULT);
        packet.write_u32(self.mail_id);
        packet.write_u32(self.response_type as u32);
        packet.write_u32(self.result as u32);

        if self.result == MailResponseResult::EquipError {
            if let Some(error) = self.equip_error {
                packet.write_u32(error);
            }
        }

        if self.response_type == MailResponseType::ItemTaken {
            if let Some(guid) = self.item_guid {
                packet.write_u32(guid);
            }
            if let Some(count) = self.item_count {
                packet.write_u32(count);
            }
        }

        packet
    }
}

/// SMSG_MAIL_LIST_RESULT - List of mails in player's mailbox
///
/// Sent when player opens their mailbox, contains all delivered and unread mails.
/// Filters out expired and undelivered mails automatically.
#[derive(Debug)]
pub struct SmsgMailListResult<'a> {
    /// Reference to array of mails to send
    pub mails: &'a [Mail],
}

impl ToWorldPacket for SmsgMailListResult<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_MAIL_LIST_RESULT);

        // Filter out expired and undelivered mails
        let delivered_mails: Vec<&Mail> = self
            .mails
            .iter()
            .filter(|m| !m.is_expired() && m.is_delivered())
            .take(254) // Max 254 mails (uint8 overflow prevention)
            .collect();

        packet.write_u8(delivered_mails.len() as u8);

        for mail in delivered_mails {
            packet.write_u32(mail.id);
            packet.write_u8(mail.message_type as u8);

            // Sender information based on message type
            match mail.message_type {
                MailMessageType::Normal => {
                    // Send GUID for player mail
                    use crate::shared::protocol::guid::{HighGuid, ObjectGuid};
                    let sender_guid =
                        ObjectGuid::new_without_entry(HighGuid::Player, mail.sender_guid);
                    packet.write_u64(sender_guid.raw());
                }
                _ => {
                    // Send entry/ID for creature/gameobject/auction
                    packet.write_u32(mail.sender_guid);
                }
            }

            packet.write_string(&mail.subject);
            packet.write_u32(mail.item_text_id);
            packet.write_u32(0); // packageId (always 0)
            packet.write_u32(mail.stationery as u32);

            // Item data (if item exists)
            if mail.has_items && !mail.items.is_empty() {
                let item = &mail.items[0]; // Only one item per mail in Classic
                                           // Try to get item from object manager to get full details
                                           // For now, write basic item info
                packet.write_u32(item.item_id);
                packet.write_u32(0); // enchantId
                packet.write_u32(0); // randomPropertyId
                packet.write_u32(0); // suffixFactor
                packet.write_u8(1); // itemCount
                packet.write_u32(0); // charges
                packet.write_u32(0); // maxDurability
                packet.write_u32(0); // durability
            } else {
                // No item - write zeros with correct types matching MaNGOS format
                packet.write_u32(0); // item_entry
                packet.write_u32(0); // enchant_id
                packet.write_u32(0); // random_property_id
                packet.write_u32(0); // suffix_factor
                packet.write_u8(0); // item_count (u8, not u32)
                packet.write_u32(0); // spell_charges
                packet.write_u32(0); // max_durability
                packet.write_u32(0); // durability
            }

            packet.write_u32(mail.money);
            packet.write_u32(mail.cod);
            packet.write_u32(mail.check_mask.as_u8() as u32);

            // Expire time (days until expiration)
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            let days_until_expire = if mail.expire_time > now {
                ((mail.expire_time - now) as f64 / (24.0 * 60.0 * 60.0)) as f32
            } else {
                0.0
            };
            packet.write_f32(days_until_expire);

            // Mail template ID (Client 1.10.0+)
            packet.write_u32(mail.mail_template_id as u32);
        }

        packet
    }
}

/// SMSG_RECEIVED_MAIL - Notification that mail was received
///
/// Sent to notify player that new mail has arrived.
/// This is an empty packet that just triggers the "new mail" indicator.
#[derive(Debug, Clone)]
pub struct SmsgReceivedMail {}

impl ToWorldPacket for SmsgReceivedMail {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_RECEIVED_MAIL);
        packet.write_u32(0); // Always 0
        packet
    }
}

/// SMSG_ITEM_TEXT_QUERY_RESPONSE - Response to item text query
///
/// Sent in response to querying item text (mail body, item descriptions, etc.).
/// Contains the text content associated with an item text ID.
#[derive(Debug, Clone)]
pub struct SmsgItemTextQueryResponse<'a> {
    /// Item text ID being queried
    pub text_id: u32,
    /// Text content
    pub text: &'a str,
}

impl ToWorldPacket for SmsgItemTextQueryResponse<'_> {
    fn to_world_packet(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_ITEM_TEXT_QUERY_RESPONSE);
        packet.write_u32(self.text_id);
        packet.write_string(self.text);
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::game::mail::{Mail, MailCheckMask, MailMessageType, MailState};
    use crate::shared::protocol::Opcode;

    #[test]
    fn test_smsg_send_mail_result() {
        let msg = SmsgSendMailResult {
            mail_id: 123,
            response_type: MailResponseType::Send,
            result: MailResponseResult::Ok,
            equip_error: None,
            item_guid: None,
            item_count: None,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_SEND_MAIL_RESULT);
    }

    #[test]
    fn test_smsg_send_mail_result_with_equip_error() {
        let msg = SmsgSendMailResult {
            mail_id: 123,
            response_type: MailResponseType::Send,
            result: MailResponseResult::EquipError,
            equip_error: Some(1),
            item_guid: None,
            item_count: None,
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_SEND_MAIL_RESULT);
    }

    #[test]
    fn test_smsg_send_mail_result_with_item_taken() {
        let msg = SmsgSendMailResult {
            mail_id: 123,
            response_type: MailResponseType::ItemTaken,
            result: MailResponseResult::Ok,
            equip_error: None,
            item_guid: Some(456),
            item_count: Some(1),
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_SEND_MAIL_RESULT);
    }

    #[test]
    fn test_smsg_mail_list_result() {
        let mails = vec![Mail {
            id: 1,
            message_type: MailMessageType::Normal,
            stationery: 41,
            mail_template_id: 0,
            sender_guid: 123,
            receiver_guid: 456,
            subject: "Test Mail".to_string(),
            item_text_id: 789,
            has_items: false,
            items: vec![],
            expire_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
                + 86400, // 1 day from now
            deliver_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
                - 3600, // 1 hour ago
            money: 100,
            cod: 0,
            checked: 0,
            state: MailState::Unchanged,
            check_mask: MailCheckMask::new(),
        }];

        let msg = SmsgMailListResult { mails: &mails };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_MAIL_LIST_RESULT);
    }

    #[test]
    fn test_smsg_received_mail() {
        let msg = SmsgReceivedMail {};
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_RECEIVED_MAIL);
    }

    #[test]
    fn test_smsg_item_text_query_response() {
        let msg = SmsgItemTextQueryResponse {
            text_id: 123,
            text: "This is some item text content.",
        };
        let packet = msg.to_world_packet();
        assert_eq!(packet.opcode(), Opcode::SMSG_ITEM_TEXT_QUERY_RESPONSE);
    }
}
