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

use crate::game::mail::{Mail, MailMessageType, MailResponseResult, MailResponseType};
use crate::messages::update::DEFAULT_REALM_ID;
use crate::messages::ToWorldPacket;
use crate::protocol::bitbuf::BitWriter;
use crate::protocol::Opcode;
use crate::protocol::WorldPacket;

/// The attachment id 1.14 uses for a mail's single item.
///
/// 1.12 does not number attachments — a mail holds at most one item and the take-item request
/// identifies it by mail id alone. 1.14 addresses attachments by id, so both the mail list and the
/// take-item result must name the *same* one or the client asks for an attachment the server has
/// never heard of.
const SINGLE_ATTACHMENT_ID: u32 = 1;

/// A 1.12 inventory-result code as 1.14 numbers it.
///
/// The enum was renumbered: 1.14 inserted `CantTradeGold` at 29, so every code from vanilla's
/// `NotEnoughMoney` (29) upward shifted one higher while keeping its name. Passing a vanilla code
/// through unchanged shows the message for the neighbouring error — "you don't have enough money"
/// becomes "not a bag", and so on for the entire upper half of the range.
///
/// This takes the raw u32 the struct stores. The enum-typed translation, which the compiler can
/// check for exhaustiveness, lives with the auction command result in
/// [`crate::messages::auction`]; keep the two in step.
fn modern_equip_error(vanilla: u32) -> u32 {
    /// `SpellFailedReagentsGeneric`, the last code the two protocols agree on.
    const LAST_SHARED_CODE: u32 = 28;
    if vanilla <= LAST_SHARED_CODE {
        vanilla
    } else {
        vanilla + 1
    }
}

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
    fn to_vanilla(&self) -> WorldPacket {
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

    /// `MailCommandResult` in 1.14.
    ///
    /// Vanilla's trailing fields are conditional — the equip error only when the result is
    /// `EquipError`, the attachment id and count only for `ItemTaken`. 1.14 writes all six u32s
    /// unconditionally and lets the client pick which ones matter, so the omitted ones become
    /// explicit zeroes. Sending vanilla's short body leaves the client reading past the end.
    ///
    /// The action and error numbers are the same in both protocols. The equip error is **not** —
    /// see [`modern_equip_error`].
    ///
    /// The attachment id is forced to the same synthetic id the mail list hands out, because 1.12
    /// answers with an item GUID that means nothing to a client addressing attachments by id.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_u32(self.mail_id);
        writer.write_u32(self.response_type as u32); // Command
        writer.write_u32(self.result as u32); // ErrorCode

        // BagResult -- only read when ErrorCode is EquipError.
        let bag_result = if self.result == MailResponseResult::EquipError {
            modern_equip_error(self.equip_error.unwrap_or(0))
        } else {
            0
        };
        writer.write_u32(bag_result);

        let (attach_id, quantity) = if self.response_type == MailResponseType::ItemTaken {
            (SINGLE_ATTACHMENT_ID, self.item_count.unwrap_or(0))
        } else {
            (0, 0)
        };
        writer.write_u32(attach_id);
        writer.write_u32(quantity); // QtyInInventory

        Some(writer.finish(Opcode::SMSG_SEND_MAIL_RESULT))
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
    fn to_vanilla(&self) -> WorldPacket {
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
                    use crate::protocol::guid::{HighGuid, ObjectGuid};
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
                // No item - write zeros with correct types
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

    /// `MailListResult` with `MailListEntry` and `MailAttachedItem` in 1.14.
    ///
    /// Three shape changes, each of which silently corrupts the rest of the body if missed:
    ///
    /// - The count is an i32 and is followed by a second i32 total, where vanilla sends a single
    ///   u8. Vanilla's 254-mail cap is kept so both protocols show the same mailbox.
    /// - Money and COD widen to u64, and the COD moves *ahead* of the money.
    /// - The attachment is no longer an always-present zero-filled block: 1.14 sends a real count
    ///   and only the items that exist, so an empty mail writes a count of zero and nothing else.
    ///   Emitting vanilla's zeroed placeholder item would hang a phantom attachment on every mail.
    ///
    /// Subject and body move to the end behind 8- and 13-bit lengths, and the sender is now either
    /// a 128-bit GUID or a numeric id, selected by a bit rather than by the sender type.
    ///
    /// **The body text is always empty.** 1.12 does not put the letter in this packet — it stores
    /// the text under `item_text_id` and expects the client to fetch it with `CMSG_ITEM_TEXT_QUERY`,
    /// which the 1.14 client never sends because it expects the text inline. Mail opens with its
    /// subject, money and attachment intact and a blank letter; filling it needs the text to reach
    /// this message, which the struct has no field for.
    fn to_modern(&self) -> Option<WorldPacket> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let delivered_mails: Vec<&Mail> = self
            .mails
            .iter()
            .filter(|m| !m.is_expired() && m.is_delivered())
            .take(254)
            .collect();

        let mut writer = BitWriter::new();
        writer.write_i32(delivered_mails.len() as i32);
        writer.write_i32(delivered_mails.len() as i32); // TotalNumRecords -- no paging in 1.12

        for mail in delivered_mails {
            let is_player_sender = mail.message_type == MailMessageType::Normal;
            let attachment = if mail.has_items {
                mail.items.first()
            } else {
                None
            };

            writer.write_i32(mail.id as i32);
            writer.write_u8(mail.message_type as u8); // SenderType
            writer.write_u64(u64::from(mail.cod));
            writer.write_i32(mail.stationery as i32);
            writer.write_u64(u64::from(mail.money)); // SentMoney
            writer.write_u32(u32::from(mail.check_mask.as_u8())); // Flags

            let days_until_expire = if mail.expire_time > now {
                (mail.expire_time - now) as f32 / 86400.0
            } else {
                0.0
            };
            writer.write_f32(days_until_expire);
            writer.write_i32(mail.mail_template_id as i32);
            writer.write_i32(i32::from(attachment.is_some()));

            let subject = mail.subject.as_bytes();
            writer.write_bit(is_player_sender); // has SenderCharacter
            writer.write_bit(!is_player_sender); // has AltSenderID
            writer.write_bits(subject.len() as u32, 8);
            writer.write_bits(0, 13); // Body -- see above
            writer.flush_bits();

            if let Some(item) = attachment {
                writer.write_u8(0); // Position
                writer.write_i32(SINGLE_ATTACHMENT_ID as i32); // AttachID
                writer.write_u32(1); // Count -- 1.12 mails carry a single item
                writer.write_i32(0); // Charges
                writer.write_u32(0); // MaxDurability
                writer.write_u32(0); // Durability
                write_modern_item_instance(&mut writer, item.item_id);
                writer.write_bits(0, 4); // Enchants count
                writer.write_bits(0, 2); // Gems count
                writer.write_bit(true); // Unlocked
                writer.flush_bits();
            }

            if is_player_sender {
                use crate::protocol::guid::{HighGuid, ObjectGuid};
                let sender = ObjectGuid::new_without_entry(HighGuid::Player, mail.sender_guid);
                let (high, low) = sender.to_guid128(DEFAULT_REALM_ID);
                writer.write_packed_guid_128(high, low);
            } else {
                // Creature, gameobject and auction mail identify their sender by entry.
                writer.write_u32(mail.sender_guid);
            }

            writer.write_bytes(subject);
        }

        Some(writer.finish(Opcode::SMSG_MAIL_LIST_RESULT))
    }
}

/// `ItemInstance::Write` for build 42597.
///
/// A bare item id with no bonuses or modifications, which is all a 1.12 item has.
fn write_modern_item_instance(writer: &mut BitWriter, item_id: u32) {
    writer.write_u32(item_id);
    writer.write_u32(0); // RandomPropertiesSeed
    writer.write_u32(0); // RandomPropertiesID
    writer.write_bit(false); // HasItemBonus
    writer.flush_bits();
    writer.write_bits(0, 6); // ItemModList count
    writer.flush_bits();
}

/// SMSG_RECEIVED_MAIL - Notification that mail was received
///
/// Sent to notify player that new mail has arrived.
/// This is an empty packet that just triggers the "new mail" indicator.
#[derive(Debug, Clone)]
pub struct SmsgReceivedMail {}

impl ToWorldPacket for SmsgReceivedMail {
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_RECEIVED_MAIL);
        packet.write_u32(0); // Always 0
        packet
    }

    /// `NotifyReceivedMail` in 1.14. Same four bytes, but they are a **float** delay in seconds
    /// rather than vanilla's unused u32 — zero means the mail is already deliverable, which is what
    /// vanilla's constant zero amounts to. The two happen to encode identically, but only for zero.
    fn to_modern(&self) -> Option<WorldPacket> {
        let mut writer = BitWriter::new();
        writer.write_f32(0.0); // Delay
        Some(writer.finish(Opcode::SMSG_RECEIVED_MAIL))
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
    fn to_vanilla(&self) -> WorldPacket {
        let mut packet = WorldPacket::new(Opcode::SMSG_ITEM_TEXT_QUERY_RESPONSE);
        packet.write_u32(self.text_id);
        packet.write_string(self.text);
        packet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::mail::{Mail, MailCheckMask, MailMessageType, MailState};
    use crate::protocol::Opcode;

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
        let packet = msg.to_vanilla();
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
        let packet = msg.to_vanilla();
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
        let packet = msg.to_vanilla();
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
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_MAIL_LIST_RESULT);
    }

    #[test]
    fn test_smsg_received_mail() {
        let msg = SmsgReceivedMail {};
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_RECEIVED_MAIL);
    }

    #[test]
    fn test_smsg_item_text_query_response() {
        let msg = SmsgItemTextQueryResponse {
            text_id: 123,
            text: "This is some item text content.",
        };
        let packet = msg.to_vanilla();
        assert_eq!(packet.opcode(), Opcode::SMSG_ITEM_TEXT_QUERY_RESPONSE);
    }
}
