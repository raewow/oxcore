use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::shared::database::characters::repositories::mail_repository::MailRepository;
use crate::shared::database::characters::repositories::mail_repository_trait::MailRepositoryTrait;
use crate::shared::game::mail::MailMessageType;
use crate::shared::protocol::guid::{HighGuid, ObjectGuid};
use crate::shared::protocol::{Opcode, WorldPacket};
use crate::world::core::common::packet::WorldPacketGuidExt;
use crate::world::core::session::WorldSession;
use crate::world::World;

/// Handle MSG_QUERY_NEXT_MAIL_TIME - client queries if there is pending mail
///
/// Sent on login. Response is MSG_QUERY_NEXT_MAIL_TIME with a single f32:
/// - 0.0 if player has unread, delivered, non-expired mail
/// - -86400.0 if no pending mail
pub async fn handle_query_next_mail_time(
    session: &WorldSession,
    _packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(g) => g,
        None => return Ok(()),
    };

    let player_low = player_guid.low();
    let mail_repo = MailRepository::new(Arc::new(world.databases.character.clone()));

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let has_unread = match mail_repo.find_by_receiver(player_low).await {
        Ok(mails) => mails.iter().any(|m| {
            let is_read = m.checked & 1 != 0;
            let is_delivered = m.deliver_time <= now;
            let is_expired = m.expire_time > 0 && now > m.expire_time;
            !is_read && is_delivered && !is_expired
        }),
        Err(e) => {
            warn!("MSG_QUERY_NEXT_MAIL_TIME: Failed to query mail: {}", e);
            false
        }
    };

    let mut response = WorldPacket::new(Opcode::MSG_QUERY_NEXT_MAIL_TIME);
    if has_unread {
        response.write_f32(0.0);
    } else {
        response.write_f32(-86400.0);
    }
    session.send_packet(response)?;

    debug!(
        "MSG_QUERY_NEXT_MAIL_TIME: has_unread={} for player {}",
        has_unread, player_low
    );
    Ok(())
}

/// Handle CMSG_GET_MAIL_LIST - player opens mailbox
///
/// Packet: packed GUID of mailbox creature/GO (ignored in vanilla)
/// Response: SMSG_MAIL_LIST_RESULT with all delivered, non-expired mail
pub async fn handle_get_mail_list(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let player_guid = match session.player_guid() {
        Some(g) => g,
        None => return Ok(()),
    };

    // Read mailbox GUID (not used for validation in vanilla)
    let _mailbox_guid = packet.read_packed_guid();

    let player_low = player_guid.low();
    let mail_repo = MailRepository::new(Arc::new(world.databases.character.clone()));

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let mails = match mail_repo.find_by_receiver(player_low).await {
        Ok(m) => m,
        Err(e) => {
            warn!("CMSG_GET_MAIL_LIST: Failed to query mail: {}", e);
            // Send empty list
            let mut response = WorldPacket::new(Opcode::SMSG_MAIL_LIST_RESULT);
            response.write_u8(0);
            session.send_packet(response)?;
            return Ok(());
        }
    };

    // Filter to delivered, non-expired mails (max 254)
    let delivered: Vec<_> = mails
        .iter()
        .filter(|m| {
            let is_delivered = m.deliver_time <= now;
            let is_expired = m.expire_time > 0 && now > m.expire_time;
            is_delivered && !is_expired
        })
        .take(254)
        .collect();

    let mut response = WorldPacket::new(Opcode::SMSG_MAIL_LIST_RESULT);
    response.write_u8(delivered.len() as u8);

    for mail in &delivered {
        response.write_u32(mail.id);
        let msg_type = MailMessageType::from(mail.message_type);
        response.write_u8(mail.message_type);

        // Sender info depends on message type
        match msg_type {
            MailMessageType::Normal => {
                let sender_guid = ObjectGuid::new_without_entry(HighGuid::Player, mail.sender_guid);
                response.write_u64(sender_guid.raw());
            }
            _ => {
                response.write_u32(mail.sender_guid);
            }
        }

        // Subject
        response.write_string(mail.subject.as_deref().unwrap_or(""));

        // Item text ID
        response.write_u32(mail.item_text_id);

        // Package ID (always 0 in vanilla)
        response.write_u32(0);

        // Stationery
        response.write_u32(mail.stationery as u32);

        // Item data block (no item support yet)
        // Format: entry(u32), enchant(u32), randomProp(u32), suffix(u32),
        //         count(u8!), charges(u32), maxDur(u32), dur(u32)
        response.write_u32(0); // item_entry
        response.write_u32(0); // enchant_id
        response.write_u32(0); // random_property_id
        response.write_u32(0); // suffix_factor
        response.write_u8(0); // item_count (u8, not u32)
        response.write_u32(0); // spell_charges
        response.write_u32(0); // max_durability
        response.write_u32(0); // durability

        // Money and COD
        response.write_u32(mail.money);
        response.write_u32(mail.cod);

        // Read flag (checked field)
        response.write_u32(mail.checked as u32);

        // Days until expiration
        let days_until_expire = if mail.expire_time > now {
            ((mail.expire_time - now) as f64 / (24.0 * 60.0 * 60.0)) as f32
        } else {
            0.0
        };
        response.write_f32(days_until_expire);

        // Mail template ID
        response.write_u32(mail.mail_template_id);
    }

    debug!(
        "CMSG_GET_MAIL_LIST: Sending {} mails to player {}",
        delivered.len(),
        player_low
    );
    session.send_packet(response)?;

    Ok(())
}

/// Handle CMSG_ITEM_TEXT_QUERY - client requests mail body text
///
/// Sent when a player clicks on a mail to read the body.
/// Packet: u32 item_text_id, u32 mail_id, u32 unk
/// Response: SMSG_ITEM_TEXT_QUERY_RESPONSE with text_id and text content
pub async fn handle_item_text_query(
    session: &WorldSession,
    packet: &mut WorldPacket,
    world: &World,
) -> Result<()> {
    let _player_guid = match session.player_guid() {
        Some(g) => g,
        None => return Ok(()),
    };

    let item_text_id = match packet.read_u32() {
        Some(v) => v,
        None => return Ok(()),
    };
    let _mail_id = packet.read_u32();
    let _unk = packet.read_u32();

    if item_text_id == 0 {
        return Ok(());
    }

    let mail_repo = MailRepository::new(Arc::new(world.databases.character.clone()));
    let text = match mail_repo.find_item_text(item_text_id).await {
        Ok(Some(row)) => row.text.unwrap_or_default(),
        Ok(None) => {
            warn!("CMSG_ITEM_TEXT_QUERY: Item text {} not found", item_text_id);
            String::new()
        }
        Err(e) => {
            warn!(
                "CMSG_ITEM_TEXT_QUERY: DB error for text {}: {}",
                item_text_id, e
            );
            String::new()
        }
    };

    let mut response = WorldPacket::new(Opcode::SMSG_ITEM_TEXT_QUERY_RESPONSE);
    response.write_u32(item_text_id);
    response.write_string(&text);
    session.send_packet(response)?;

    debug!(
        "CMSG_ITEM_TEXT_QUERY: Sent text {} ({} bytes)",
        item_text_id,
        text.len()
    );
    Ok(())
}
